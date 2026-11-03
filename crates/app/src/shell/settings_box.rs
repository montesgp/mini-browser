//! The settings box: a small dark overlay to change the padding frame and when the top bar
//! shows, with a thin edge in the address box's border color so it stands out on dark pages.
//! Every change applies at once and is saved to the settings file.

use std::{cell::RefCell, path::PathBuf};

use cef::*;
use mini_core::{
    layout,
    settings::{PADDING_MAX, PADDING_MIN, Settings, TopBarMode},
};

use super::{
    address_box::{self, BORDER_COLOR, BOX_COLOR, TEXT_COLOR},
    tabs, top_bar,
    window::{self, DarkPanelDelegate},
};

const WIDTH: i32 = 280;
const INSET: i32 = 14;
const EDGE: i32 = 1;
const DIM_TEXT_COLOR: u32 = 0xFF8C_8C8C;
/// A `−` / `+` button whose direction is at its bound.
const BOUND_BUTTON_COLOR: u32 = 0xFF4A_4A4A;
const WARNING_TEXT_COLOR: u32 = 0xFFD8_A657;
/// Shown while the last change could not be saved to the settings file.
const NOT_SAVED: &str = "Not saved: applies until restart";
const FONT: &str = "Segoe UI, Helvetica, Arial, 14px";
const TITLE_FONT: &str = "Segoe UI, Helvetica, Arial, Bold 15px";
/// The `−` and `+` glyphs; at this size their strokes look equally heavy.
const STEP_FONT: &str = "Segoe UI, Helvetica, Arial, 20px";
const VALUE_WIDTH: i32 = 56;
const BUTTON_SIZE: i32 = 28;
const MODE_WIDTH: i32 = 64;
/// Background of the selected top bar mode.
const SELECTED_COLOR: u32 = 0xFF38_3838;

const VK_ESCAPE: i32 = 0x1B;

struct SettingsBox {
    window: Window,
    controller: OverlayController,
    panel: Panel,
    minus: LabelButton,
    value: LabelButton,
    plus: LabelButton,
    auto: LabelButton,
    pinned: LabelButton,
    not_saved: LabelButton,
    settings: Settings,
    path: PathBuf,
}

thread_local! {
    static SETTINGS_BOX: RefCell<Option<SettingsBox>> = const { RefCell::new(None) };
}

/// Creates the (hidden) box as an overlay of `window`. Changes are saved to `path`.
pub fn create(window: &Window, settings: Settings, path: PathBuf) {
    let Some(panel) = panel_create(Some(&mut DarkPanelDelegate::new(BOX_COLOR, 0))) else {
        return;
    };
    panel.set_to_box_layout(Some(&BoxLayoutSettings {
        inside_border_insets: Insets {
            top: INSET,
            left: INSET,
            bottom: INSET,
            right: INSET,
        },
        between_child_spacing: 10,
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    let (
        Some(title),
        Some(label),
        Some(minus),
        Some(value),
        Some(plus),
        Some(bar_label),
        Some(auto),
        Some(pinned),
        Some(not_saved),
    ) = (
        text("Settings", TITLE_FONT, TEXT_COLOR),
        text("Padding", FONT, DIM_TEXT_COLOR),
        step_button("\u{2212}", -1),
        text("", FONT, TEXT_COLOR),
        step_button("+", 1),
        text("Top bar", FONT, DIM_TEXT_COLOR),
        mode_button(TopBarMode::Auto),
        mode_button(TopBarMode::Pinned),
        text(NOT_SAVED, FONT, WARNING_TEXT_COLOR),
    )
    else {
        return;
    };
    not_saved.set_visible(0);
    value.set_horizontal_alignment(HorizontalAlignment::CENTER);
    value.set_minimum_size(Some(&Size {
        width: VALUE_WIDTH,
        height: BUTTON_SIZE,
    }));

    panel.add_child_view(Some(&mut View::from(&title)));
    for (label, controls) in [
        (&label, [&minus, &value, &plus].as_slice()),
        (&bar_label, [&auto, &pinned].as_slice()),
    ] {
        let Some(row) = row(label, controls) else {
            return;
        };
        panel.add_child_view(Some(&mut View::from(&row)));
    }
    panel.add_child_view(Some(&mut View::from(&not_saved)));
    let Some(panel) = edged(&panel) else {
        return;
    };

    let Some(controller) =
        window.add_overlay_view(Some(&mut View::from(&panel)), DockingMode::CUSTOM, 1)
    else {
        return;
    };
    SETTINGS_BOX.set(Some(SettingsBox {
        window: window.clone(),
        controller,
        panel,
        minus,
        value,
        plus,
        auto,
        pinned,
        not_saved,
        settings,
        path,
    }));
    show_padding();
    show_top_bar();
}

/// A row with `label` on the left and `controls` on the right.
fn row(label: &LabelButton, controls: &[&LabelButton]) -> Option<Panel> {
    let row = panel_create(Some(&mut DarkPanelDelegate::new(BOX_COLOR, 0)))?;
    let layout = row.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        between_child_spacing: 6,
        cross_axis_alignment: AxisAlignment::CENTER,
        ..Default::default()
    }))?;
    let mut label_view = View::from(label);
    row.add_child_view(Some(&mut label_view));
    for control in controls {
        row.add_child_view(Some(&mut View::from(*control)));
    }
    layout.set_flex_for_view(Some(&mut label_view), 1);
    Some(row)
}

pub fn destroy() {
    SETTINGS_BOX.take();
}

/// Wraps `content` in an [`EDGE`] wide frame in the border color.
fn edged(content: &Panel) -> Option<Panel> {
    let edge = panel_create(Some(&mut DarkPanelDelegate::new(BORDER_COLOR, 0)))?;
    edge.set_to_box_layout(Some(&BoxLayoutSettings {
        inside_border_insets: Insets {
            top: EDGE,
            left: EDGE,
            bottom: EDGE,
            right: EDGE,
        },
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    edge.add_child_view(Some(&mut View::from(content)));
    Some(edge)
}

/// A non-interactive line of text. Views has no label in the CEF API, so it is a disabled
/// label button.
fn text(text: &str, font: &str, color: u32) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(Choice::None)),
        Some(&CefString::from(text)),
    )?;
    button.set_font_list(Some(&CefString::from(font)));
    button.set_text_color(ButtonState::DISABLED, color);
    button.set_enabled(0);
    button.set_focusable(0);
    button.set_ink_drop_enabled(0);
    Some(button)
}

/// A `−` or `+` button that moves the padding by `steps`.
fn step_button(label: &str, steps: i32) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(Choice::Padding(steps))),
        Some(&CefString::from(label)),
    )?;
    button.set_font_list(Some(&CefString::from(STEP_FONT)));
    button.set_enabled_text_colors(TEXT_COLOR);
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width: BUTTON_SIZE,
        height: BUTTON_SIZE,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    button.set_focusable(1);
    button.set_accessible_name(Some(&CefString::from(if steps < 0 {
        "Decrease padding"
    } else {
        "Increase padding"
    })));
    Some(button)
}

/// An `Auto` or `Pinned` button that sets when the top bar shows.
fn mode_button(mode: TopBarMode) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(Choice::TopBar(mode))),
        Some(&CefString::from(mode_name(mode))),
    )?;
    button.set_font_list(Some(&CefString::from(FONT)));
    button.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let size = Size {
        width: MODE_WIDTH,
        height: BUTTON_SIZE,
    };
    button.set_minimum_size(Some(&size));
    button.set_maximum_size(Some(&size));
    button.set_focusable(1);
    Some(button)
}

fn mode_name(mode: TopBarMode) -> &'static str {
    match mode {
        TopBarMode::Auto => "Auto",
        TopBarMode::Pinned => "Pinned",
    }
}

/// Highlights the button of the current top bar mode.
fn show_top_bar() {
    let Some((mode, buttons)) = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box.as_ref().map(|settings_box| {
            (
                settings_box.settings.top_bar,
                [
                    (TopBarMode::Auto, settings_box.auto.clone()),
                    (TopBarMode::Pinned, settings_box.pinned.clone()),
                ],
            )
        })
    }) else {
        return;
    };
    for (button_mode, button) in buttons {
        let selected = button_mode == mode;
        let (background, text_color) = if selected {
            (SELECTED_COLOR, TEXT_COLOR)
        } else {
            (BOX_COLOR, DIM_TEXT_COLOR)
        };
        button.set_background_color(background);
        button.set_enabled_text_colors(text_color);
        let state = if selected { "selected" } else { "not selected" };
        let name = format!("Top bar: {} ({state})", mode_name(button_mode));
        button.set_accessible_name(Some(&CefString::from(name.as_str())));
    }
}

/// Shows the current padding and dims the button whose direction is at its bound. The button
/// stays enabled and focusable, so the keyboard focus never jumps to the other one (which
/// would make `Space` reverse direction); pressing it does nothing.
fn show_padding() {
    let Some((settings, minus, value, plus, panel)) = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box.as_ref().map(|settings_box| {
            (
                settings_box.settings.clone(),
                settings_box.minus.clone(),
                settings_box.value.clone(),
                settings_box.plus.clone(),
                settings_box.panel.clone(),
            )
        })
    }) else {
        return;
    };
    let padding = settings.padding;
    value.set_text(Some(&CefString::from(format!("{padding} px").as_str())));
    for (button, steps, name) in [
        (&minus, -1, "Decrease padding"),
        (&plus, 1, "Increase padding"),
    ] {
        let can_move = settings.can_move_padding(steps);
        button.set_enabled_text_colors(if can_move {
            TEXT_COLOR
        } else {
            BOUND_BUTTON_COLOR
        });
        let limit = if steps < 0 { PADDING_MIN } else { PADDING_MAX };
        let accessible_name = if can_move {
            name.to_owned()
        } else {
            format!("{name} (at {limit} px)")
        };
        button.set_accessible_name(Some(&CefString::from(accessible_name.as_str())));
    }
    panel.layout();
}

/// The current settings.
pub fn settings() -> Option<Settings> {
    SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box
            .as_ref()
            .map(|settings_box| settings_box.settings.clone())
    })
}

/// Moves the padding by `steps`, applies it to the window and saves it.
pub fn step(steps: i32) {
    let Some(settings) = change(|settings| settings.with_padding_steps(steps)) else {
        return;
    };
    window::set_padding(settings.padding);
    show_padding();
    // The warning line changes the box's height.
    reposition();
}

/// Sets when the top bar shows, applies it and saves it.
fn set_top_bar(mode: TopBarMode) {
    let Some(settings) = change(|settings| Settings {
        top_bar: mode,
        ..settings.clone()
    }) else {
        return;
    };
    top_bar::set_mode(settings.top_bar);
    show_top_bar();
    reposition();
}

/// Switches the top bar between auto and pinned and saves it (`Ctrl/Cmd+Shift+F`).
pub fn toggle_top_bar() {
    let mode = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box
            .as_ref()
            .map(|settings_box| settings_box.settings.top_bar)
    });
    if let Some(mode) = mode {
        set_top_bar(mode.toggled());
    }
}

/// Replaces the settings with `new` of them and saves them; returns the new settings, or `None`
/// when nothing changed. A failed save is reported, shown in the box until a later save
/// succeeds, and the change still applies for this run.
fn change(new: impl FnOnce(&Settings) -> Settings) -> Option<Settings> {
    let (settings, path, not_saved) = SETTINGS_BOX.with_borrow_mut(|settings_box| {
        let settings_box = settings_box.as_mut()?;
        let settings = new(&settings_box.settings);
        if settings == settings_box.settings {
            return None;
        }
        settings_box.settings = settings.clone();
        Some((
            settings,
            settings_box.path.clone(),
            settings_box.not_saved.clone(),
        ))
    })?;
    let saved = settings.save(&path);
    if let Err(error) = &saved {
        eprintln!(
            "minibrowser: cannot save settings to {}: {error}",
            path.display()
        );
    }
    not_saved.set_visible(saved.is_err().into());
    Some(settings)
}

fn is_visible() -> bool {
    SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box
            .as_ref()
            .is_some_and(|settings_box| settings_box.controller.is_visible() == 1)
    })
}

/// Opens the box over the page, or closes it when it is open.
pub fn toggle() {
    if is_visible() {
        hide();
        return;
    }
    let Some((controller, minus, plus, at_max)) = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box.as_ref().map(|settings_box| {
            (
                settings_box.controller.clone(),
                settings_box.minus.clone(),
                settings_box.plus.clone(),
                !settings_box.settings.can_move_padding(1),
            )
        })
    }) else {
        return;
    };
    address_box::hide();
    // Theme changes reset button backgrounds, so show the selected mode again.
    show_top_bar();
    reposition();
    controller.set_visible(1);
    // Focus a button, so the window gets `Esc` and the shortcuts.
    let target = if at_max { minus } else { plus };
    target.request_focus();
}

/// Closes the box and gives the keyboard focus back to the page.
fn hide() {
    if close() {
        tabs::focus_active_page();
    }
}

/// Closes the box without moving the focus. Returns whether the box exists.
pub fn close() -> bool {
    let controller = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box
            .as_ref()
            .map(|settings_box| settings_box.controller.clone())
    });
    if let Some(controller) = &controller {
        controller.set_visible(0);
    }
    controller.is_some()
}

/// Closes the open box on `Esc`. Returns whether the event was used.
pub fn handle_escape(event: Option<&KeyEvent>) -> bool {
    let is_escape = event.is_some_and(|event| {
        event.type_ == KeyEventType::RAWKEYDOWN && event.windows_key_code == VK_ESCAPE
    });
    if is_escape && is_visible() {
        hide();
        return true;
    }
    false
}

/// Centers the box horizontally, a fifth of the way down the window. Call it whenever the
/// window is resized.
pub fn reposition() {
    let Some((window, controller, panel)) = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box.as_ref().map(|settings_box| {
            (
                settings_box.window.clone(),
                settings_box.controller.clone(),
                settings_box.panel.clone(),
            )
        })
    }) else {
        return;
    };
    let size = window.size();
    let height = panel.preferred_size().height;
    let bounds = layout::floating_box(size.width, size.height, WIDTH, 0, height);
    controller.set_bounds(Some(&window::cef_rect(bounds)));
}

/// What a settings box button changes.
#[derive(Clone, Copy)]
enum Choice {
    /// A line of text.
    None,
    /// The padding change of a `−` / `+` button.
    Padding(i32),
    /// The top bar mode of an `Auto` / `Pinned` button.
    TopBar(TopBarMode),
}

wrap_button_delegate! {
    struct SettingsButtonDelegate {
        choice: Choice,
    }

    impl ViewDelegate {}

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            match self.choice {
                Choice::None => {}
                Choice::Padding(steps) => step(steps),
                Choice::TopBar(mode) => set_top_bar(mode),
            }
        }
    }
}
