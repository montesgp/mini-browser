//! Windows only: the tab gestures Views does not report. A mouse hook on the app's own UI thread
//! (`WH_MOUSE` for this thread, not a low-level or global hook) sees the mouse messages of the
//! app's windows before Views handles them, and each gesture belongs to the window it started
//! in: a middle click on a tab closes it, a press on a tab dragged
//! along the strip moves it among the shown tabs, and a left click on the frame or the top bar
//! closes the settings box. The hook only reads messages: each one goes on to the next hook and
//! to Views unchanged, and the gesture runs in a posted task, after the hook returns.
//!
//! A tab dragged far enough from its bar tears off into a new window, and dragging a window's
//! only tab moves its window. That window then follows the pointer from a short timer until the
//! button is up, rather than the system's move loop, which would hold CEF's tasks until the
//! drop. A tab dragged onto another app window's bar joins that window at once, as in Chrome,
//! and the drag goes on in its strip. A window released with the pointer at a screen's left,
//! right or top edge snaps to that half or maximizes, like Windows' Aero Snap.
//!
//! `Esc` undoes a drag, as in Chrome: the tab goes back to its place, in its own window, and a
//! window moved by its only tab goes back where it was. The same timer reads the key, so `Esc`
//! works while the pointer rests, whichever view has the keyboard focus.

use std::cell::Cell;

use cef::*;
use mini_core::{
    layout::{self, Snap},
    strip::{beyond_drag_threshold, tears_off},
    tabs::TabId,
    windows::{CancelDrag, DragStart, cancel_drag, dragging_moves_window},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetKeyState, VK_ESCAPE, VK_LBUTTON, VK_RBUTTON,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GA_ROOT, GW_HWNDNEXT, GWL_EXSTYLE, GetAncestor, GetCursorPos,
            GetSystemMetrics, GetTopWindow, GetWindow, GetWindowLongW, GetWindowRect, HC_ACTION,
            HHOOK, IsIconic, IsWindowVisible, MOUSEHOOKSTRUCT, SHOW_WINDOW_CMD, SM_SWAPBUTTON,
            SW_MAXIMIZE, SW_RESTORE, SetWindowsHookExW, ShowWindow, UnhookWindowsHookEx, WH_MOUSE,
            WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN,
            WM_MBUTTONUP, WM_MOUSEMOVE, WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP,
            WM_NCMBUTTONDBLCLK, WM_NCMBUTTONDOWN, WM_NCMBUTTONUP, WM_NCMOUSEMOVE,
            WS_EX_TRANSPARENT,
        },
    },
};

use super::{
    settings_box,
    tab_strip::{self, STRIP_HEIGHT},
    tabs, top_bar, window,
    windows::{self, WindowId},
};

/// How often a tab drag checks `Esc` and the button, and a window dragged by its tab follows
/// the pointer.
const FOLLOW_MS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    LeftDown,
    LeftUp,
    Move,
    MiddleDown,
    MiddleUp,
}

/// The gesture in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    None,
    /// The left button went down on `tab` at `start`, `grab` pixels from its label's left edge.
    Pressed {
        tab: TabId,
        start: (i32, i32),
        grab: i32,
    },
    /// `tab` follows the pointer along the strip.
    Dragging {
        tab: TabId,
        grab: i32,
    },
    /// The middle button went down on `tab`.
    Middle {
        tab: TabId,
    },
}

/// A window that follows the pointer: a torn-off tab's new window, or the window whose only tab
/// is dragged.
#[derive(Debug, Clone, Copy)]
struct WindowDrag {
    win: WindowId,
    /// Where the pointer holds the window, from its top left corner on the screen.
    offset: (i32, i32),
    /// How far from its label's left edge the pointer holds the tab.
    grab: i32,
}

/// The tab drag in progress, from the press becoming a drag until the button is up or `Esc`
/// undoes it.
#[derive(Debug, Clone, Copy)]
struct Session {
    start: DragStart,
    /// The window the dragged tab is in now.
    current: WindowId,
    /// Where the window moved by its only tab was (its restored bounds on the screen) and
    /// whether it was maximized.
    home: Option<(layout::Rect, bool)>,
}

/// A window's part of the hook: its native handle and its gesture in progress.
pub struct WindowMouse {
    handle: Cell<HWND>,
    gesture: Cell<Gesture>,
}

impl Default for WindowMouse {
    fn default() -> Self {
        Self {
            handle: Cell::new(std::ptr::null_mut()),
            gesture: Cell::new(Gesture::None),
        }
    }
}

thread_local! {
    /// The one hook of the UI thread, shared by every window.
    static HOOK: Cell<HHOOK> = const { Cell::new(std::ptr::null_mut()) };
    /// The window following the pointer, while the user drags it by its tab.
    static WINDOW_DRAG: Cell<Option<WindowDrag>> = const { Cell::new(None) };
    /// The tab drag in progress.
    static SESSION: Cell<Option<Session>> = const { Cell::new(None) };
    /// Whether [`watch`] runs.
    static WATCHING: Cell<bool> = const { Cell::new(false) };
    /// Whether a tab drag started since the left button last went down in an app window.
    static DRAGGED: Cell<bool> = const { Cell::new(false) };
}

/// Whether the left button's release now ends a tab drag rather than a click: set when a drag
/// starts, until the left button goes down again.
pub fn press_ends_drag() -> bool {
    DRAGGED.get()
}

/// The app window whose native handle is `handle`.
fn window_of(handle: HWND) -> Option<WindowId> {
    (!handle.is_null())
        .then(|| windows::find(|state| state.mouse.handle.get() == handle))
        .flatten()
}

fn handle_of(win: WindowId) -> HWND {
    windows::get(win).map_or(std::ptr::null_mut(), |state| state.mouse.handle.get())
}

fn gesture(win: WindowId) -> Gesture {
    windows::get(win).map_or(Gesture::None, |state| state.mouse.gesture.get())
}

fn set_gesture(win: WindowId, gesture: Gesture) {
    if let Some(state) = windows::get(win) {
        state.mouse.gesture.set(gesture);
    }
}

/// Starts watching the mouse messages of `window` (window `win`). Call it on the UI thread once
/// the window exists, and [`uninstall`] when it is destroyed.
pub fn install(win: WindowId, window: &Window) {
    if let Some(state) = windows::get(win) {
        state.mouse.handle.set(window.window_handle().0.cast());
        state.mouse.gesture.set(Gesture::None);
    }
    if !HOOK.get().is_null() {
        return;
    }
    // SAFETY: `hook` matches HOOKPROC; a thread hook of this process's own thread takes no
    // module handle.
    let hook = unsafe {
        SetWindowsHookExW(
            WH_MOUSE,
            Some(hook),
            std::ptr::null_mut(),
            GetCurrentThreadId(),
        )
    };
    if hook.is_null() {
        eprintln!("minibrowser: cannot watch the mouse; middle click and tab dragging are off");
    }
    HOOK.set(hook);
}

/// Stops watching the mouse of window `win`, and removes the hook with the last window.
pub fn uninstall(win: WindowId) {
    if let Some(state) = windows::get(win) {
        state.mouse.handle.set(std::ptr::null_mut());
        state.mouse.gesture.set(Gesture::None);
    }
    let watched = windows::find(|state| !state.mouse.handle.get().is_null());
    if watched.is_some() {
        return;
    }
    let hook = HOOK.replace(std::ptr::null_mut());
    if !hook.is_null() {
        // SAFETY: `hook` was returned by SetWindowsHookExW on this thread and not removed yet.
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }
}

fn kind_of(message: u32) -> Option<Kind> {
    match message {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK | WM_NCLBUTTONDOWN | WM_NCLBUTTONDBLCLK => {
            Some(Kind::LeftDown)
        }
        WM_LBUTTONUP | WM_NCLBUTTONUP => Some(Kind::LeftUp),
        WM_MOUSEMOVE | WM_NCMOUSEMOVE => Some(Kind::Move),
        WM_MBUTTONDOWN | WM_MBUTTONDBLCLK | WM_NCMBUTTONDOWN | WM_NCMBUTTONDBLCLK => {
            Some(Kind::MiddleDown)
        }
        WM_MBUTTONUP | WM_NCMBUTTONUP => Some(Kind::MiddleUp),
        _ => None,
    }
}

/// Posts the gesture step of each mouse message aimed at an app window (its views or its pages)
/// for that window. Moves matter only while the left button is down on a tab, and they and the
/// release belong to the window the press was in, wherever the pointer is.
unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let kind = u32::try_from(wparam).ok().and_then(kind_of);
    if code == HC_ACTION as i32
        && lparam != 0
        && let Some(kind) = kind
    {
        // SAFETY: for HC_ACTION, `lparam` points to the message's MOUSEHOOKSTRUCT.
        let info = unsafe { &*(lparam as *const MOUSEHOOKSTRUCT) };
        // Here, before Views sees the press, rather than in the posted task: a quick click's
        // release could reach a button before the task runs.
        if kind == Kind::LeftDown {
            DRAGGED.set(false);
        }
        // SAFETY: GetAncestor takes any window handle and returns a handle or null.
        let under = window_of(unsafe { GetAncestor(info.hwnd, GA_ROOT) });
        let tracking = windows::find(|state| {
            matches!(
                state.mouse.gesture.get(),
                Gesture::Pressed { .. } | Gesture::Dragging { .. }
            )
        });
        let win = match kind {
            Kind::Move | Kind::LeftUp => tracking.or(under),
            _ => under,
        };
        if let Some(win) = win
            && (kind != Kind::Move || tracking.is_some())
        {
            let mut task = MouseTask::new(win, kind, info.pt.x, info.pt.y);
            post_task(ThreadId::UI, Some(&mut task));
        }
    }
    // SAFETY: the arguments are the ones this hook received.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

wrap_task! {
    struct MouseTask {
        win: WindowId,
        kind: Kind,
        x: i32,
        y: i32,
    }

    impl Task {
        fn execute(&self) {
            handle(self.win, self.kind, POINT { x: self.x, y: self.y });
        }
    }
}

/// Runs the gesture step of a `kind` message at `pixels` on the screen in window `win`.
fn handle(win: WindowId, kind: Kind, pixels: POINT) {
    let Some(window) = window::window(win) else {
        return;
    };
    let screen = to_screen(pixels);
    let origin = window::origin_in_screen(&window);
    let point = (screen.x - origin.x, screen.y - origin.y);
    let gesture = match (kind, gesture(win)) {
        (Kind::LeftDown, _) => {
            settings_box::close_on_click(win, point);
            match tab_under(win, point) {
                Some((tab, left, x)) => Gesture::Pressed {
                    tab,
                    start: point,
                    grab: x - left,
                },
                None => Gesture::None,
            }
        }
        (Kind::Move, gesture) => drag(win, gesture, point, pixels, &screen),
        (Kind::MiddleDown, _) => match tab_under(win, point) {
            Some((tab, ..)) => Gesture::Middle { tab },
            None => Gesture::None,
        },
        (Kind::MiddleUp, Gesture::Middle { tab }) => {
            // Like other browsers, the tab closes when the button is released over it.
            if tab_under(win, point).is_some_and(|(under, ..)| under == tab) {
                tabs::close(win, tab);
            }
            Gesture::None
        }
        (Kind::LeftUp | Kind::MiddleUp, _) => Gesture::None,
    };
    set_gesture(win, gesture);
}

/// The shown tab under `point` (window coordinates), where its label starts and the point's `x`,
/// both in strip coordinates.
fn tab_under(win: WindowId, point: (i32, i32)) -> Option<(TabId, i32, i32)> {
    let (x, y) = top_bar::strip_point(win, point)?;
    if !(0..STRIP_HEIGHT).contains(&y) {
        return None;
    }
    let (tab, left) = tab_strip::tab_at(win, x)?;
    Some((tab, left, x))
}

/// The next step of `gesture` for the pointer at `point` (window coordinates; `pixels` and
/// `screen` on the screen): a press that moved far enough becomes a drag, which activates its
/// tab, or moves the window when it is the window's only tab. A dragged tab moves to the place
/// under the pointer, joins another window whose bar the pointer reaches (the drag goes on
/// there), and tears off into a new window once the pointer is far enough from the bar. The
/// gesture ends once the left button is up (released outside the window) or the bar hides.
fn drag(
    win: WindowId,
    gesture: Gesture,
    point: (i32, i32),
    pixels: POINT,
    screen: &Point,
) -> Gesture {
    // SAFETY: GetKeyState takes a virtual-key code and reads this thread's input state.
    let left_down = unsafe { GetKeyState(i32::from(VK_LBUTTON)) } < 0;
    let (tab, grab) = match gesture {
        _ if !left_down => return Gesture::None,
        Gesture::Pressed { tab, start, grab } => {
            if !beyond_drag_threshold(point.0 - start.0, point.1 - start.1) {
                return gesture;
            }
            if dragging_moves_window(tabs::count(win)) {
                move_window(win, point, grab);
                return Gesture::None;
            }
            let Some(index) = tabs::index_of(win, tab) else {
                return Gesture::None;
            };
            start_session(win, index, None);
            if !tabs::is_active(win, tab) {
                tabs::activate(win, tab);
            }
            (tab, grab)
        }
        Gesture::Dragging { tab, grab } => (tab, grab),
        other => return other,
    };
    if let Some(target) = drop_target(win, pixels, screen, false) {
        if tabs::is_active(win, tab) {
            join(win, target, grab, screen);
        }
        return Gesture::None;
    }
    let Some(bar) = top_bar::shown_bounds(win) else {
        return Gesture::None;
    };
    let bar = layout::Rect {
        x: bar.x,
        y: bar.y,
        width: bar.width,
        height: bar.height,
    };
    if tears_off(point, bar) {
        if tabs::is_active(win, tab) {
            tear_off(win, grab, screen);
        }
        return Gesture::None;
    }
    let Some((x, _)) = top_bar::strip_point(win, point) else {
        return Gesture::None;
    };
    if let Some(to) = tab_strip::drag_index(win, x, grab)
        && tabs::index_of(win, tab).is_some_and(|from| from != to)
    {
        tabs::move_tab(win, tab, to);
    }
    Gesture::Dragging { tab, grab }
}

/// Moves the active tab of window `win`, dragged `grab` pixels from its label's left edge, into
/// a new window under the pointer at `screen`, which then follows the pointer.
fn tear_off(win: WindowId, grab: i32, screen: &Point) {
    let Some(window) = window::window(win) else {
        return;
    };
    let offset = only_tab_grab(grab);
    // The new window has its source's size, unless the source fills its screen.
    let size = if window.is_maximized() == 1 {
        window::INITIAL_SIZE
    } else {
        let bounds = window.bounds_in_screen();
        Size {
            width: bounds.width,
            height: bounds.height,
        }
    };
    let bounds = Rect {
        x: screen.x - offset.0,
        y: screen.y - offset.1,
        width: size.width,
        height: size.height,
    };
    if let Some(moved) = tabs::tear_off(win, bounds) {
        set_current(moved);
        follow(WindowDrag {
            win: moved,
            offset,
            grab,
        });
    }
}

/// Where the pointer holding a window's only tab `grab` pixels from its label's left edge sits,
/// from the window's corner, with the saved frame.
fn only_tab_grab(grab: i32) -> (i32, i32) {
    let padding = settings_box::settings().map_or(0, |settings| settings.padding);
    tab_strip::only_tab_grab(grab, i32::try_from(padding).unwrap_or(0))
}

/// Makes window `win`, whose only tab is dragged from `point` (window coordinates) `grab`
/// pixels from its label's left edge, follow the pointer. A maximized window is restored first,
/// with its tab under the pointer.
fn move_window(win: WindowId, point: (i32, i32), grab: i32) {
    let Some(window) = window::window(win) else {
        return;
    };
    let maximized = window.is_maximized() == 1;
    let offset = if maximized {
        restore(win);
        only_tab_grab(grab)
    } else {
        // `point` starts where the window's views start; its position is its bounds' corner.
        let origin = window::origin_in_screen(&window);
        let bounds = window.bounds_in_screen();
        (point.0 + origin.x - bounds.x, point.1 + origin.y - bounds.y)
    };
    // A maximized window's restored bounds, read once it is restored.
    let home = window::core_rect(&window.bounds_in_screen());
    start_session(win, 0, Some((home, maximized)));
    follow(WindowDrag { win, offset, grab });
}

/// Restores maximized window `win`. CEF's `Window::restore` leaves a frameless maximized window
/// maximized while a mouse button is down, so the window is restored directly.
fn restore(win: WindowId) {
    show_window(win, SW_RESTORE);
}

/// Maximizes window `win` directly, for the same reason as [`restore`].
fn maximize(win: WindowId) {
    show_window(win, SW_MAXIMIZE);
}

fn show_window(win: WindowId, command: SHOW_WINDOW_CMD) {
    let handle = handle_of(win);
    if !handle.is_null() {
        // SAFETY: ShowWindow takes any window handle; `handle` is one of the app's windows.
        unsafe { ShowWindow(handle, command) };
    }
}

/// Starts the drag of the tab at `index` among the tabs of window `win`, which moves the window
/// when `home` (where the window was) is given, and [`watch`]es it.
fn start_session(win: WindowId, index: usize, home: Option<(layout::Rect, bool)>) {
    DRAGGED.set(true);
    SESSION.set(Some(Session {
        start: DragStart {
            window: win,
            index,
            moves_window: home.is_some(),
        },
        current: win,
        home,
    }));
    if !WATCHING.replace(true) {
        watch_later();
    }
}

/// Notes that the dragged tab moved into window `win`.
fn set_current(win: WindowId) {
    if let Some(session) = SESSION.get() {
        SESSION.set(Some(Session {
            current: win,
            ..session
        }));
    }
}

/// Ends the tab drag in progress.
fn end_session() {
    WINDOW_DRAG.set(None);
    SESSION.set(None);
}

/// Starts `drag`: its window follows the pointer until the button is up.
fn follow(drag: WindowDrag) {
    WINDOW_DRAG.set(Some(drag));
}

/// Runs every [`FOLLOW_MS`] while a tab drag goes on: `Esc` undoes it, a window dragged by its
/// tab follows the pointer, and the drag ends once the button is up.
fn watch() {
    if let Some(session) = SESSION.get() {
        if escape_down() {
            cancel(session);
        } else if WINDOW_DRAG.get().is_some() {
            follow_pointer();
        } else if !physical_left_down() {
            end_session();
        }
    }
    if SESSION.get().is_some() {
        watch_later();
    } else {
        WATCHING.set(false);
    }
}

fn watch_later() {
    let mut task = Watch::new();
    if post_delayed_task(ThreadId::UI, Some(&mut task), FOLLOW_MS) == 0 {
        // The task did not post (the app is shutting down): the next drag starts a new watch.
        WATCHING.set(false);
    }
}

wrap_task! {
    struct Watch {}

    impl Task {
        fn execute(&self) {
            watch();
        }
    }
}

/// Undoes the tab drag `session`, as in Chrome: a tab dragged along its strip goes back to its
/// place, a tab dragged out of its window goes back into it at its place (a window left without
/// tabs closes), and a window moved by its only tab goes back where it was.
fn cancel(session: Session) {
    let dragged = match gesture(session.current) {
        Gesture::Dragging { tab, .. } => Some(tab),
        _ => None,
    };
    end_session();
    set_gesture(session.current, Gesture::None);
    let source_open = tabs::keeps_running(session.start.window);
    match cancel_drag(session.start, session.current, source_open) {
        CancelDrag::MoveBack { window } => {
            if let Some(home) = session.home {
                put_back(window, home);
            }
        }
        CancelDrag::Reorder { window, index } => {
            if let Some(tab) = dragged.or_else(|| tabs::active(window)) {
                tabs::move_tab(window, tab, index);
            }
        }
        CancelDrag::Return {
            from,
            window,
            index,
        } => {
            tabs::drop_active(from, window, Some(index));
        }
        CancelDrag::Nothing => {}
    }
}

/// Puts window `win` back at `bounds` on the screen, maximized again when it was.
fn put_back(win: WindowId, (bounds, maximized): (layout::Rect, bool)) {
    if let Some(window) = window::window(win) {
        window.set_bounds(Some(&window::cef_rect(bounds)));
        if maximized {
            maximize(win);
        }
    }
}

/// Moves the dragged window under the pointer until the button is up, where it snaps at a
/// screen's edge, or its tab reaches another window's bar and joins it.
fn follow_pointer() {
    let Some(drag) = WINDOW_DRAG.get() else {
        return;
    };
    let (Some(window), Some(pixels)) = (window::window(drag.win), cursor()) else {
        end_session();
        return;
    };
    let screen = to_screen(pixels);
    if let Some(target) = drop_target(drag.win, pixels, &screen, true) {
        WINDOW_DRAG.set(None);
        if physical_left_down() {
            join(drag.win, target, drag.grab, &screen);
        } else {
            end_session();
            tabs::drop_active(drag.win, target, drop_index(target, &screen, drag.grab));
        }
        return;
    }
    if !physical_left_down() {
        end_session();
        snap(&window, drag.win, &screen);
        return;
    }
    let position = Point {
        x: screen.x - drag.offset.0,
        y: screen.y - drag.offset.1,
    };
    if window.is_maximized() == 1 {
        // Moving a maximized window only moves its saved restored bounds.
        restore(drag.win);
    } else {
        let bounds = window.bounds_in_screen();
        if (bounds.x, bounds.y) != (position.x, position.y) {
            window.set_position(Some(&position));
        }
    }
}

/// Snaps `window` (window `win`), released with the pointer at `screen`, at the edge of the
/// screen under the pointer: to the half of its work area at the left or right edge, maximized
/// at the top edge (see `layout::snap_target`).
fn snap(window: &Window, win: WindowId, screen: &Point) {
    let Some(display) = display_get_nearest_point(Some(screen), 0) else {
        return;
    };
    let bounds = window::core_rect(&display.bounds());
    let work_area = window::core_rect(&display.work_area());
    let mut displays = vec![None; display_get_count()];
    display_get_alls(Some(&mut displays));
    let other: Vec<layout::Rect> = displays
        .iter()
        .flatten()
        .map(|display| window::core_rect(&display.bounds()))
        .filter(|other| *other != bounds)
        .collect();
    match layout::snap_target_among((screen.x, screen.y), bounds, &other) {
        Some(Snap::Maximize) => {
            // Windows maximizes a window on the screen it overlaps most, so it first moves onto
            // the screen under the pointer.
            let size = window.bounds_in_screen();
            window.set_bounds(Some(&window::cef_rect(layout::Rect {
                x: work_area.x,
                y: work_area.y,
                width: size.width.min(work_area.width),
                height: size.height.min(work_area.height),
            })));
            maximize(win);
        }
        Some(half) => {
            if let Some(half) = layout::snap_bounds(half, work_area) {
                window.set_bounds(Some(&window::cef_rect(half)));
            }
        }
        None => {}
    }
}

/// Moves the active tab of window `win`, dragged `grab` pixels from its label's left edge, into
/// window `target` at the place under the pointer at `screen`, where the drag goes on.
fn join(win: WindowId, target: WindowId, grab: i32, screen: &Point) {
    if tabs::drop_active(win, target, drop_index(target, screen, grab))
        && let Some(tab) = tabs::active(target)
    {
        set_current(target);
        set_gesture(target, Gesture::Dragging { tab, grab });
    }
}

/// The other app window whose bar's area lies under the pointer (at `pixels` and `screen`) while
/// a tab of window `dragged` is dragged: the topmost window there, looking through `dragged`
/// when it is the window moving with the pointer (`moving`). A window of another app on top
/// hides the windows below it.
fn drop_target(dragged: WindowId, pixels: POINT, screen: &Point, moving: bool) -> Option<WindowId> {
    let skip = if moving {
        handle_of(dragged)
    } else {
        std::ptr::null_mut()
    };
    let target = window_of(top_level_at(pixels, skip))?;
    (target != dragged && tabs::keeps_running(target) && top_bar::in_bar_area(target, screen))
        .then_some(target)
}

/// Where a tab dragged `grab` pixels from its label's left edge goes among the tabs of window
/// `target` for the pointer at `screen`; `None` (the end) when its strip is not shown.
fn drop_index(target: WindowId, screen: &Point, grab: i32) -> Option<usize> {
    let window = window::window(target)?;
    let origin = window::origin_in_screen(&window);
    let (x, _) = top_bar::strip_point(target, (screen.x - origin.x, screen.y - origin.y))?;
    tab_strip::drop_index(target, x, grab)
}

/// The topmost top-level window under `pixels` on the screen, other than `skip`, by walking the
/// windows in z-order. Not `WindowFromPoint`: called from this thread, it asks the app's own
/// windows to hit-test (`WM_NCHITTEST`), and Views lets it pass through some of them.
fn top_level_at(pixels: POINT, skip: HWND) -> HWND {
    // SAFETY: the window functions take plain values and return a window handle or null;
    // GW_HWNDNEXT walks the top-level windows from the top of the z-order down.
    let mut window = unsafe { GetTopWindow(std::ptr::null_mut()) };
    while !window.is_null() && (window == skip || !hit_by(window, pixels)) {
        // SAFETY: as above.
        window = unsafe { GetWindow(window, GW_HWNDNEXT) };
    }
    window
}

/// Whether the pointer at `pixels` would hit `window`: shown, not minimized, cloaked or
/// transparent to the mouse, and covering the point.
fn hit_by(window: HWND, pixels: POINT) -> bool {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: the window functions take any window handle, and `rect` is a valid RECT for
    // GetWindowRect to fill.
    let shown = unsafe {
        IsWindowVisible(window) != 0
            && IsIconic(window) == 0
            && (GetWindowLongW(window, GWL_EXSTYLE).cast_unsigned() & WS_EX_TRANSPARENT) == 0
            && GetWindowRect(window, &mut rect) != 0
    };
    shown
        && !is_cloaked(window)
        && (rect.left..rect.right).contains(&pixels.x)
        && (rect.top..rect.bottom).contains(&pixels.y)
}

#[link(name = "dwmapi", kind = "raw-dylib")]
unsafe extern "system" {
    fn DwmGetWindowAttribute(
        window: HWND,
        attribute: u32,
        value: *mut std::ffi::c_void,
        size: u32,
    ) -> i32;
}

/// `DWMWA_CLOAKED`: whether the system hides a window that counts as visible (an app on another
/// virtual desktop, a suspended store app).
const DWMWA_CLOAKED: u32 = 14;

fn is_cloaked(window: HWND) -> bool {
    let mut cloaked: u32 = 0;
    // SAFETY: `cloaked` is the DWORD that DWMWA_CLOAKED fills, and its size is passed.
    let result = unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED,
            (&raw mut cloaked).cast(),
            u32::try_from(std::mem::size_of::<u32>()).unwrap_or(4),
        )
    };
    result == 0 && cloaked != 0
}

/// The pointer's place on the screen, in pixels.
fn cursor() -> Option<POINT> {
    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: `point` is a valid POINT for the call to fill.
    (unsafe { GetCursorPos(&mut point) } != 0).then_some(point)
}

fn to_screen(pixels: POINT) -> Point {
    display_convert_screen_point_from_pixels(Some(&Point {
        x: pixels.x,
        y: pixels.y,
    }))
}

/// Whether `Esc` is down now, whichever window has the keyboard focus.
fn escape_down() -> bool {
    // SAFETY: GetAsyncKeyState takes a virtual-key code and reads the system's state.
    unsafe { GetAsyncKeyState(i32::from(VK_ESCAPE)) < 0 }
}

/// Whether the primary mouse button is down now, whichever window gets the mouse messages:
/// `GetAsyncKeyState` reads the physical buttons, so with swapped buttons that is the right one.
fn physical_left_down() -> bool {
    // SAFETY: both take plain values and read the system's state.
    unsafe {
        let key = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
            VK_RBUTTON
        } else {
            VK_LBUTTON
        };
        GetAsyncKeyState(i32::from(key)) < 0
    }
}
