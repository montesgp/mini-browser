//! Each window's tabs: the [`Tabs`] model plus one browser view per tab, stacked in a container
//! where only the active view is visible. The state lives in the window's registry entry
//! (`windows`). CEF calls are made after the state borrow ends, because they can call back into
//! this module.

use cef::*;
use mini_core::{
    home::HOME_URL,
    tabs::{Tab, TabId, Tabs},
    windows::{can_move_to_new_window, closes_without_tabs},
};

use super::{
    address_box, loading_line, tab_strip, top_bar, window,
    windows::{self, WindowId, WindowState},
};

/// URL of a popup that has not navigated yet. The model stores it as an empty URL.
const BLANK_URL: &str = "about:blank";

pub struct Shell {
    window: Window,
    container: Panel,
    client: Client,
    tabs: Tabs,
    views: Vec<(TabId, BrowserView)>,
    /// Whether the window is closing, so its last tab closes instead of returning home.
    quitting: bool,
}

impl Shell {
    fn active_view(&self) -> Option<&BrowserView> {
        let id = self.tabs.active()?.id;
        self.view(id)
    }

    fn view(&self, id: TabId) -> Option<&BrowserView> {
        self.views
            .iter()
            .find(|(tab, _)| *tab == id)
            .map(|(_, view)| view)
    }

    fn tab_for(&self, browser: &Browser) -> Option<TabId> {
        let identifier = browser.identifier();
        self.views
            .iter()
            .find(|(_, view)| {
                view.browser()
                    .is_some_and(|open| open.identifier() == identifier)
            })
            .map(|(tab, _)| *tab)
    }
}

/// Runs `f` on the tabs of window `win`; `None` when the window is gone or the state is
/// borrowed.
fn with_shell<R>(win: WindowId, f: impl FnOnce(&mut Shell) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let mut shell = state.shell.try_borrow_mut().ok()?;
    shell.as_mut().map(f)
}

/// Whether one of the tabs of the window with `state` shows `browser`.
pub fn holds(state: &WindowState, browser: &Browser) -> bool {
    state.shell.try_borrow().is_ok_and(|shell| {
        shell
            .as_ref()
            .is_some_and(|shell| shell.tab_for(browser).is_some())
    })
}

/// Starts tracking tabs for window `win`, whose browser views go into `container`.
pub fn attach(win: WindowId, window: Window, container: Panel, client: Client) {
    let Some(state) = windows::get(win) else {
        return;
    };
    state.shell.replace(Some(Shell {
        window,
        container,
        client,
        tabs: Tabs::new(),
        views: Vec::new(),
        quitting: false,
    }));
}

/// Opens a home tab at the end of the tabs of window `win`, like `Ctrl/Cmd+T` and the strip's
/// `+`.
pub fn new_tab(win: WindowId) {
    create_tab(win, None, Place::End);
}

/// Opens a tab after the active one, like a link the user opened (`--url` when the app is
/// started again). With `url` it loads it; without, the home page, which stays the first entry
/// of the tab's history.
pub fn open_tab(win: WindowId, url: Option<&str>) {
    create_tab(win, url, Place::AfterActive);
}

/// Where a new tab goes in the tab list.
#[derive(Clone, Copy)]
enum Place {
    AfterActive,
    End,
}

fn create_tab(win: WindowId, url: Option<&str>, place: Place) {
    let url = url.unwrap_or(HOME_URL);
    let Some(mut client) = with_shell(win, |shell| shell.client.clone()) else {
        return;
    };
    let view = browser_view_create(
        Some(&mut client),
        Some(&CefString::from(url)),
        Some(&window::browser_settings()),
        None,
        None,
        Some(&mut window::MiniBrowserViewDelegate::new()),
    );
    if let Some(view) = view {
        let tab = Tab {
            url: url.to_owned(),
            ..Tab::default()
        };
        add_tab(win, view, tab, place);
    }
}

/// Adds an already created browser view (a popup) as a new active tab of window `win` after
/// the active one.
pub fn adopt_popup(win: WindowId, view: BrowserView) {
    add_tab(win, view, Tab::default(), Place::AfterActive);
}

/// Adds the tab `tab`, moved with its live browser view from another window, to window `win`
/// after the active tab, and activates it.
pub fn adopt_moved(win: WindowId, tab: Tab, view: BrowserView) {
    add_tab(win, view, tab, Place::AfterActive);
}

/// Adds `view` as tab `tab` (its title, URL and loading state) and activates it. Returns
/// `false`, adding nothing, when window `win` has no tabs (it is gone).
fn add_tab(win: WindowId, view: BrowserView, tab: Tab, place: Place) -> bool {
    let Some(container) = with_shell(win, |shell| {
        let index = match place {
            Place::AfterActive => shell.tabs.after_active(),
            Place::End => shell.tabs.len(),
        };
        let id = shell.tabs.insert_at(tab, index);
        // The views follow the model's order (the container's order does not matter: it shows
        // one view at a time).
        let index = shell.tabs.index_of(id).unwrap_or(shell.views.len());
        shell
            .views
            .insert(index.min(shell.views.len()), (id, view.clone()));
        shell.container.clone()
    }) else {
        return false;
    };
    container.add_child_view(Some(&mut View::from(&view)));
    show_active(win);
    true
}

/// Moves the active tab of window `win`, with its live page (no reload, its state kept), into a
/// new window next to it. Nothing when it is the window's only tab.
pub fn move_to_new_window(win: WindowId) {
    let Some((window, client)) =
        with_shell(win, |shell| (shell.window.clone(), shell.client.clone()))
    else {
        return;
    };
    let Some((tab, view, _)) = take_active(win, can_move_to_new_window) else {
        return;
    };
    let incoming = window::Incoming {
        tab,
        view,
        bounds: window::next_to(&window),
    };
    // A window that cannot open leaves the tab where it was.
    if let Some(incoming) = window::open_moved(client, win, incoming) {
        add_tab(win, incoming.view, incoming.tab, Place::AfterActive);
    }
}

/// Moves the active tab of window `win`, opened by a move, back to the window it came from (or
/// another open window, see `Windows::return_target`), after that window's active tab. A window
/// left without tabs closes.
pub fn return_tab(win: WindowId) {
    let Some(target) = windows::return_target(win) else {
        return;
    };
    let Some((tab, view, emptied)) = take_active(win, |_| true) else {
        return;
    };
    if !add_tab(target, view.clone(), tab.clone(), Place::AfterActive) {
        add_tab(win, view, tab, Place::AfterActive);
        return;
    }
    if emptied {
        close_emptied(win);
    }
    window::bring_to_front(target);
    focus_active_page_later(target);
}

/// Takes the active tab of window `win` and its browser view out of the window, to move them to
/// another one, when `allowed` says so for the window's tab count. The window shows its next
/// tab; returns whether none is left.
fn take_active(
    win: WindowId,
    allowed: impl FnOnce(usize) -> bool,
) -> Option<(Tab, BrowserView, bool)> {
    let (container, tab, view, emptied) = with_shell(win, |shell| {
        if !allowed(shell.tabs.len()) {
            return None;
        }
        let id = shell.tabs.active()?.id;
        let index = shell.views.iter().position(|(tab, _)| *tab == id)?;
        let tab = shell.tabs.take(id)?;
        let (_, view) = shell.views.remove(index);
        Some((shell.container.clone(), tab, view, shell.tabs.is_empty()))
    })
    .flatten()?;
    container.remove_child_view(Some(&mut View::from(&view)));
    if !emptied {
        show_active(win);
    }
    Some((tab, view, emptied))
}

/// Closes window `win`, left without tabs while other windows stay open.
fn close_emptied(win: WindowId) {
    start_quitting(win);
    // After CEF is done with the current event (a closing browser's `do_close`).
    let mut task = CloseWindow::new(win);
    post_task(ThreadId::UI, Some(&mut task));
}

wrap_task! {
    struct CloseWindow {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            quit(self.win);
        }
    }
}

/// Makes the active tab's view the only visible one and focuses it, hides the address box,
/// updates the strip and loading line, and flashes the top bar.
fn show_active(win: WindowId) {
    let Some((views, active)) = with_shell(win, |shell| {
        let views: Vec<BrowserView> = shell.views.iter().map(|(_, view)| view.clone()).collect();
        (views, shell.active_view().cloned())
    }) else {
        return;
    };
    let Some(active) = active else {
        return;
    };
    // Hide the other views before the focus moves. The focus moves asynchronously, so the
    // previous page still has it here; hiding it while the new page is visible makes Views
    // move the focus to the new page as if Tab was pressed, which focuses (and outlines) its
    // first link. With no other page visible the focus is simply dropped.
    hide_all_but(&views, &active);
    active.set_visible(1);
    tab_strip::refresh(win);
    top_bar::tabs_changed(win);
    show_indicators(win);
    address_box::hide(win);
    active.request_focus();
}

fn hide_all_but(views: &[BrowserView], shown: &BrowserView) {
    for view in views {
        if view.is_same(Some(&mut View::from(shown))) == 0 {
            view.set_visible(0);
        }
    }
}

/// Moves the keyboard focus to the active page of window `win`.
pub fn focus_active_page(win: WindowId) {
    if let Some(view) = with_shell(win, |shell| shell.active_view().cloned()).flatten() {
        view.request_focus();
    }
}

/// Focuses the active page of window `win` once CEF is done with its current work: a window
/// just shown or brought to the front does not take the focus before that.
pub fn focus_active_page_later(win: WindowId) {
    let mut task = FocusPage::new(win);
    post_task(ThreadId::UI, Some(&mut task));
}

wrap_task! {
    struct FocusPage {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            focus_active_page(self.win);
        }
    }
}

pub fn activate(win: WindowId, id: TabId) {
    if with_shell(win, |shell| shell.tabs.activate(id)) == Some(true) {
        show_active(win);
    }
}

/// Activates the tab at 1-based `position` (`Ctrl/Cmd+1` to `8`); nothing when there is none.
pub fn activate_position(win: WindowId, position: usize) {
    if let Some(id) = with_shell(win, |shell| shell.tabs.at_position(position)).flatten() {
        activate(win, id);
    }
}

/// Activates the last tab (`Ctrl/Cmd+9`).
pub fn activate_last(win: WindowId) {
    if let Some(id) = with_shell(win, |shell| shell.tabs.last()).flatten() {
        activate(win, id);
    }
}

pub fn activate_next(win: WindowId) {
    if with_shell(win, |shell| shell.tabs.activate_next())
        .flatten()
        .is_some()
    {
        show_active(win);
    }
}

pub fn activate_previous(win: WindowId) {
    if with_shell(win, |shell| shell.tabs.activate_previous())
        .flatten()
        .is_some()
    {
        show_active(win);
    }
}

/// Moves tab `id` to position `to` among the tabs and shows the new order in the strip.
#[cfg(target_os = "windows")]
pub fn move_tab(win: WindowId, id: TabId, to: usize) {
    let moved = with_shell(win, |shell| {
        if !shell.tabs.move_tab(id, to) {
            return false;
        }
        let order: Vec<TabId> = shell.tabs.iter().map(|tab| tab.id).collect();
        shell
            .views
            .sort_by_key(|(tab, _)| order.iter().position(|id| id == tab));
        true
    });
    if moved == Some(true) {
        tab_strip::refresh(win);
    }
}

/// The position of tab `id` among the tabs.
#[cfg(target_os = "windows")]
pub fn index_of(win: WindowId, id: TabId) -> Option<usize> {
    with_shell(win, |shell| shell.tabs.index_of(id)).flatten()
}

/// Whether `id` is the active tab.
#[cfg(target_os = "windows")]
pub fn is_active(win: WindowId, id: TabId) -> bool {
    with_shell(win, |shell| {
        shell.tabs.active().is_some_and(|tab| tab.id == id)
    })
    .unwrap_or(false)
}

/// Asks the active tab's browser to close; [`close_tab_for`] completes the close.
pub fn close_active(win: WindowId) {
    with_active_browser(win, close_browser);
}

/// Asks the browser of tab `id` to close, like [`close_active`].
pub fn close(win: WindowId, id: TabId) {
    let browser = with_shell(win, |shell| shell.view(id).and_then(BrowserView::browser)).flatten();
    if let Some(browser) = browser {
        close_browser(&browser);
    }
}

fn close_browser(browser: &Browser) {
    if let Some(host) = browser.host() {
        host.close_browser(0);
    }
}

/// Called when `browser` is ready to close. Unless its window is closing and this is its last
/// tab, removes its tab and returns `true`: the tab's view is torn down instead of closing the
/// window, and a last tab is replaced by a home tab. While the window closes, the last tab
/// returns `false`, so CEF closes the window (and, with the last window, the app).
pub fn close_tab_for(browser: &Browser) -> bool {
    let Some(win) = windows::of_browser(browser) else {
        return false;
    };
    let removed = with_shell(win, |shell| {
        let id = shell.tab_for(browser)?;
        let last = shell.tabs.is_only(id);
        if last && shell.quitting {
            return None;
        }
        shell.tabs.close(id);
        let index = shell.views.iter().position(|(tab, _)| *tab == id)?;
        let (_, view) = shell.views.remove(index);
        Some((shell.container.clone(), view, last))
    })
    .flatten();
    let Some((container, view, last)) = removed else {
        return false;
    };
    // Hide the closing page at once (see `show_active` for the order); removing it waits.
    view.set_visible(0);
    // Tear the view down after CEF returns from `do_close`.
    let mut task = RemoveView::new(container, view);
    post_task(ThreadId::UI, Some(&mut task));
    if !last {
        show_active(win);
    } else if closes_without_tabs(other_running_windows(win)) {
        close_emptied(win);
    } else {
        new_tab(win);
    }
    true
}

wrap_task! {
    struct RemoveView {
        container: Panel,
        view: BrowserView,
    }

    impl Task {
        fn execute(&self) {
            self.container
                .remove_child_view(Some(&mut View::from(&self.view)));
        }
    }
}

/// Every tab's browser of window `win`, for closing the window.
pub fn browsers(win: WindowId) -> Vec<Browser> {
    with_shell(win, |shell| {
        shell
            .views
            .iter()
            .filter_map(|(_, view)| view.browser())
            .collect()
    })
    .unwrap_or_default()
}

/// Closes window `win`; each tab's browser closes first (see [`close_tab_for`]).
pub fn quit(win: WindowId) {
    if let Some(window) = with_shell(win, |shell| shell.window.clone()) {
        window.close();
    }
}

/// Closes every window, so the app quits (`Ctrl/Cmd+Q`).
pub fn quit_all() {
    for win in windows::ids() {
        quit(win);
    }
}

/// Marks window `win` as closing, so its last tab closes the window instead of returning home.
/// Call it before the browsers are asked to close for quitting.
pub fn start_quitting(win: WindowId) {
    with_shell(win, |shell| shell.quitting = true);
}

/// Whether window `win` is open and not closing, so the app runs on without browsers for a
/// moment while a home tab replaces the closed last tab.
pub fn keeps_running(win: WindowId) -> bool {
    with_shell(win, |shell| !shell.quitting).unwrap_or(false)
}

/// How many windows are open and not closing.
pub fn running_windows() -> usize {
    windows::ids()
        .into_iter()
        .filter(|win| keeps_running(*win))
        .count()
}

/// How many windows other than `win` are open and not closing.
fn other_running_windows(win: WindowId) -> usize {
    windows::ids()
        .into_iter()
        .filter(|other| *other != win && keeps_running(*other))
        .count()
}

pub fn with_active_browser(win: WindowId, f: impl FnOnce(&Browser)) {
    let browser = with_shell(win, |shell| {
        shell.active_view().and_then(BrowserView::browser)
    })
    .flatten();
    if let Some(browser) = browser {
        f(&browser);
    }
}

/// Runs `f` with the browser of every tab, in every window, that shows the home page.
pub fn with_home_browsers(mut f: impl FnMut(&Browser)) {
    let browsers: Vec<Browser> = windows::ids()
        .into_iter()
        .flat_map(|win| {
            with_shell(win, |shell| {
                shell
                    .views
                    .iter()
                    .filter(|(id, _)| shell.tabs.get(*id).is_some_and(|tab| tab.is_home()))
                    .filter_map(|(_, view)| view.browser())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
        })
        .collect();
    for browser in &browsers {
        f(browser);
    }
}

/// Whether the active tab of window `win` shows the home page.
pub fn active_is_home(win: WindowId) -> bool {
    with_shell(win, |shell| {
        shell.tabs.active().is_some_and(|tab| tab.is_home())
    })
    .unwrap_or(false)
}

/// The window of `browser` when `browser` is the active tab of its own window and shows the
/// home page.
pub fn active_home_window(browser: &Browser) -> Option<WindowId> {
    let win = windows::of_browser(browser)?;
    with_shell(win, |shell| {
        let active = shell.tabs.active()?;
        let id = shell.tab_for(browser)?;
        (active.id == id && active.is_home()).then_some(win)
    })
    .flatten()
}

/// The active tab's URL, empty for a tab that has not navigated yet.
pub fn active_url(win: WindowId) -> String {
    with_shell(win, |shell| shell.tabs.active().map(|tab| tab.url.clone()))
        .flatten()
        .unwrap_or_default()
}

/// Loads `url` in the active tab of window `win`.
pub fn navigate_active(win: WindowId, url: &str) {
    with_shell(win, |shell| {
        if let Some(id) = shell.tabs.active().map(|tab| tab.id) {
            shell.tabs.set_url(id, url);
        }
    });
    show_indicators(win);
    with_active_browser(win, |browser| {
        if let Some(frame) = browser.main_frame() {
            frame.load_url(Some(&CefString::from(url)));
        }
    });
}

pub fn set_title(browser: &Browser, title: &str) {
    update(browser, |tabs, id| tabs.set_title(id, page_text(title)));
}

pub fn set_url(browser: &Browser, url: &str) {
    update(browser, |tabs, id| tabs.set_url(id, page_text(url)));
}

pub fn set_loading(browser: &Browser, loading: bool) {
    update(browser, |tabs, id| tabs.set_loading(id, loading));
}

/// Applies `f` to the tab of `browser`, shows the tab's new title in its window's strip and
/// updates the loading line.
fn update(browser: &Browser, f: impl FnOnce(&mut Tabs, TabId)) {
    let Some(win) = windows::of_browser(browser) else {
        return;
    };
    let updated = with_shell(win, |shell| {
        let id = shell.tab_for(browser)?;
        f(&mut shell.tabs, id);
        let title = shell.tabs.get(id)?.display_title().to_owned();
        Some((id, title))
    })
    .flatten();
    if let Some((id, title)) = updated {
        tab_strip::retitle(win, id, &title);
        show_indicators(win);
    }
}

/// Shows the loading line of window `win` while its active tab needs it.
fn show_indicators(win: WindowId) {
    let loading = with_shell(win, |shell| {
        shell.tabs.active().is_some_and(|tab| tab.shows_loading())
    })
    .unwrap_or(false);
    loading_line::set_loading(win, loading);
}

/// A popup's blank page reports its URL as its title; neither is shown.
fn page_text(text: &str) -> &str {
    if text == BLANK_URL { "" } else { text }
}

/// One tab as the strip shows it.
pub struct StripEntry {
    pub id: TabId,
    pub title: String,
    pub active: bool,
}

pub fn strip_entries(win: WindowId) -> Vec<StripEntry> {
    with_shell(win, |shell| {
        let active = shell.tabs.active().map(|tab| tab.id);
        shell
            .tabs
            .iter()
            .map(|tab| StripEntry {
                id: tab.id,
                title: tab.display_title().to_owned(),
                active: Some(tab.id) == active,
            })
            .collect()
    })
    .unwrap_or_default()
}
