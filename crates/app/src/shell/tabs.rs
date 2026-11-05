//! The window's tabs: the [`Tabs`] model plus one browser view per tab, stacked in a container
//! where only the active view is visible. Everything runs on the CEF UI thread, so the state
//! lives in thread-local storage. CEF calls are made after the state borrow ends, because they
//! can call back into this module.

use std::cell::RefCell;

use cef::*;
use mini_core::{
    home::HOME_URL,
    tabs::{TabId, Tabs},
};

use super::{address_box, loading_line, tab_strip, top_bar, window};

/// URL of a popup that has not navigated yet. The model stores it as an empty URL.
const BLANK_URL: &str = "about:blank";

struct Shell {
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

thread_local! {
    static SHELL: RefCell<Option<Shell>> = const { RefCell::new(None) };
}

/// Runs `f` on the shell state; `None` when the window is gone or the state is borrowed.
fn with_shell<R>(f: impl FnOnce(&mut Shell) -> R) -> Option<R> {
    SHELL.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

/// Starts tracking tabs for `window`, whose browser views go into `container`.
pub fn attach(window: Window, container: Panel, client: Client) {
    SHELL.set(Some(Shell {
        window,
        container,
        client,
        tabs: Tabs::new(),
        views: Vec::new(),
        quitting: false,
    }));
}

/// Forgets the window and its views once it is destroyed.
pub fn detach() {
    SHELL.take();
}

/// Opens a tab after the active one. With `url` it loads it; without, the home page, which
/// stays the first entry of the tab's history.
pub fn open_tab(url: Option<&str>) {
    let url = url.unwrap_or(HOME_URL);
    let Some(mut client) = with_shell(|shell| shell.client.clone()) else {
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
        add_tab(view, Some(url));
    }
}

/// Adds an already created browser view (a popup) as a new active tab.
pub fn adopt_popup(view: BrowserView) {
    add_tab(view, None);
}

fn add_tab(view: BrowserView, url: Option<&str>) {
    let Some(container) = with_shell(|shell| {
        let id = shell.tabs.open();
        if let Some(url) = url {
            shell.tabs.set_url(id, url);
        }
        shell.views.push((id, view.clone()));
        shell.container.clone()
    }) else {
        return;
    };
    container.add_child_view(Some(&mut View::from(&view)));
    show_active();
}

/// Makes the active tab's view the only visible one and focuses it, hides the address box,
/// updates the strip and loading line, and flashes the top bar.
fn show_active() {
    let Some((views, active)) = with_shell(|shell| {
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
    tab_strip::refresh();
    top_bar::tabs_changed();
    show_indicators();
    address_box::hide();
    active.request_focus();
}

fn hide_all_but(views: &[BrowserView], shown: &BrowserView) {
    for view in views {
        if view.is_same(Some(&mut View::from(shown))) == 0 {
            view.set_visible(0);
        }
    }
}

/// Moves the keyboard focus to the active page.
pub fn focus_active_page() {
    if let Some(view) = with_shell(|shell| shell.active_view().cloned()).flatten() {
        view.request_focus();
    }
}

pub fn activate(id: TabId) {
    if with_shell(|shell| shell.tabs.activate(id)) == Some(true) {
        show_active();
    }
}

pub fn activate_next() {
    if with_shell(|shell| shell.tabs.activate_next())
        .flatten()
        .is_some()
    {
        show_active();
    }
}

pub fn activate_previous() {
    if with_shell(|shell| shell.tabs.activate_previous())
        .flatten()
        .is_some()
    {
        show_active();
    }
}

/// Asks the active tab's browser to close; [`close_tab_for`] completes the close.
pub fn close_active() {
    with_active_browser(close_browser);
}

/// Asks the browser of tab `id` to close, like [`close_active`].
pub fn close(id: TabId) {
    let browser = with_shell(|shell| shell.view(id).and_then(BrowserView::browser)).flatten();
    if let Some(browser) = browser {
        close_browser(&browser);
    }
}

fn close_browser(browser: &Browser) {
    if let Some(host) = browser.host() {
        host.close_browser(0);
    }
}

/// Called when `browser` is ready to close. Unless the window is closing and this is its last
/// tab, removes its tab and returns `true`: the tab's view is torn down instead of closing the
/// window, and a last tab is replaced by a home tab. While the window closes, the last tab
/// returns `false`, so CEF closes the window (and with it the app).
pub fn close_tab_for(browser: &Browser) -> bool {
    let removed = with_shell(|shell| {
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
    if last {
        open_tab(None);
    } else {
        show_active();
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

/// Every tab's browser, for closing the window.
pub fn browsers() -> Vec<Browser> {
    with_shell(|shell| {
        shell
            .views
            .iter()
            .filter_map(|(_, view)| view.browser())
            .collect()
    })
    .unwrap_or_default()
}

/// Closes the window; each tab's browser closes first (see [`close_tab_for`]).
pub fn quit() {
    if let Some(window) = with_shell(|shell| shell.window.clone()) {
        window.close();
    }
}

/// Marks the window as closing, so its last tab closes the window instead of returning home.
/// Call it before the browsers are asked to close for quitting.
pub fn start_quitting() {
    with_shell(|shell| shell.quitting = true);
}

/// Whether the window is open and not closing, so the app runs on without browsers for a moment
/// while a home tab replaces the closed last tab.
pub fn keeps_running() -> bool {
    with_shell(|shell| !shell.quitting).unwrap_or(false)
}

pub fn with_active_browser(f: impl FnOnce(&Browser)) {
    let browser = with_shell(|shell| shell.active_view().and_then(BrowserView::browser)).flatten();
    if let Some(browser) = browser {
        f(&browser);
    }
}

/// Whether the active tab shows the home page.
/// Runs `f` with the browser of every tab that shows the home page.
pub fn with_home_browsers(mut f: impl FnMut(&Browser)) {
    let browsers: Vec<Browser> = with_shell(|shell| {
        shell
            .views
            .iter()
            .filter(|(id, _)| shell.tabs.get(*id).is_some_and(|tab| tab.is_home()))
            .filter_map(|(_, view)| view.browser())
            .collect()
    })
    .unwrap_or_default();
    for browser in &browsers {
        f(browser);
    }
}

pub fn active_is_home() -> bool {
    with_shell(|shell| shell.tabs.active().is_some_and(|tab| tab.is_home())).unwrap_or(false)
}

/// Whether `browser` is the active tab's and shows the home page.
pub fn is_active_home(browser: &Browser) -> bool {
    with_shell(|shell| {
        let active = shell.tabs.active()?;
        let id = shell.tab_for(browser)?;
        Some(active.id == id && active.is_home())
    })
    .flatten()
    .unwrap_or(false)
}

/// The active tab's URL, empty for a tab that has not navigated yet.
pub fn active_url() -> String {
    with_shell(|shell| shell.tabs.active().map(|tab| tab.url.clone()))
        .flatten()
        .unwrap_or_default()
}

/// Loads `url` in the active tab.
pub fn navigate_active(url: &str) {
    with_shell(|shell| {
        if let Some(id) = shell.tabs.active().map(|tab| tab.id) {
            shell.tabs.set_url(id, url);
        }
    });
    show_indicators();
    with_active_browser(|browser| {
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

/// Applies `f` to the tab of `browser`, shows the tab's new title in the strip and updates the
/// loading line.
fn update(browser: &Browser, f: impl FnOnce(&mut Tabs, TabId)) {
    let updated = with_shell(|shell| {
        let id = shell.tab_for(browser)?;
        f(&mut shell.tabs, id);
        let title = shell.tabs.get(id)?.display_title().to_owned();
        Some((id, title))
    })
    .flatten();
    if let Some((id, title)) = updated {
        tab_strip::retitle(id, &title);
        show_indicators();
    }
}

/// Shows the loading line while the active tab needs it.
fn show_indicators() {
    let loading = with_shell(|shell| shell.tabs.active().is_some_and(|tab| tab.shows_loading()))
        .unwrap_or(false);
    loading_line::set_loading(loading);
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

pub fn strip_entries() -> Vec<StripEntry> {
    with_shell(|shell| {
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
