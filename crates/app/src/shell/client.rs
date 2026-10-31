//! Browser-level callbacks and the list of open browsers. All of it runs on the CEF UI thread,
//! so the list lives in thread-local storage.

use std::cell::{Cell, RefCell};

use cef::*;

use super::{commands, tabs};

thread_local! {
    static BROWSERS: RefCell<Vec<Browser>> = const { RefCell::new(Vec::new()) };
    static CLOSING: Cell<bool> = const { Cell::new(false) };
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
