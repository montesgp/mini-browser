//! The top bar: a thin dark row below the frame's top side with the tab strip and the
//! minimize, maximize/restore and close buttons (right-aligned, or left-aligned on macOS).
//! Pinned, it always shows and the page sits below it. In auto mode it is an overlay that shows
//! while the pointer is at the top edge of the window or over the bar, and briefly when the
//! tabs change; hidden, nothing of it shows. A timer checks the pointer, because Views reports
//! no pointer moves over the page. The rules live in `mini_core::top_bar`.

use std::{cell::RefCell, time::Instant};

use cef::*;
use mini_core::{
    settings::TopBarMode,
    top_bar::{self, TopBar},
};

use super::{
    tab_strip::{self, STRIP_COLOR, STRIP_HEIGHT},
    tabs,
    window::{self, DarkPanelDelegate},
};

const BUTTON_WIDTH: i32 = 40;
const BUTTON_TEXT_COLOR: u32 = 0xFFB4_B4B4;
const BUTTON_HOVER_COLOR: u32 = 0xFF33_3333;
/// A muted red, like the close button of Windows.
const CLOSE_HOVER_COLOR: u32 = 0xFFB0_3A3A;
/// On Windows, Segoe UI Symbol's window control glyphs; elsewhere, plain text glyphs. (Views
/// draws the private-use glyphs of Segoe Fluent Icons as missing characters.)
#[cfg(target_os = "windows")]
const BUTTON_FONT: &str = "Segoe UI Symbol, 12px";
#[cfg(not(target_os = "windows"))]
const BUTTON_FONT: &str = "Helvetica, Arial, 13px";
/// How often the pointer is checked.
const POLL_MS: i64 = 100;

// U+FE0E asks for the text (monochrome) form of a symbol, not the color emoji.
#[cfg(target_os = "windows")]
const MINIMIZE_GLYPH: &str = "\u{1F5D5}\u{FE0E}";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Minimize,
    Maximize,
    Close,
}

struct Bar {
    window: Window,
    controller: OverlayController,
    /// Holds the page below the pinned bar.
    spacer: Panel,
    maximize: LabelButton,
    mode: TopBarMode,
    state: TopBar,
    /// Whether the first tab opened, so the tabs changing afterwards flashes the bar.
    started: bool,
}

thread_local! {
    static BAR: RefCell<Option<Bar>> = const { RefCell::new(None) };
}

/// Creates the bar as an overlay of `window` in `mode` and returns the spacer, the panel that
/// keeps the page below the pinned bar; add it to the window's layout below the frame's top
/// side. Call it before the other overlays, so they stay above the bar.
pub fn create(window: &Window, mode: TopBarMode) -> Option<Panel> {
    let spacer = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    let panel = panel_create(Some(&mut DarkPanelDelegate::new(STRIP_COLOR, STRIP_HEIGHT)))?;
    let layout = panel.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }))?;
    let mut strip = View::from(&tab_strip::create()?);
    let buttons = buttons_panel()?;
    let maximize = buttons.1;
    let mut buttons = View::from(&buttons.0);
    if cfg!(target_os = "macos") {
        panel.add_child_view(Some(&mut buttons));
        panel.add_child_view(Some(&mut strip));
    } else {
        panel.add_child_view(Some(&mut strip));
        panel.add_child_view(Some(&mut buttons));
    }
    layout.set_flex_for_view(Some(&mut strip), 1);

    let controller =
        window.add_overlay_view(Some(&mut View::from(&panel)), DockingMode::CUSTOM, 0)?;
    BAR.set(Some(Bar {
        window: window.clone(),
        controller,
        spacer: spacer.clone(),
        maximize,
        mode,
        state: TopBar::default(),
        started: false,
    }));
    apply_mode();
    poll();
    Some(spacer)
}

pub fn destroy() {
    BAR.take();
}

/// The window buttons in platform order, and the maximize/restore button.
fn buttons_panel() -> Option<(Panel, LabelButton)> {
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
        let button = window_button(action)?;
        panel.add_child_view(Some(&mut View::from(&button)));
        if action == Action::Maximize {
            maximize = Some(button);
        }
    }
    Some((panel, maximize?))
}

fn window_button(action: Action) -> Option<LabelButton> {
    let (glyph, name) = match action {
        Action::Minimize => (MINIMIZE_GLYPH, "Minimize"),
        Action::Maximize => (MAXIMIZE_GLYPH, "Maximize"),
        Action::Close => (CLOSE_GLYPH, "Close"),
    };
    let button = label_button_create(
        Some(&mut WindowButtonDelegate::new(action)),
        Some(&CefString::from(glyph)),
    )?;
    button.set_font_list(Some(&CefString::from(BUTTON_FONT)));
    button.set_enabled_text_colors(BUTTON_TEXT_COLOR);
    button.set_background_color(STRIP_COLOR);
    button.set_ink_drop_enabled(0);
    button.set_focusable(0);
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width: BUTTON_WIDTH,
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

/// Switches to `mode`: pinned shows the bar and moves the page below it; auto hides it until
/// the pointer reaches the top edge.
pub fn set_mode(mode: TopBarMode) {
    BAR.with_borrow_mut(|bar| {
        if let Some(bar) = bar.as_mut() {
            bar.mode = mode;
        }
    });
    apply_mode();
}

fn apply_mode() {
    let Some((window, spacer, mode)) = BAR.with_borrow(|bar| {
        bar.as_ref()
            .map(|bar| (bar.window.clone(), bar.spacer.clone(), bar.mode))
    }) else {
        return;
    };
    spacer.set_visible((mode == TopBarMode::Pinned).into());
    window.layout();
    update();
}

/// Shows the bar for a moment in auto mode; call it when the tabs or the active tab change.
/// The first tab, when the window opens, does not flash it.
pub fn tabs_changed() {
    let flashed = BAR.with_borrow_mut(|bar| {
        let bar = bar.as_mut()?;
        if bar.started {
            bar.state.flash(Instant::now());
        }
        bar.started = true;
        Some(())
    });
    if flashed.is_some() {
        update();
    }
}

/// Places the bar below the frame's top side, inset by the frame, and shows the window's
/// state on the maximize button. Call it whenever the window is laid out.
pub fn reposition() {
    let Some((window, controller, maximize)) = BAR.with_borrow(|bar| {
        bar.as_ref().map(|bar| {
            (
                bar.window.clone(),
                bar.controller.clone(),
                bar.maximize.clone(),
            )
        })
    }) else {
        return;
    };
    let size = window.size();
    let padding = i32::try_from(window::padding()).unwrap_or(0);
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

/// The part of the shown bar right of the tab labels and +N count, up to the window buttons,
/// in window coordinates: it drags the window.
pub fn empty_area() -> Option<Rect> {
    let controller = BAR.with_borrow(|bar| bar.as_ref().map(|bar| bar.controller.clone()))?;
    if controller.is_visible() == 0 {
        return None;
    }
    // The strip's bounds are relative to the bar, which takes the overlay's bounds.
    let bar = controller.bounds();
    let strip = tab_strip::panel()?.bounds();
    let labels_end = tab_strip::labels_end();
    Some(Rect {
        x: bar.x + strip.x + labels_end,
        y: bar.y,
        width: strip.width - labels_end,
        height: bar.height,
    })
}

/// Shows or hides the bar for the pointer's place and the time.
fn update() {
    let Some((window, controller, mode, padding)) = BAR.with_borrow(|bar| {
        bar.as_ref().map(|bar| {
            (
                bar.window.clone(),
                bar.controller.clone(),
                bar.mode,
                window::padding(),
            )
        })
    }) else {
        return;
    };
    let size = window.size();
    let point = (window.is_minimized() == 0)
        .then(|| pointer_in(&window))
        .flatten();
    // A maximized window on Windows reaches past the screen's top; the pointer stops there.
    let cut = window.display().map_or(0, |display| {
        display.work_area().y - window.bounds_in_screen().y
    });
    let pointer = top_bar::pointer_zone(
        point,
        size.width,
        cut,
        i32::try_from(padding).unwrap_or(0),
        STRIP_HEIGHT,
    );
    let Some(visible) = BAR.with_borrow_mut(|bar| {
        bar.as_mut()
            .map(|bar| bar.state.update(mode, pointer, Instant::now()))
    }) else {
        return;
    };
    if (controller.is_visible() == 1) != visible {
        if visible {
            reposition();
            tab_strip::fit();
        }
        controller.set_visible(visible.into());
        window::update_drag_regions();
    }
}

/// Checks the pointer every [`POLL_MS`] while the window exists.
fn poll() {
    let mut task = Poll::new();
    post_delayed_task(ThreadId::UI, Some(&mut task), POLL_MS);
}

wrap_task! {
    struct Poll;

    impl Task {
        fn execute(&self) {
            if BAR.with_borrow(Option::is_some) {
                update();
                poll();
            }
        }
    }
}

/// The pointer's place in `window` coordinates, or `None` when it is not over the window.
fn pointer_in(window: &Window) -> Option<(i32, i32)> {
    let point = pointer_on_screen(window)?;
    // `convert_point_from_screen` leaves the point unchanged for a window, so subtract the
    // window's origin instead.
    let origin = window.bounds_in_screen();
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

fn run(action: Action) {
    let Some(window) = BAR.with_borrow(|bar| bar.as_ref().map(|bar| bar.window.clone())) else {
        return;
    };
    match action {
        Action::Minimize => window.minimize(),
        Action::Maximize if window.is_maximized() == 1 => window.restore(),
        Action::Maximize => window.maximize(),
        Action::Close => tabs::quit(),
    }
}

wrap_button_delegate! {
    struct WindowButtonDelegate {
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
            let mut task = RunAction::new(self.action);
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
        action: Action,
    }

    impl Task {
        fn execute(&self) {
            run(self.action);
        }
    }
}
