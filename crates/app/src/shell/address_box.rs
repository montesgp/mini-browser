//! The floating address box: a dark text field centered near the top of the window, shown as a
//! window overlay over the page.

use std::cell::RefCell;

use cef::*;
use mini_core::address::resolve_input;

use super::{commands, tabs, window::DarkPanelDelegate};

pub const PLACEHOLDER: &str = "Enter URL or search…";
/// Background of the box and its text field.
pub const BOX_COLOR: u32 = 0xFF1E_1E1E;
pub const TEXT_COLOR: u32 = 0xFFED_EDED;
pub const PLACEHOLDER_COLOR: u32 = 0xFF8A_8A8A;
pub const BORDER_COLOR: u32 = 0xFF3A_3A3A;
pub const SELECTION_COLOR: u32 = 0xFF2F_4A6D;

const MAX_WIDTH: i32 = 640;
/// Space kept free on each side of the box in narrow windows.
const SIDE_MARGIN: i32 = 32;
const INSET: i32 = 10;
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
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }));
    panel.add_child_view(Some(&mut View::from(&textfield)));

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

/// Shows the box with `text` selected and focuses it.
pub fn show(text: &str) {
    let Some((_, controller, _, textfield)) = handles() else {
        return;
    };
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
    let width = MAX_WIDTH.min(size.width - 2 * SIDE_MARGIN).max(1);
    let height = panel.preferred_size().height;
    controller.set_bounds(Some(&Rect {
        x: (size.width - width) / 2,
        y: size.height / 5,
        width,
        height,
    }));
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
            let Some(event) = event.filter(|event| event.type_ == KeyEventType::RAWKEYDOWN) else {
                return 0;
            };
            match event.windows_key_code {
                VK_RETURN => submit(),
                VK_ESCAPE => {
                    hide();
                    tabs::focus_active_page();
                }
                _ => return commands::handle_event(Some(event)).into(),
            }
            1
        }
    }
}
