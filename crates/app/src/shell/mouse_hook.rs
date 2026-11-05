//! Windows only: the tab gestures Views does not report. A mouse hook on the app's own UI thread
//! (`WH_MOUSE` for this thread, not a low-level or global hook) sees the mouse messages of the
//! window before Views handles them: a middle click on a tab closes it, a press on a tab dragged
//! along the strip moves it among the shown tabs, and a left click on the frame or the top bar
//! closes the settings box. The hook only reads messages: each one goes on to the next hook and
//! to Views unchanged, and the gesture runs in a posted task, after the hook returns.

use std::cell::{Cell, RefCell};

use cef::*;
use mini_core::{strip::beyond_drag_threshold, tabs::TabId};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::{
        Input::KeyboardAndMouse::{GetKeyState, VK_LBUTTON},
        WindowsAndMessaging::{
            CallNextHookEx, GA_ROOT, GetAncestor, HC_ACTION, HHOOK, MOUSEHOOKSTRUCT,
            SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN,
            WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE,
            WM_NCLBUTTONDBLCLK, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMBUTTONDBLCLK,
            WM_NCMBUTTONDOWN, WM_NCMBUTTONUP, WM_NCMOUSEMOVE,
        },
    },
};

use super::{
    settings_box,
    tab_strip::{self, STRIP_HEIGHT},
    tabs, top_bar, window,
};

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
enum Gesture {
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

thread_local! {
    static HOOK: Cell<HHOOK> = const { Cell::new(std::ptr::null_mut()) };
    static WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
    static HANDLE: Cell<HWND> = const { Cell::new(std::ptr::null_mut()) };
    static GESTURE: Cell<Gesture> = const { Cell::new(Gesture::None) };
}

/// Starts watching the mouse messages of `window`. Call it on the UI thread once the window
/// exists, and [`uninstall`] when it is destroyed.
pub fn install(window: &Window) {
    uninstall();
    HANDLE.set(window.window_handle().0.cast());
    WINDOW.set(Some(window.clone()));
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

/// Stops watching the mouse and forgets the window.
pub fn uninstall() {
    let hook = HOOK.replace(std::ptr::null_mut());
    if !hook.is_null() {
        // SAFETY: `hook` was returned by SetWindowsHookExW on this thread and not removed yet.
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }
    HANDLE.set(std::ptr::null_mut());
    WINDOW.take();
    GESTURE.set(Gesture::None);
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

/// Posts the gesture step of each mouse message aimed at the app window (its views or its pages).
/// Moves matter only while the left button is down on a tab.
unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let kind = u32::try_from(wparam).ok().and_then(kind_of);
    if code == HC_ACTION as i32
        && lparam != 0
        && let Some(kind) = kind
    {
        // SAFETY: for HC_ACTION, `lparam` points to the message's MOUSEHOOKSTRUCT.
        let info = unsafe { &*(lparam as *const MOUSEHOOKSTRUCT) };
        // SAFETY: GetAncestor takes any window handle and returns a handle or null.
        let root = unsafe { GetAncestor(info.hwnd, GA_ROOT) };
        let tracking = matches!(
            GESTURE.get(),
            Gesture::Pressed { .. } | Gesture::Dragging { .. }
        );
        if !root.is_null() && root == HANDLE.get() && (kind != Kind::Move || tracking) {
            let mut task = MouseTask::new(kind, info.pt.x, info.pt.y);
            post_task(ThreadId::UI, Some(&mut task));
        }
    }
    // SAFETY: the arguments are the ones this hook received.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

wrap_task! {
    struct MouseTask {
        kind: Kind,
        x: i32,
        y: i32,
    }

    impl Task {
        fn execute(&self) {
            handle(self.kind, Point { x: self.x, y: self.y });
        }
    }
}

/// Runs the gesture step of a `kind` message at `pixels` on the screen.
fn handle(kind: Kind, pixels: Point) {
    let Some(window) = WINDOW.with_borrow(Clone::clone) else {
        return;
    };
    let screen = display_convert_screen_point_from_pixels(Some(&pixels));
    let origin = window::origin_in_screen(&window);
    let point = (screen.x - origin.x, screen.y - origin.y);
    let gesture = match (kind, GESTURE.get()) {
        (Kind::LeftDown, _) => {
            settings_box::close_on_click(point);
            match tab_under(point) {
                Some((tab, left, x)) => Gesture::Pressed {
                    tab,
                    start: point,
                    grab: x - left,
                },
                None => Gesture::None,
            }
        }
        (Kind::Move, gesture) => drag(gesture, point),
        (Kind::MiddleDown, _) => match tab_under(point) {
            Some((tab, ..)) => Gesture::Middle { tab },
            None => Gesture::None,
        },
        (Kind::MiddleUp, Gesture::Middle { tab }) => {
            // Like other browsers, the tab closes when the button is released over it.
            if tab_under(point).is_some_and(|(under, ..)| under == tab) {
                tabs::close(tab);
            }
            Gesture::None
        }
        (Kind::LeftUp | Kind::MiddleUp, _) => Gesture::None,
    };
    GESTURE.set(gesture);
}

/// The shown tab under `point` (window coordinates), where its label starts and the point's `x`,
/// both in strip coordinates.
fn tab_under(point: (i32, i32)) -> Option<(TabId, i32, i32)> {
    let (x, y) = top_bar::strip_point(point)?;
    if !(0..STRIP_HEIGHT).contains(&y) {
        return None;
    }
    let (tab, left) = tab_strip::tab_at(x)?;
    Some((tab, left, x))
}

/// The next step of `gesture` for the pointer at `point`: a press that moved far enough becomes
/// a drag, which activates its tab, and a dragged tab moves to the place under the pointer. The
/// gesture ends once the left button is up (released outside the window) or the bar hides.
fn drag(gesture: Gesture, point: (i32, i32)) -> Gesture {
    // SAFETY: GetKeyState takes a virtual-key code and reads this thread's input state.
    let left_down = unsafe { GetKeyState(i32::from(VK_LBUTTON)) } < 0;
    let (tab, grab) = match gesture {
        _ if !left_down => return Gesture::None,
        Gesture::Pressed { tab, start, grab } => {
            if !beyond_drag_threshold(point.0 - start.0, point.1 - start.1) {
                return gesture;
            }
            if !tabs::is_active(tab) {
                tabs::activate(tab);
            }
            (tab, grab)
        }
        Gesture::Dragging { tab, grab } => (tab, grab),
        other => return other,
    };
    let Some((x, _)) = top_bar::strip_point(point) else {
        return Gesture::None;
    };
    if let Some(to) = tab_strip::drag_index(x, grab)
        && tabs::index_of(tab).is_some_and(|from| from != to)
    {
        tabs::move_tab(tab, to);
    }
    Gesture::Dragging { tab, grab }
}
