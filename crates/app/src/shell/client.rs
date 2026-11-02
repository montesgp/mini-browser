//! Browser-level callbacks and the list of open browsers. All of it runs on the CEF UI thread,
//! so the list lives in thread-local storage.

use std::{
    cell::{Cell, RefCell},
    time::Instant,
};

use cef::*;
use mini_core::popups::PopupThrottle;

use super::{commands, tabs, window};

thread_local! {
    static BROWSERS: RefCell<Vec<Browser>> = const { RefCell::new(Vec::new()) };
    static CLOSING: Cell<bool> = const { Cell::new(false) };
    /// Popups each page (keyed by browser identifier) opened recently.
    static POPUPS: RefCell<PopupThrottle<i32>> = RefCell::new(PopupThrottle::default());
}

wrap_client! {
    pub struct MiniClient;

    impl Client {
        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(MiniLifeSpanHandler::new())
        }

        fn keyboard_handler(&self) -> Option<KeyboardHandler> {
            Some(MiniKeyboardHandler::new())
        }

        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(MiniDisplayHandler::new())
        }

        fn drag_handler(&self) -> Option<DragHandler> {
            Some(MiniDragHandler::new())
        }
    }
}

wrap_drag_handler! {
    struct MiniDragHandler;

    impl DragHandler {
        fn on_draggable_regions_changed(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _regions: Option<&[DraggableRegion]>,
        ) {
            // Pages never drag the window; CEF has just merged the page's regions into the
            // window's, so restore the padding frame's.
            window::update_drag_regions_later();
        }
    }
}

/// Runs shortcuts pressed while a page has the focus, before the page sees them.
fn pre_key_event(event: Option<&KeyEvent>) -> i32 {
    commands::handle_event(event).into()
}

// The native event type differs per platform, so each one gets its own signature.
wrap_keyboard_handler! {
    struct MiniKeyboardHandler;

    impl KeyboardHandler {
        #[cfg(target_os = "windows")]
        fn on_pre_key_event(
            &self,
            _browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: Option<&mut sys::MSG>,
            _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            pre_key_event(event)
        }

        #[cfg(target_os = "macos")]
        fn on_pre_key_event(
            &self,
            _browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: *mut u8,
            _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            pre_key_event(event)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        fn on_pre_key_event(
            &self,
            _browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: Option<&mut sys::XEvent>,
            _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            pre_key_event(event)
        }
    }
}

wrap_display_handler! {
    struct MiniDisplayHandler;

    impl DisplayHandler {
        fn on_title_change(&self, browser: Option<&mut Browser>, title: Option<&CefString>) {
            if let (Some(browser), Some(title)) = (browser, title) {
                tabs::set_title(browser, &title.to_string());
            }
        }

        fn on_address_change(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            let is_main = frame.is_some_and(|frame| frame.is_main() == 1);
            if let (true, Some(browser), Some(url)) = (is_main, browser, url) {
                tabs::set_url(browser, &url.to_string());
            }
        }
    }
}

wrap_life_span_handler! {
    struct MiniLifeSpanHandler;

    impl LifeSpanHandler {
        fn on_before_popup(
            &self,
            browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: i32,
            _target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            _target_disposition: WindowOpenDisposition,
            _user_gesture: i32,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut i32>,
        ) -> i32 {
            // Popups become tabs, so a page may only open a few at a time; cancel the rest.
            let Some(browser) = browser else {
                return 0;
            };
            let opener = browser.identifier();
            let allowed = POPUPS.with_borrow_mut(|popups| popups.allow(opener, Instant::now()));
            (!allowed).into()
        }

        fn on_after_created(&self, browser: Option<&mut Browser>) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);
            if let Some(browser) = browser {
                BROWSERS.with_borrow_mut(|browsers| browsers.push(browser.clone()));
            }
        }

        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            // A tab other than the last one closes on its own, without closing the window.
            if browser.is_some_and(|browser| tabs::close_tab_for(browser)) {
                return 1;
            }
            // Closing the last browser closes the window and the app; see the DoClose()
            // documentation in the CEF header for the full close sequence.
            if BROWSERS.with_borrow(Vec::len) == 1 {
                CLOSING.set(true);
            }
            // Allow the close.
            0
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);
            let Some(browser) = browser else {
                return;
            };
            let opener = browser.identifier();
            POPUPS.with_borrow_mut(|popups| popups.forget(&opener));
            let remaining = BROWSERS.with_borrow_mut(|browsers| {
                browsers.retain(|open| open.is_same(Some(&mut browser.clone())) == 0);
                browsers.len()
            });
            if remaining == 0 {
                quit_message_loop();
            }
        }
    }
}

/// Whether the last browser is closing, i.e. the app is shutting down.
#[cfg(target_os = "macos")]
pub fn is_closing() -> bool {
    CLOSING.get()
}

/// Asks every open browser to close; the app quits once the last one is gone.
#[cfg(target_os = "macos")]
pub fn close_all_browsers(force_close: bool) {
    let browsers = BROWSERS.with_borrow(Vec::clone);
    for browser in browsers {
        if let Some(host) = browser.host() {
            host.close_browser(force_close.into());
        }
    }
}

/// Shows the window of the first open browser.
#[cfg(target_os = "macos")]
pub fn show_main_window() {
    let Some(mut browser) = BROWSERS.with_borrow(|browsers| browsers.first().cloned()) else {
        return;
    };
    if let Some(window) =
        browser_view_get_for_browser(Some(&mut browser)).and_then(|view| view.window())
    {
        window.show();
    }
}
