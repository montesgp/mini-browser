//! The floating address box: a dark rounded text field centered near the top of the window,
//! shown as a window overlay over the page. The overlay around the field is transparent, so the
//! field's own outline is the box's edge on light and dark pages alike.

use std::cell::RefCell;

use cef::*;
use mini_core::{address::resolve_input, layout};

use super::{
    commands, settings_box, tabs,
    window::{self, DarkPanelDelegate},
};

pub const PLACEHOLDER: &str = "Enter URL or search…";
/// Background of the address field and the settings box.
pub const BOX_COLOR: u32 = 0xFF1E_1E1E;
pub const TEXT_COLOR: u32 = 0xFFED_EDED;
pub const PLACEHOLDER_COLOR: u32 = 0xFF8A_8A8A;
pub const BORDER_COLOR: u32 = 0xFF3A_3A3A;
pub const SELECTION_COLOR: u32 = 0xFF2F_4A6D;

const MAX_WIDTH: i32 = 640;
/// Space kept free on each side of the box in narrow windows.
const SIDE_MARGIN: i32 = 32;
/// Height of the field, taller than its text needs so the box reads as the main input.
const FIELD_HEIGHT: i32 = 44;
/// The overlay's color around the field: transparent.
const TRANSPARENT: u32 = 0;
const FONT: &str = "Segoe UI, Helvetica, Arial, 16px";

const VK_RETURN: i32 = 0x0D;
const VK_ESCAPE: i32 = 0x1B;

struct AddressBox {
    window: Window,
    controller: OverlayController,
    panel: Panel,
    textfield: Textfield,
    search_template: String,
}

thread_local! {
    static ADDRESS_BOX: RefCell<Option<AddressBox>> = const { RefCell::new(None) };
}

/// Clones the box handles, so CEF calls happen outside the borrow.
fn handles() -> Option<(Window, OverlayController, Panel, Textfield)> {
    ADDRESS_BOX.with_borrow(|address_box| {
        address_box.as_ref().map(|address_box| {
            (
                address_box.window.clone(),
                address_box.controller.clone(),
                address_box.panel.clone(),
                address_box.textfield.clone(),
            )
        })
    })
}

/// Creates the (hidden) box as an overlay of `window`. Call it after the window's other views
/// are added so the box stays on top.
pub fn create(window: &Window, search_template: &str) {
    let Some(textfield) = textfield_create(Some(&mut AddressFieldDelegate::new())) else {
        return;
    };
    textfield.set_placeholder_text(Some(&CefString::from(PLACEHOLDER)));
    textfield.set_font_list(Some(&CefString::from(FONT)));
    textfield.set_accessible_name(Some(&CefString::from("Address")));

    let Some(panel) = panel_create(Some(&mut DarkPanelDelegate::new(TRANSPARENT, FIELD_HEIGHT)))
    else {
        return;
    };
    let layout = panel.set_to_box_layout(Some(&BoxLayoutSettings {
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    let mut field_view = View::from(&textfield);
    panel.add_child_view(Some(&mut field_view));
    if let Some(layout) = layout {
        layout.set_flex_for_view(Some(&mut field_view), 1);
    }

    let Some(controller) =
        window.add_overlay_view(Some(&mut View::from(&panel)), DockingMode::CUSTOM, 1)
    else {
        return;
    };
    ADDRESS_BOX.set(Some(AddressBox {
        window: window.clone(),
        controller,
        panel,
        textfield,
        search_template: search_template.to_owned(),
    }));
}

pub fn destroy() {
    ADDRESS_BOX.take();
}

/// Shows the box with `text` selected and focuses it, closing the settings box: the two never
/// stack.
pub fn show(text: &str) {
    let Some((_, controller, _, textfield)) = handles() else {
        return;
    };
    settings_box::close();
    textfield.set_text(Some(&CefString::from(text)));
    textfield.select_all(0);
    reposition();
    controller.set_visible(1);
    textfield.request_focus();
}

pub fn hide() {
    if let Some((_, controller, _, _)) = handles() {
        controller.set_visible(0);
    }
}

/// Centers the box horizontally, a fifth of the way down the window. Call it whenever the
/// window is resized.
pub fn reposition() {
    let Some((window, controller, panel, _)) = handles() else {
        return;
    };
    let size = window.size();
    let height = panel.preferred_size().height;
    let bounds = layout::floating_box(size.width, size.height, MAX_WIDTH, SIDE_MARGIN, height);
    controller.set_bounds(Some(&window::cef_rect(bounds)));
}

/// Loads the typed address in the active tab. Empty input keeps the box open.
fn submit() {
    let Some((text, search_template)) = ADDRESS_BOX.with_borrow(|address_box| {
        address_box.as_ref().map(|address_box| {
            (
                CefString::from(&address_box.textfield.text()).to_string(),
                address_box.search_template.clone(),
            )
        })
    }) else {
        return;
    };
    let Some(url) = resolve_input(&text, &search_template) else {
        return;
    };
    hide();
    tabs::navigate_active(&url);
    tabs::focus_active_page();
}

wrap_textfield_delegate! {
    struct AddressFieldDelegate {}

    impl ViewDelegate {}

    impl TextfieldDelegate {
        fn on_key_event(&self, _textfield: Option<&mut Textfield>, event: Option<&KeyEvent>) -> i32 {
            let Some(event) = event else {
                return 0;
            };
            let submit = match event.windows_key_code {
                VK_RETURN => true,
                VK_ESCAPE => false,
                // Every event of a shortcut is swallowed, so its character never reaches the
                // field.
                _ => return commands::handle_event(Some(event)).into(),
            };
            if event.type_ == KeyEventType::RAWKEYDOWN {
                // Move the focus once CEF is done with the key event (see `commands`).
                let mut task = FinishInput::new(submit);
                post_task(ThreadId::UI, Some(&mut task));
            }
            1
        }
    }
}

// Runs `Enter` (`submit`) or `Esc` (otherwise) in the address box.
wrap_task! {
    struct FinishInput {
        submit: bool,
    }

    impl Task {
        fn execute(&self) {
            if self.submit {
                submit();
            } else {
                hide();
                tabs::focus_active_page();
            }
        }
    }
}
