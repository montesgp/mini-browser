//! The top bar: a thin dark row below the frame's top side with the tab strip, a pin button and
//! the minimize, maximize/restore and close buttons (right-aligned, or left-aligned on macOS).
//! Pinned, it always shows. In auto mode it shows while the pointer is over the bar's area (the
//! frame's top side and the bar's height), and briefly when the tabs change; choosing a tab
//! hides it at once. Pinned, it pushes the page down by its height. In auto mode the page moves
//! down by that height without resizing (its bottom is cut off while the bar shows), so the page
//! does not lay out and repaint again each time the bar comes and goes. Hidden, nothing of it
//! shows.
//! A timer per window checks the pointer, because Views reports no pointer moves over the page.
//! The rules live in `mini_core::top_bar`.

use std::time::Instant;

use cef::*;
use mini_core::{
    settings::TopBarMode,
    top_bar::{self, Room, TopBar},
};

use super::{
    settings_box,
    tab_strip::{self, STRIP_COLOR, STRIP_HEIGHT},
    tabs,
    window::{self, DarkPanelDelegate},
    windows::{self, WindowId},
};

const BUTTON_WIDTH: i32 = 40;
const BUTTON_TEXT_COLOR: u32 = 0xFFB4_B4B4;
const BUTTON_HOVER_COLOR: u32 = 0xFF33_3333;
/// A muted red, like the close button of Windows.
const CLOSE_HOVER_COLOR: u32 = 0xFFB0_3A3A;
const PIN_WIDTH: i32 = 32;
/// The pin of a pinned bar, in the accent color of the loading line.
const PINNED_COLOR: u32 = 0xFF5B_8DEF;
const UNPINNED_COLOR: u32 = 0xFF6E_6E6E;
/// On Windows, Segoe UI Symbol's window control glyphs; elsewhere, plain text glyphs. (Views
/// draws the private-use glyphs of Segoe Fluent Icons as missing characters.)
#[cfg(target_os = "windows")]
const BUTTON_FONT: &str = "Segoe UI Symbol, 12px";
#[cfg(not(target_os = "windows"))]
const BUTTON_FONT: &str = "Helvetica, Arial, 13px";
/// How often the pointer is checked.
const POLL_MS: i64 = 100;

// U+FE0E asks for the text (monochrome) form of a symbol, not the color emoji. Segoe UI
// Symbol's minimize glyph (U+1F5D5) sits on the baseline, below the other two; its horizontal
// line extension sits on their center.
#[cfg(target_os = "windows")]
const MINIMIZE_GLYPH: &str = "\u{23AF}";
#[cfg(target_os = "windows")]
const MAXIMIZE_GLYPH: &str = "\u{1F5D6}\u{FE0E}";
#[cfg(target_os = "windows")]
const RESTORE_GLYPH: &str = "\u{1F5D7}\u{FE0E}";
#[cfg(target_os = "windows")]
const CLOSE_GLYPH: &str = "\u{2715}\u{FE0E}";
#[cfg(not(target_os = "windows"))]
const MINIMIZE_GLYPH: &str = "\u{2013}";
#[cfg(not(target_os = "windows"))]
const MAXIMIZE_GLYPH: &str = "\u{25A1}";
#[cfg(not(target_os = "windows"))]
const RESTORE_GLYPH: &str = "\u{2750}";
#[cfg(not(target_os = "windows"))]
const CLOSE_GLYPH: &str = "\u{2715}";
const PIN_GLYPH: &str = "\u{1F4CC}\u{FE0E}";
#[cfg(target_os = "windows")]
const PIN_FONT: &str = "Segoe UI Symbol, 14px";
#[cfg(not(target_os = "windows"))]
const PIN_FONT: &str = "Helvetica, Arial, 14px";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Pin,
    Minimize,
    Maximize,
    Close,
}

pub struct Bar {
    window: Window,
    controller: OverlayController,
    /// Holds the page below the pinned bar.
    spacer: Panel,
    pin: LabelButton,
    maximize: LabelButton,
    mode: TopBarMode,
    state: TopBar,
    /// Whether the first tab opened, so the tabs changing afterwards flashes the bar.
    started: bool,
}

/// Reads the bar of window `win` with `f`; `None` before it is created or once it is gone.
fn with_bar<R>(win: WindowId, f: impl FnOnce(&Bar) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let bar = state.bar.borrow();
    bar.as_ref().map(f)
}

/// Changes the bar of window `win` with `f`.
fn with_bar_mut<R>(win: WindowId, f: impl FnOnce(&mut Bar) -> R) -> Option<R> {
    let state = windows::get(win)?;
    let mut bar = state.bar.borrow_mut();
    bar.as_mut().map(f)
}

/// Creates the bar as an overlay of `window` (window `win`) in `mode` and returns the spacer,
/// the panel that keeps the page below the shown bar; add it to the window's layout below the
/// frame's top side. Call it before the other overlays, so they stay above the bar.
pub fn create(win: WindowId, window: &Window, mode: TopBarMode) -> Option<Panel> {
    let spacer = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    let layout = panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }))?;
    let mut strip = View::from(&tab_strip::create(win)?);
    let pin = window_button(win, Action::Pin)?;
    let buttons = buttons_panel(win)?;
    let maximize = buttons.1;
    let mut buttons = View::from(&buttons.0);
    // The pin follows the strip: before the window buttons on the right, or at the end.
    if cfg!(target_os = "macos") {
        panel.add_child_view(Some(&mut buttons));
        panel.add_child_view(Some(&mut strip));
        panel.add_child_view(Some(&mut View::from(&pin)));
    } else {
        panel.add_child_view(Some(&mut strip));
        panel.add_child_view(Some(&mut View::from(&pin)));
        panel.add_child_view(Some(&mut buttons));
    }
    layout.set_flex_for_view(Some(&mut strip), 1);

    let controller =
        window.add_overlay_view(Some(&mut View::from(&panel)), DockingMode::CUSTOM, 0)?;
    windows::get(win)?.bar.replace(Some(Bar {
        window: window.clone(),
        controller,
        spacer: spacer.clone(),
        pin,
        maximize,
        mode,
        state: TopBar::default(),
        started: false,
    }));
    apply_mode(win);
    poll(win);
    Some(spacer)
}

/// The window buttons in platform order, and the maximize/restore button.
fn buttons_panel(win: WindowId) -> Option<(Panel, LabelButton)> {
    // No fixed height: the panel takes its buttons' size.
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, 0)))?;
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    let order = if cfg!(target_os = "macos") {
        [Action::Close, Action::Minimize, Action::Maximize]
    } else {
        [Action::Minimize, Action::Maximize, Action::Close]
    };
    let mut maximize = None;
    for action in order {
        let button = window_button(win, action)?;
        panel.add_child_view(Some(&mut View::from(&button)));
        if action == Action::Maximize {
            maximize = Some(button);
        }
    }
    Some((panel, maximize?))
}

fn window_button(win: WindowId, action: Action) -> Option<LabelButton> {
    let (glyph, name) = match action {
        Action::Pin => (PIN_GLYPH, "Pin top bar"),
        Action::Minimize => (MINIMIZE_GLYPH, "Minimize"),
        Action::Maximize => (MAXIMIZE_GLYPH, "Maximize"),
        Action::Close => (CLOSE_GLYPH, "Close"),
    };
    let button = label_button_create(
        Some(&mut WindowButtonDelegate::new(win, action)),
        Some(&CefString::from(glyph)),
    )?;
    let font = if action == Action::Pin {
        PIN_FONT
    } else {
        BUTTON_FONT
    };
    button.set_font_list(Some(&CefString::from(font)));
    button.set_enabled_text_colors(BUTTON_TEXT_COLOR);
    button.set_background_color(STRIP_COLOR);
    button.set_ink_drop_enabled(0);
    button.set_focusable(0);
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let width = if action == Action::Pin {
        PIN_WIDTH
    } else {
        BUTTON_WIDTH
    };
    let size = Size {
        width,
        height: STRIP_HEIGHT,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    set_name(&button, name);
    Some(button)
}

fn set_name(button: &LabelButton, name: &str) {
    button.set_tooltip_text(Some(&CefString::from(name)));
    button.set_accessible_name(Some(&CefString::from(name)));
}

/// Switches the bar of window `win` to `mode`: pinned shows the bar; auto hides it unless the
/// pointer is over it.
pub fn set_mode(win: WindowId, mode: TopBarMode) {
    with_bar_mut(win, |bar| bar.mode = mode);
    apply_mode(win);
}

/// Shows the mode on the pin button, and the bar for the mode.
fn apply_mode(win: WindowId) {
    let Some((pin, mode)) = with_bar(win, |bar| (bar.pin.clone(), bar.mode)) else {
        return;
    };
    let (color, name) = match mode {
        TopBarMode::Pinned => (PINNED_COLOR, "Unpin top bar"),
        TopBarMode::Auto => (UNPINNED_COLOR, "Pin top bar"),
    };
    pin.set_enabled_text_colors(color);
    set_name(&pin, name);
    update(win);
}

/// Shows the bar of window `win` for a moment in auto mode; call it when its tabs or its active
/// tab change. The first tab, when the window opens, does not flash it.
pub fn tabs_changed(win: WindowId) {
    let flashed = with_bar_mut(win, |bar| {
        if bar.started {
            bar.state.flash(Instant::now());
        }
        bar.started = true;
    });
    if flashed.is_some() {
        update(win);
    }
}

/// Hides the auto-mode bar of window `win` at once; call it after the user chose a tab in it.
pub fn tab_chosen(win: WindowId) {
    let dismissed = with_bar_mut(win, |bar| bar.state.dismiss(bar.mode));
    if dismissed.is_some() {
        update(win);
    }
}

/// Places the bar below the frame's top side, inset by the frame, and shows the window's
/// state on the maximize button. Call it whenever window `win` is laid out.
pub fn reposition(win: WindowId) {
    let Some((window, controller, maximize)) = with_bar(win, |bar| {
        (
            bar.window.clone(),
            bar.controller.clone(),
            bar.maximize.clone(),
        )
    }) else {
        return;
    };
    let size = window.size();
    let padding = i32::try_from(window::padding(win)).unwrap_or(0);
    controller.set_bounds(Some(&Rect {
        x: padding,
        y: padding,
        width: (size.width - 2 * padding).max(0),
        height: STRIP_HEIGHT,
    }));
    let (glyph, name) = if window.is_maximized() == 1 {
        (RESTORE_GLYPH, "Restore")
    } else {
        (MAXIMIZE_GLYPH, "Maximize")
    };
    if CefString::from(&maximize.text()).to_string() != glyph {
        maximize.set_text(Some(&CefString::from(glyph)));
        set_name(&maximize, name);
    }
}

/// The part of the shown bar right of the tab labels, the `+` button and the +N count, up to
/// the pin, in window coordinates: it drags the window.
pub fn empty_area(win: WindowId) -> Option<Rect> {
    let controller = with_bar(win, |bar| bar.controller.clone())?;
    if controller.is_visible() == 0 {
        return None;
    }
    // The strip's bounds are relative to the bar, which takes the overlay's bounds.
    let bar = controller.bounds();
    let strip = tab_strip::panel(win)?.bounds();
    let labels_end = tab_strip::labels_end(win);
    Some(Rect {
        x: bar.x + strip.x + labels_end,
        y: bar.y,
        width: strip.width - labels_end,
        height: bar.height,
    })
}

/// The shown bar's bounds in window coordinates; `None` while it is hidden.
#[cfg(target_os = "windows")]
pub fn shown_bounds(win: WindowId) -> Option<Rect> {
    let controller = with_bar(win, |bar| bar.controller.clone())?;
    (controller.is_visible() == 1).then(|| controller.bounds())
}

/// `point` (window coordinates) in the tab strip's coordinates, while the bar is shown.
#[cfg(target_os = "windows")]
pub fn strip_point(win: WindowId, point: (i32, i32)) -> Option<(i32, i32)> {
    let bar = shown_bounds(win)?;
    let strip = tab_strip::panel(win)?.bounds();
    Some((point.0 - bar.x - strip.x, point.1 - bar.y - strip.y))
}

/// Whether `screen` (a point on the screen) lies in the bar's area of window `win`, the area
/// that reveals its bar, while the window is not minimized.
#[cfg(target_os = "windows")]
pub fn in_bar_area(win: WindowId, screen: &Point) -> bool {
    let Some(window) = with_bar(win, |bar| bar.window.clone()) else {
        return false;
    };
    if window.is_minimized() == 1 {
        return false;
    }
    let origin = window::origin_in_screen(&window);
    let point = (screen.x - origin.x, screen.y - origin.y);
    zone(win, &window, Some(point)) == top_bar::Pointer::Bar
}

/// Where `point` (window coordinates) is relative to the bar's area of `window` (window `win`).
fn zone(win: WindowId, window: &Window, point: Option<(i32, i32)>) -> top_bar::Pointer {
    let padding = window::padding(win);
    // A maximized window on Windows reaches past the screen's top; the pointer stops there.
    let cut = window::overhang(window);
    top_bar::pointer_zone(
        point,
        window.size().width,
        cut,
        i32::try_from(padding).unwrap_or(0),
        STRIP_HEIGHT,
    )
}

/// Shows or hides the bar of window `win` for the pointer's place and the time.
fn update(win: WindowId) {
    let Some((window, controller, spacer, mode)) = with_bar(win, |bar| {
        (
            bar.window.clone(),
            bar.controller.clone(),
            bar.spacer.clone(),
            bar.mode,
        )
    }) else {
        return;
    };
    let point = (window.is_minimized() == 0)
        .then(|| pointer_in(&window))
        .flatten();
    let pointer = zone(win, &window, point);
    let Some(visible) = with_bar_mut(win, |bar| bar.state.update(mode, pointer, Instant::now()))
    else {
        return;
    };
    let state = i32::from(visible);
    let room = top_bar::room(mode, visible);
    let pushed = i32::from(room == Room::Push);
    if controller.is_visible() != state || spacer.is_visible() != pushed {
        // The bar never covers the page: the spacer pushes it down below the pinned bar, and
        // the auto-mode bar moves it down without resizing it, which would make it repaint.
        spacer.set_visible(pushed);
        window.layout();
        window::shift_pages(win, if room == Room::Shift { STRIP_HEIGHT } else { 0 });
        if visible {
            reposition(win);
            tab_strip::fit(win);
        }
        controller.set_visible(state);
        window::update_drag_regions(win);
    }
}

/// Checks the pointer over window `win` every [`POLL_MS`] while the window exists.
fn poll(win: WindowId) {
    let mut task = Poll::new(win);
    post_delayed_task(ThreadId::UI, Some(&mut task), POLL_MS);
}

wrap_task! {
    struct Poll {
        win: WindowId,
    }

    impl Task {
        fn execute(&self) {
            if with_bar(self.win, |_| ()).is_some() {
                update(self.win);
                poll(self.win);
            }
        }
    }
}

/// The pointer's place in `window` coordinates, or `None` when it is not over the window.
fn pointer_in(window: &Window) -> Option<(i32, i32)> {
    let point = pointer_on_screen(window)?;
    let origin = window::origin_in_screen(window);
    Some((point.x - origin.x, point.y - origin.y))
}

/// The pointer's place on the screen in density-independent pixels, when it is over `window`
/// and no other window covers it there.
#[cfg(target_os = "windows")]
fn pointer_on_screen(window: &Window) -> Option<Point> {
    use windows_sys::Win32::{
        Foundation::POINT,
        UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, GetCursorPos, WindowFromPoint},
    };

    let mut cursor = POINT { x: 0, y: 0 };
    // SAFETY: `cursor` is a valid POINT for the call to fill; the window functions take plain
    // values and return a window handle or null.
    let root = unsafe {
        if GetCursorPos(&mut cursor) == 0 {
            return None;
        }
        GetAncestor(WindowFromPoint(cursor), GA_ROOT)
    };
    if root != window.window_handle().0.cast() {
        return None;
    }
    Some(display_convert_screen_point_from_pixels(Some(&Point {
        x: cursor.x,
        y: cursor.y,
    })))
}

/// macOS screen coordinates start at the bottom left of the primary screen; CEF's start at its
/// top left.
#[cfg(target_os = "macos")]
fn pointer_on_screen(_window: &Window) -> Option<Point> {
    let location = objc2_app_kit::NSEvent::mouseLocation();
    let height = display_get_primary()?.bounds().height;
    Some(Point {
        x: location.x as i32,
        y: height - location.y as i32,
    })
}

/// Linux reports no pointer yet, so in the auto mode the bar shows only when tabs change; the
/// user can pin it.
// TODO(linux): query the pointer (XQueryPointer through the X11 library CEF loads) without
// adding a dependency.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn pointer_on_screen(_window: &Window) -> Option<Point> {
    None
}

fn run(win: WindowId, action: Action) {
    let Some(window) = with_bar(win, |bar| bar.window.clone()) else {
        return;
    };
    match action {
        Action::Pin => settings_box::toggle_top_bar(),
        Action::Minimize => window.minimize(),
        Action::Maximize if window.is_maximized() == 1 => window.restore(),
        Action::Maximize => window.maximize(),
        Action::Close => tabs::quit(win),
    }
}

wrap_button_delegate! {
    struct WindowButtonDelegate {
        win: WindowId,
        action: Action,
    }

    impl ViewDelegate {
        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Theme changes reset the background; reapply the bar color.
            if let Some(view) = view {
                view.set_background_color(STRIP_COLOR);
            }
        }
    }

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            // Closing or minimizing destroys or hides this button: do it after the press.
            let mut task = RunAction::new(self.win, self.action);
            post_task(ThreadId::UI, Some(&mut task));
        }

        fn on_button_state_changed(&self, button: Option<&mut Button>) {
            let Some(button) = button else {
                return;
            };
            let state = button.state();
            let lit = state == ButtonState::HOVERED || state == ButtonState::PRESSED;
            let color = match (lit, self.action) {
                (false, _) => STRIP_COLOR,
                (true, Action::Close) => CLOSE_HOVER_COLOR,
                (true, _) => BUTTON_HOVER_COLOR,
            };
            button.set_background_color(color);
        }
    }
}

wrap_task! {
    struct RunAction {
        win: WindowId,
        action: Action,
    }

    impl Task {
        fn execute(&self) {
            run(self.win, self.action);
        }
    }
}
