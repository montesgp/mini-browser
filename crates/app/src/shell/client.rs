//! Browser-level callbacks. Each one finds the window of its browser in the registry
//! (`windows`), which also lists the app's open browsers for the quit rule.

use std::{cell::RefCell, time::Duration, time::Instant};

use cef::*;
use mini_core::{address, home, popups::PopupThrottle, windows::app_quits};

use super::{commands, tabs, window, windows};

thread_local! {
    /// At most one "popup blocked" message every 10 seconds, so a page cannot flood stderr.
    static BLOCKED_LOG: RefCell<PopupThrottle<()>> =
        RefCell::new(PopupThrottle::new(1, Duration::from_secs(10)));
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

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(MiniLoadHandler::new())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(MiniRequestHandler::new())
        }
    }
}

wrap_request_handler! {
    struct MiniRequestHandler;

    impl RequestHandler {
        fn on_before_browse(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _user_gesture: i32,
            is_redirect: i32,
        ) -> i32 {
            // Only the app opens `mini:` pages. Web pages cannot link to or frame them (the
            // scheme is display-isolated), but a server's redirect is not checked against the
            // page that started the navigation, so cancel any redirect into the scheme.
            // Chromium keeps web pages from opening `file:` pages and refuses redirects into
            // them; cancel such redirects here too, and never open a file on another machine.
            let Some(request) = request else {
                return 0;
            };
            let url = CefString::from(&request.url()).to_string();
            let into_local_scheme = home::is_app_url(&url) || address::is_file_url(&url);
            ((is_redirect == 1 && into_local_scheme) || address::is_remote_file_url(&url)).into()
        }

        fn resource_request_handler(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _is_navigation: i32,
            _is_download: i32,
            _request_initiator: Option<&CefString>,
            _disable_default_handling: Option<&mut i32>,
        ) -> Option<ResourceRequestHandler> {
            // A local page may ask for `file://server/…` images, scripts or frames; on Windows
            // that opens the share and can send the user's credentials to the server.
            let url = CefString::from(&request?.url()).to_string();
            address::is_remote_file_url(&url).then(CancelRequest::new)
        }
    }
}

wrap_resource_request_handler! {
    struct CancelRequest;

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            ReturnValue::CANCEL
        }
    }
}

wrap_load_handler! {
    struct MiniLoadHandler;

    impl LoadHandler {
        fn on_loading_state_change(
            &self,
            browser: Option<&mut Browser>,
            is_loading: i32,
            _can_go_back: i32,
            _can_go_forward: i32,
        ) {
            if let Some(browser) = browser {
                tabs::set_loading(browser, is_loading == 1);
            }
        }
    }
}

wrap_drag_handler! {
    struct MiniDragHandler;

    impl DragHandler {
        fn on_draggable_regions_changed(
            &self,
            browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _regions: Option<&[DraggableRegion]>,
        ) {
            // Pages never drag the window; CEF has just merged the page's regions into the
            // window's, so restore the padding frame's.
            if let Some(win) = browser.and_then(|browser| windows::of_browser(browser)) {
                window::update_drag_regions_later(win);
            }
        }
    }
}

/// Runs shortcuts pressed while a page has the focus, in the page's window, before the page
/// sees them.
fn pre_key_event(browser: Option<&mut Browser>, event: Option<&KeyEvent>) -> i32 {
    let win = browser.and_then(|browser| windows::of_browser_or_last(browser));
    win.is_some_and(|win| commands::handle_event(win, event))
        .into()
}

// The native event type differs per platform, so each one gets its own signature.
wrap_keyboard_handler! {
    struct MiniKeyboardHandler;

    impl KeyboardHandler {
        #[cfg(target_os = "windows")]
        fn on_pre_key_event(
            &self,
            browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: Option<&mut sys::MSG>,
            _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            pre_key_event(browser, event)
        }

        #[cfg(target_os = "macos")]
        fn on_pre_key_event(
            &self,
            browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: *mut u8,
            _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            pre_key_event(browser, event)
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        fn on_pre_key_event(
            &self,
            browser: Option<&mut Browser>,
            event: Option<&KeyEvent>,
            _os_event: Option<&mut sys::XEvent>,
            _is_keyboard_shortcut: Option<&mut i32>,
        ) -> i32 {
            pre_key_event(browser, event)
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
            // Popups become tabs of their opener's window, so it only accepts a few at a time;
            // cancel the rest.
            let Some(browser) = browser else {
                return 0;
            };
            let Some(state) = windows::of_browser_or_last(browser).and_then(windows::get) else {
                return 1;
            };
            let now = Instant::now();
            let allowed = state.popups.borrow_mut().allow((), now);
            if !allowed && BLOCKED_LOG.with_borrow_mut(|log| log.allow((), now)) {
                eprintln!("minibrowser: blocking popups (more than 3 within 2 seconds)");
            }
            (!allowed).into()
        }

        fn on_after_created(&self, browser: Option<&mut Browser>) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);
            if let Some(browser) = browser {
                windows::browser_created(browser);
            }
        }

        fn do_close(&self, browser: Option<&mut Browser>) -> i32 {
            // A tab closes on its own, without closing the window, unless it is the last one
            // while the window closes.
            if browser.is_some_and(|browser| tabs::close_tab_for(browser)) {
                return 1;
            }
            // Closing a window's last browser closes the window, and with the app's last
            // browser the app; see the DoClose() documentation in the CEF header for the full
            // close sequence.
            #[cfg(target_os = "macos")]
            if windows::browser_count() == 1 {
                windows::set_closing();
            }
            // Allow the close.
            0
        }

        fn on_before_close(&self, browser: Option<&mut Browser>) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);
            let Some(browser) = browser else {
                return;
            };
            let remaining = windows::browser_closed(browser);
            // A closed last tab is replaced by a home tab, whose browser may not exist yet; the
            // app quits once its last window is gone.
            if app_quits(remaining, tabs::running_windows()) {
                quit_message_loop();
            }
        }
    }
}

/// Whether the last browser is closing, i.e. the app is shutting down.
#[cfg(target_os = "macos")]
pub fn is_closing() -> bool {
    windows::is_closing()
}

/// Asks every open browser of every window to close; the app quits once the last one is gone.
#[cfg(target_os = "macos")]
pub fn close_all_browsers(force_close: bool) {
    for win in windows::ids() {
        tabs::start_quitting(win);
    }
    for browser in windows::browsers() {
        if let Some(host) = browser.host() {
            host.close_browser(force_close.into());
        }
    }
}

/// Shows the last active window.
#[cfg(target_os = "macos")]
pub fn show_main_window() {
    if let Some(window) = windows::last_active().and_then(window::window) {
        window.show();
    }
}
