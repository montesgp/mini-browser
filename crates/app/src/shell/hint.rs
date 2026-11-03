//! The empty-tab hint: faint centered text naming the address shortcut, shown over tabs the
//! user opened without an address until they load a page. It never takes the focus.

use std::cell::RefCell;

use cef::*;
use mini_core::{
    layout,
    shortcuts::{Platform, address_hint},
};

use super::window;

const TEXT_COLOR: u32 = 0xFF5C_5C5C;
const FONT: &str = "Segoe UI, Helvetica, Arial, 14px";

struct Hint {
    window: Window,
    controller: OverlayController,
    label: LabelButton,
}

thread_local! {
    static HINT: RefCell<Option<Hint>> = const { RefCell::new(None) };
}

fn handles() -> Option<(Window, OverlayController, LabelButton)> {
    HINT.with_borrow(|hint| {
        hint.as_ref().map(|hint| {
            (
                hint.window.clone(),
                hint.controller.clone(),
                hint.label.clone(),
            )
        })
    })
}

/// Creates the (hidden) hint as an overlay of `window`. Overlays without a background are
/// transparent, so only the text shows.
pub fn create(window: &Window) {
    let text = CefString::from(address_hint(Platform::current()));
    let Some(label) = label_button_create(Some(&mut HintDelegate::new()), Some(&text)) else {
        return;
    };
    // Views has no label in the CEF API: a disabled, unfocusable button is plain text.
    label.set_font_list(Some(&CefString::from(FONT)));
    label.set_text_color(ButtonState::DISABLED, TEXT_COLOR);
    label.set_enabled(0);
    label.set_focusable(0);
    label.set_ink_drop_enabled(0);
    label.set_horizontal_alignment(HorizontalAlignment::CENTER);
    let Some(controller) =
        window.add_overlay_view(Some(&mut View::from(&label)), DockingMode::CUSTOM, 0)
    else {
        return;
    };
    controller.set_visible(0);
    HINT.set(Some(Hint {
        window: window.clone(),
        controller,
        label,
    }));
}

pub fn destroy() {
    HINT.take();
}

pub fn set_visible(visible: bool) {
    let Some((_, controller, _)) = handles() else {
        return;
    };
    if visible {
        reposition();
    }
    if (controller.is_visible() == 1) != visible {
        controller.set_visible(visible.into());
    }
}

/// Centers the hint in the window. Call it whenever the window is resized.
pub fn reposition() {
    let Some((window, controller, label)) = handles() else {
        return;
    };
    let size = window.size();
    let text = label.preferred_size();
    let bounds = layout::centered_box(size.width, size.height, text.width, text.height);
    controller.set_bounds(Some(&window::cef_rect(bounds)));
}

wrap_button_delegate! {
    struct HintDelegate {}

    impl ViewDelegate {}

    impl ButtonDelegate {
        fn on_button_pressed(&self, _button: Option<&mut Button>) {}
    }
}
