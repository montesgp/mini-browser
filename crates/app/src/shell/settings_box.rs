//! The settings box: a small dark overlay to change the padding frame. Every change applies at
//! once and is saved to the settings file.

use std::{cell::RefCell, path::PathBuf};

use cef::*;
use mini_core::{
    layout,
    settings::{PADDING_MAX, PADDING_MIN, Settings},
};

use super::{
    address_box::{self, BOX_COLOR, TEXT_COLOR},
    tabs,
    window::{self, DarkPanelDelegate},
};

const WIDTH: i32 = 280;
const INSET: i32 = 14;
const DIM_TEXT_COLOR: u32 = 0xFF8C_8C8C;
const DISABLED_BUTTON_COLOR: u32 = 0xFF4A_4A4A;
const WARNING_TEXT_COLOR: u32 = 0xFFD8_A657;
/// Shown while the last change could not be saved to the settings file.
const NOT_SAVED: &str = "Not saved: applies until restart";
const FONT: &str = "Segoe UI, Helvetica, Arial, 14px";
const TITLE_FONT: &str = "Segoe UI, Helvetica, Arial, Bold 15px";
const VALUE_WIDTH: i32 = 56;
const BUTTON_SIZE: i32 = 28;

const VK_ESCAPE: i32 = 0x1B;

struct SettingsBox {
    window: Window,
    controller: OverlayController,
    panel: Panel,
    minus: LabelButton,
    value: LabelButton,
    plus: LabelButton,
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
    let Some(row) = panel_create(Some(&mut DarkPanelDelegate::new(BOX_COLOR, 0))) else {
        return;
    };
    let row_layout = row.set_to_box_layout(Some(&BoxLayoutSettings {
        horizontal: 1,
        between_child_spacing: 6,
        cross_axis_alignment: AxisAlignment::CENTER,
        ..Default::default()
    }));

    let (Some(title), Some(label), Some(minus), Some(value), Some(plus), Some(not_saved)) = (
        text("Settings", TITLE_FONT, TEXT_COLOR),
        text("Padding", FONT, DIM_TEXT_COLOR),
        step_button("\u{2212}", -1),
        text("", FONT, TEXT_COLOR),
        step_button("+", 1),
        text(NOT_SAVED, FONT, WARNING_TEXT_COLOR),
    ) else {
        return;
    };
    not_saved.set_visible(0);
    value.set_horizontal_alignment(HorizontalAlignment::CENTER);
    value.set_minimum_size(Some(&Size {
        width: VALUE_WIDTH,
        height: BUTTON_SIZE,
    }));

    panel.add_child_view(Some(&mut View::from(&title)));
    let mut label_view = View::from(&label);
    row.add_child_view(Some(&mut label_view));
    row.add_child_view(Some(&mut View::from(&minus)));
    row.add_child_view(Some(&mut View::from(&value)));
    row.add_child_view(Some(&mut View::from(&plus)));
    if let Some(layout) = row_layout {
        layout.set_flex_for_view(Some(&mut label_view), 1);
    }
    panel.add_child_view(Some(&mut View::from(&row)));
    panel.add_child_view(Some(&mut View::from(&not_saved)));

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
        not_saved,
        settings,
        path,
    }));
    show_padding();
}

pub fn destroy() {
    SETTINGS_BOX.take();
}

/// A non-interactive line of text. Views has no label in the CEF API, so it is a disabled
/// label button.
fn text(text: &str, font: &str, color: u32) -> Option<LabelButton> {
    let button = label_button_create(
        Some(&mut SettingsButtonDelegate::new(0)),
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
        Some(&mut SettingsButtonDelegate::new(steps)),
        Some(&CefString::from(label)),
    )?;
    button.set_font_list(Some(&CefString::from(TITLE_FONT)));
    button.set_enabled_text_colors(TEXT_COLOR);
    button.set_text_color(ButtonState::DISABLED, DISABLED_BUTTON_COLOR);
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

/// Shows the current padding and disables the button that would leave the range.
fn show_padding() {
    let Some((padding, minus, value, plus, panel)) = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box.as_ref().map(|settings_box| {
            (
                settings_box.settings.padding,
                settings_box.minus.clone(),
                settings_box.value.clone(),
                settings_box.plus.clone(),
                settings_box.panel.clone(),
            )
        })
    }) else {
        return;
    };
    value.set_text(Some(&CefString::from(format!("{padding} px").as_str())));
    minus.set_enabled((padding > PADDING_MIN).into());
    plus.set_enabled((padding < PADDING_MAX).into());
    panel.layout();
}

/// Moves the padding by `steps`, applies it to the window and saves it. A failed save is
/// reported, shown in the box until a later save succeeds, and the new padding still applies
/// for this run.
fn step(steps: i32) {
    let Some((settings, path, not_saved)) = SETTINGS_BOX.with_borrow_mut(|settings_box| {
        let settings_box = settings_box.as_mut()?;
        let settings = settings_box.settings.with_padding_steps(steps);
        if settings == settings_box.settings {
            return None;
        }
        settings_box.settings = settings.clone();
        Some((
            settings,
            settings_box.path.clone(),
            settings_box.not_saved.clone(),
        ))
    }) else {
        return;
    };
    let saved = settings.save(&path);
    if let Err(error) = &saved {
        eprintln!(
            "minibrowser: cannot save settings to {}: {error}",
            path.display()
        );
    }
    not_saved.set_visible(saved.is_err().into());
    window::set_padding(settings.padding);
    show_padding();
    // The warning line changes the box's height.
    reposition();
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
    let Some((controller, minus, plus)) = SETTINGS_BOX.with_borrow(|settings_box| {
        settings_box.as_ref().map(|settings_box| {
            (
                settings_box.controller.clone(),
                settings_box.minus.clone(),
                settings_box.plus.clone(),
            )
        })
    }) else {
        return;
    };
    address_box::hide();
    reposition();
    controller.set_visible(1);
    // Focus a button, so the window gets `Esc` and the shortcuts.
    let target = if plus.is_enabled() == 1 { plus } else { minus };
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

// `steps` is the padding change of a `−` / `+` button, or 0 for a line of text.
wrap_button_delegate! {
    struct SettingsButtonDelegate {
        steps: i32,
    }

    impl ViewDelegate {}

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {
            if self.steps != 0 {
                step(self.steps);
            }
        }
    }
}
