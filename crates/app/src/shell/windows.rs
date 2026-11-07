//! The registry of the app's windows: one [`WindowState`] per top-level window, keyed by its
//! [`WindowId`], plus the app's open browsers. Everything runs on the CEF UI thread, so the
//! registry lives in thread-local storage. Each part of a window's state has its own cell,
//! borrowed by its module only while it reads or changes it; the registry itself is borrowed
//! only to add, find or remove a window, never across a CEF call or a module's code.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use cef::*;
use mini_core::{popups::PopupThrottle, windows::Windows};

pub use mini_core::windows::WindowId;

#[cfg(target_os = "windows")]
use super::mouse_hook;
use super::{address_box, loading_line, settings_box, tab_strip, tabs, top_bar, window};

/// Everything that belongs to one window.
#[derive(Default)]
pub struct WindowState {
    pub frame: RefCell<Option<window::Frame>>,
    pub shell: RefCell<Option<tabs::Shell>>,
    pub bar: RefCell<Option<top_bar::Bar>>,
    pub strip: RefCell<tab_strip::Strip>,
    pub address_box: RefCell<Option<address_box::AddressBox>>,
    pub settings_box: RefCell<Option<settings_box::SettingsBox>>,
    pub loading_line: RefCell<Option<loading_line::LoadingLine>>,
    /// The height of the loading line's band: the frame's top side.
    pub band_height: Cell<i32>,
    /// Popups opened recently in the window. One shared budget (unit key) rather than one per
    /// opener, so a popup that opens another popup cannot reset the count.
    pub popups: RefCell<PopupThrottle<()>>,
    /// The window this one was opened from by moving a tab into it; its tabs return there.
    pub origin: Cell<Option<WindowId>>,
    /// The tab moving into the window and where the window opens, until the window is created.
    pub incoming: RefCell<Option<window::Incoming>>,
    #[cfg(target_os = "windows")]
    pub mouse: mouse_hook::WindowMouse,
}

#[derive(Default)]
struct Registry {
    windows: Windows<Rc<WindowState>>,
    /// Every browser of the app from its creation until CEF reports it closed, including the
    /// browsers of closing tabs, which no window lists any more.
    browsers: Vec<Browser>,
    /// Whether the app's last browser is closing, i.e. the app is shutting down.
    #[cfg(target_os = "macos")]
    closing: bool,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Registers a new window with empty state; it becomes the last active window.
pub fn add() -> WindowId {
    REGISTRY.with_borrow_mut(|registry| registry.windows.add(Rc::default()))
}

/// Forgets window `win` once it is destroyed.
pub fn remove(win: WindowId) {
    // The state is dropped after the registry borrow ends: releasing its views calls into CEF.
    let state = REGISTRY.with_borrow_mut(|registry| registry.windows.remove(win));
    drop(state);
}

/// The state of window `win`, while it exists. `None` while the registry is borrowed (a
/// re-entrant call, such as the mouse hook's during a registry change).
pub fn get(win: WindowId) -> Option<Rc<WindowState>> {
    REGISTRY.with(|registry| registry.try_borrow().ok()?.windows.get(win).cloned())
}

/// Every window, in the order they opened.
pub fn ids() -> Vec<WindowId> {
    REGISTRY.with_borrow(|registry| registry.windows.ids())
}

/// Records that the user activated window `win`.
pub fn activate(win: WindowId) {
    REGISTRY.with_borrow_mut(|registry| registry.windows.activate(win));
}

/// The window the user activated last (else the newest one).
pub fn last_active() -> Option<WindowId> {
    REGISTRY.with_borrow(|registry| registry.windows.last_active())
}

/// The window a tab of window `win` returns to (see `Windows::return_target`).
pub fn return_target(win: WindowId) -> Option<WindowId> {
    // Closing windows are found before the registry is borrowed: the check reads each window's
    // tabs.
    let closing: Vec<WindowId> = ids()
        .into_iter()
        .filter(|id| !tabs::keeps_running(*id))
        .collect();
    REGISTRY.with_borrow(|registry| {
        let origin = registry.windows.get(win)?.origin.get();
        registry
            .windows
            .return_target_skipping(win, origin, |id| closing.contains(&id))
    })
}

/// The first window whose state matches `predicate`. The predicate runs outside the registry
/// borrow; while the registry is borrowed (a re-entrant call), no window matches.
pub fn find(mut predicate: impl FnMut(&WindowState) -> bool) -> Option<WindowId> {
    let windows: Vec<(WindowId, Rc<WindowState>)> = REGISTRY.with(|registry| {
        let registry = registry.try_borrow().ok()?;
        Some(
            registry
                .windows
                .ids()
                .into_iter()
                .filter_map(|win| Some((win, registry.windows.get(win)?.clone())))
                .collect(),
        )
    })?;
    windows
        .into_iter()
        .find(|(_, state)| predicate(state))
        .map(|(win, _)| win)
}

/// The window whose tabs include `browser`.
pub fn of_browser(browser: &Browser) -> Option<WindowId> {
    find(|state| tabs::holds(state, browser))
}

/// The window of `browser`, else the last active one, for a browser that no window lists (a
/// closing tab's).
pub fn of_browser_or_last(browser: &Browser) -> Option<WindowId> {
    of_browser(browser).or_else(last_active)
}

/// Records a new browser of the app.
pub fn browser_created(browser: &Browser) {
    REGISTRY.with_borrow_mut(|registry| registry.browsers.push(browser.clone()));
}

/// Forgets a closed browser; returns how many browsers remain.
pub fn browser_closed(browser: &Browser) -> usize {
    REGISTRY.with_borrow_mut(|registry| {
        registry
            .browsers
            .retain(|open| open.is_same(Some(&mut browser.clone())) == 0);
        registry.browsers.len()
    })
}

/// How many browsers the app has.
#[cfg(target_os = "macos")]
pub fn browser_count() -> usize {
    REGISTRY.with_borrow(|registry| registry.browsers.len())
}

/// Every browser of the app.
#[cfg(target_os = "macos")]
pub fn browsers() -> Vec<Browser> {
    REGISTRY.with_borrow(|registry| registry.browsers.clone())
}

/// Marks the app as shutting down.
#[cfg(target_os = "macos")]
pub fn set_closing() {
    REGISTRY.with_borrow_mut(|registry| registry.closing = true);
}

/// Whether the app's last browser is closing, i.e. the app is shutting down.
#[cfg(target_os = "macos")]
pub fn is_closing() -> bool {
    REGISTRY.with_borrow(|registry| registry.closing)
}
