//! Browser-level callbacks and the list of open browsers. All of it runs on the CEF UI thread,
//! so the list lives in thread-local storage.

use std::cell::{Cell, RefCell};

use cef::*;

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

        fn do_close(&self, _browser: Option<&mut Browser>) -> i32 {
            // Closing the last browser closes the app; see the DoClose() documentation in the
            // CEF header for the full close sequence.
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
