//! The frameless, dark top-level window and the browser view inside it. Window and browser view
//! delegates are derived from the cef-rs `cefsimple` example
//! (https://github.com/tauri-apps/cef-rs), licensed Apache-2.0 OR MIT.

use std::{cell::RefCell, rc::Rc};

use cef::*;
use mini_core::settings::PADDING_DEFAULT;

/// Near-black ARGB color behind the page: window, padding frame and the browser before paint.
pub const BACKGROUND_COLOR: u32 = 0xFF11_1111;

/// Initial window size in density-independent pixels; the window opens centered.
const INITIAL_SIZE: Size = Size {
    width: 1280,
    height: 800,
};

/// Opens a top-level window with one browser view loading `url`.
pub fn open(client: &mut Client, url: &str) {
    let settings = BrowserSettings {
        background_color: BACKGROUND_COLOR,
        ..Default::default()
    };
    let mut delegate = MiniBrowserViewDelegate::new();
    let browser_view = browser_view_create(
        Some(client),
        Some(&CefString::from(url)),
        Some(&settings),
        None,
        None,
        Some(&mut delegate),
    );

    // The window shows itself once created.
    let mut delegate = MiniWindowDelegate::new(Rc::new(RefCell::new(browser_view)));
    window_create_top_level(Some(&mut delegate));
}

/// Box layout that insets the page by `padding` pixels on every side and stretches it to fill
/// the rest of the window.
fn padded_layout(padding: u32) -> BoxLayoutSettings {
    let padding = i32::try_from(padding).unwrap_or(i32::MAX);
    BoxLayoutSettings {
        inside_border_insets: Insets {
            top: padding,
            left: padding,
            bottom: padding,
            right: padding,
        },
        cross_axis_alignment: AxisAlignment::STRETCH,
        ..Default::default()
    }
}

wrap_window_delegate! {
    struct MiniWindowDelegate {
        browser_view: Rc<RefCell<Option<BrowserView>>>,
    }

    impl ViewDelegate {
        fn preferred_size(&self, _view: Option<&mut View>) -> Size {
            INITIAL_SIZE
        }

        fn on_theme_changed(&self, view: Option<&mut View>) {
            // Every theme change (including the Chrome theme applied when the window is created)
            // resets the background, so reapply the dark color here.
            if let Some(view) = view {
                view.set_background_color(BACKGROUND_COLOR);
            }
        }
    }

    impl PanelDelegate {}

    impl WindowDelegate {
        fn on_window_created(&self, window: Option<&mut Window>) {
            let browser_view = self.browser_view.borrow();
            let (Some(window), Some(browser_view)) = (window, browser_view.as_ref()) else {
                return;
            };

            window.set_background_color(BACKGROUND_COLOR);
            let layout = window.set_to_box_layout(Some(&padded_layout(PADDING_DEFAULT)));
            let mut view = View::from(browser_view);
            window.add_child_view(Some(&mut view));
            if let Some(layout) = layout {
                layout.set_flex_for_view(Some(&mut view), 1);
            }

            window.center_window(Some(&INITIAL_SIZE));
            window.show();
            browser_view.request_focus();
        }

        fn on_window_destroyed(&self, _window: Option<&mut Window>) {
            self.browser_view.borrow_mut().take();
        }

        fn can_close(&self, _window: Option<&mut Window>) -> i32 {
            // Allow the window to close if the browser says it's OK.
            let browser = self.browser_view.borrow().as_ref().and_then(|view| view.browser());
            match browser.and_then(|browser| browser.host()) {
                Some(host) => host.try_close_browser(),
                None => 1,
            }
        }

        fn is_frameless(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn with_standard_window_buttons(&self, _window: Option<&mut Window>) -> i32 {
            0
        }

        fn can_resize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn can_maximize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn can_minimize(&self, _window: Option<&mut Window>) -> i32 {
            1
        }

        fn window_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::DEFAULT
        }
    }
}

wrap_browser_view_delegate! {
    struct MiniBrowserViewDelegate {}

    impl ViewDelegate {}

    impl BrowserViewDelegate {
        fn chrome_toolbar_type(&self, _browser_view: Option<&mut BrowserView>) -> ChromeToolbarType {
            ChromeToolbarType::NONE
        }

        fn browser_runtime_style(&self) -> RuntimeStyle {
            RuntimeStyle::DEFAULT
        }
    }
}
