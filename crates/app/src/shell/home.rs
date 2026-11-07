//! The home page `mini://home/`: serves its files, embedded in the app, and runs the commands
//! it asks for with `fetch('mini://home/cmd/<name>')`. A command runs only when the request
//! comes from the main frame of the active tab of its own window while that tab shows the home
//! page, and it runs in that window; any other request (web pages, frames, other tabs) gets
//! `403` and runs nothing. Routing lives in `mini_core::home`.

use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
};

use cef::*;
use mini_core::{
    address::resolve_input,
    home::{self, Asset, HomeCommand, Route},
};

use super::{app, commands, settings_box, tabs, windows::WindowId};

const INDEX_HTML: &str = include_str!("../../home/index.html");
const HOME_CSS: &str = include_str!("../../home/home.css");
const HOME_JS: &str = include_str!("../../home/home.js");

/// Sent with every reply: the page loads nothing but its own files and cannot be framed.
const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; script-src 'self'; \
    style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; \
    base-uri 'none'; form-action 'none'";

/// Serves `mini://home/` from now on. Call it once the CEF context is initialized.
pub fn register_handler() {
    let mut factory = HomeSchemeFactory::new();
    register_scheme_handler_factory(
        Some(&CefString::from(home::SCHEME)),
        Some(&CefString::from(home::HOST)),
        Some(&mut factory),
    );
}

/// Focuses the home page's address field in the active tab of window `win`.
pub fn focus_address_field(win: WindowId) {
    tabs::focus_active_page(win);
    tabs::with_active_browser(win, |browser| {
        if let Some(frame) = browser.main_frame() {
            frame.execute_java_script(
                Some(&CefString::from("window.miniFocusAddress?.()")),
                Some(&CefString::from(home::HOME_URL)),
                0,
            );
        }
    });
}

/// Makes every open home page read the settings again, so its commands card shows the current
/// top bar mode and padding at once.
pub fn refresh_state() {
    tabs::with_home_browsers(|browser| {
        if let Some(frame) = browser.main_frame() {
            frame.execute_java_script(
                Some(&CefString::from("window.miniRefreshState?.()")),
                Some(&CefString::from(home::HOME_URL)),
                0,
            );
        }
    });
}

/// A reply: status, MIME type and body, plus how much of the body was read.
#[derive(Clone)]
struct Reply {
    status: i32,
    mime_type: &'static str,
    body: Cow<'static, [u8]>,
    offset: usize,
}

impl Reply {
    fn new(status: i32, mime_type: &'static str, body: Cow<'static, [u8]>) -> Self {
        Self {
            status,
            mime_type,
            body,
            offset: 0,
        }
    }

    fn asset(asset: Asset) -> Self {
        let text = match asset {
            Asset::Html => INDEX_HTML,
            Asset::Css => HOME_CSS,
            Asset::Script => HOME_JS,
        };
        Self::new(200, asset.mime_type(), Cow::Borrowed(text.as_bytes()))
    }

    fn state() -> Self {
        let settings = settings_box::settings().unwrap_or_default();
        let json = home::state_json(home::platform_name(), settings.padding, settings.top_bar);
        Self::new(200, "application/json", Cow::Owned(json.into_bytes()))
    }

    fn refused() -> Self {
        Self::new(403, "text/plain", Cow::Borrowed(b"Forbidden"))
    }

    fn not_found() -> Self {
        Self::new(404, "text/plain", Cow::Borrowed(b"Not found"))
    }
}

/// A `state` or command request that passed the frame checks, answered on the UI thread.
#[derive(Clone)]
struct UiRequest {
    browser: Browser,
    command: Option<HomeCommand>,
}

/// Whether `request` is a `fetch` by the main frame `frame` while it shows the home page.
fn from_home_page(frame: Option<&Frame>, request: &Request) -> bool {
    let frame_on_home = frame.is_some_and(|frame| {
        frame.is_main() == 1 && home::is_home_url(&CefString::from(&frame.url()).to_string())
    });
    let method = CefString::from(&request.method()).to_string();
    frame_on_home && request.resource_type() == ResourceType::XHR && method == "GET"
}

/// Answers `request` on the UI thread: refused unless its browser is the active tab of its own
/// window and shows the home page; otherwise runs the command, if any, in that window and
/// replies with the current state.
fn answer(request: &UiRequest) -> Reply {
    let Some(win) = tabs::active_home_window(&request.browser) else {
        return Reply::refused();
    };
    match &request.command {
        Some(HomeCommand::Go(input)) => {
            let template = settings_box::settings().unwrap_or_default().search_template;
            if let Some(url) = resolve_input(input, &template, &app::local_files()) {
                tabs::navigate_active(win, &url);
            }
        }
        Some(HomeCommand::Run(command)) => commands::run(win, *command),
        Some(HomeCommand::Padding(steps)) => settings_box::step(*steps),
        None => {}
    }
    Reply::state()
}

wrap_scheme_handler_factory! {
    struct HomeSchemeFactory;

    impl SchemeHandlerFactory {
        fn create(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _scheme_name: Option<&CefString>,
            request: Option<&mut Request>,
        ) -> Option<ResourceHandler> {
            let request = request?;
            let url = CefString::from(&request.url()).to_string();
            let (reply, ui_request) = match home::route(&url) {
                None => (Reply::not_found(), None),
                Some(Route::Asset(asset)) => (Reply::asset(asset), None),
                // `state` and the commands are only for the page's `fetch`, never a document.
                Some(_) if request.resource_type() != ResourceType::XHR => {
                    (Reply::not_found(), None)
                }
                Some(route) => match browser {
                    Some(browser) if from_home_page(frame.as_deref(), request) => {
                        let command = match route {
                            Route::Command(command) => Some(command),
                            _ => None,
                        };
                        let browser = browser.clone();
                        (Reply::refused(), Some(UiRequest { browser, command }))
                    }
                    _ => (Reply::refused(), None),
                },
            };
            Some(HomeHandler::new(Arc::new(Mutex::new(reply)), ui_request))
        }
    }
}

// Sends `reply`; with `ui_request`, the reply is first replaced by its answer on the UI thread.
wrap_resource_handler! {
    struct HomeHandler {
        reply: Arc<Mutex<Reply>>,
        ui_request: Option<UiRequest>,
    }

    impl ResourceHandler {
        fn open(
            &self,
            _request: Option<&mut Request>,
            handle_request: Option<&mut i32>,
            callback: Option<&mut Callback>,
        ) -> i32 {
            let Some(handle_request) = handle_request else {
                return 0;
            };
            match (&self.ui_request, callback) {
                (Some(request), Some(callback)) => {
                    *handle_request = 0;
                    let mut task =
                        AnswerOnUi::new(request.clone(), self.reply.clone(), callback.clone());
                    post_task(ThreadId::UI, Some(&mut task));
                }
                _ => *handle_request = 1,
            }
            1
        }

        fn response_headers(
            &self,
            response: Option<&mut Response>,
            response_length: Option<&mut i64>,
            _redirect_url: Option<&mut CefString>,
        ) {
            let Ok(reply) = self.reply.lock() else {
                return;
            };
            if let Some(response) = response {
                response.set_status(reply.status);
                response.set_mime_type(Some(&CefString::from(reply.mime_type)));
                response.set_charset(Some(&CefString::from("utf-8")));
                for (name, value) in [
                    ("Content-Security-Policy", CONTENT_SECURITY_POLICY),
                    ("X-Frame-Options", "DENY"),
                    ("X-Content-Type-Options", "nosniff"),
                    ("Cache-Control", "no-store"),
                ] {
                    response.set_header_by_name(
                        Some(&CefString::from(name)),
                        Some(&CefString::from(value)),
                        1,
                    );
                }
            }
            if let Some(length) = response_length {
                *length = i64::try_from(reply.body.len()).unwrap_or(-1);
            }
        }

        fn read(
            &self,
            data_out: *mut u8,
            bytes_to_read: i32,
            bytes_read: Option<&mut i32>,
            _callback: Option<&mut ResourceReadCallback>,
        ) -> i32 {
            let Some(bytes_read) = bytes_read else {
                return 0;
            };
            *bytes_read = 0;
            let Ok(mut reply) = self.reply.lock() else {
                return 0;
            };
            let remaining = &reply.body[reply.offset.min(reply.body.len())..];
            let count = remaining
                .len()
                .min(usize::try_from(bytes_to_read).unwrap_or(0));
            if count == 0 || data_out.is_null() {
                return 0;
            }
            // SAFETY: CEF passes a buffer of at least `bytes_to_read` bytes, and `count` is no
            // larger; the body is a separate allocation.
            unsafe { std::ptr::copy_nonoverlapping(remaining.as_ptr(), data_out, count) };
            reply.offset += count;
            *bytes_read = i32::try_from(count).unwrap_or(0);
            1
        }
    }
}

wrap_task! {
    struct AnswerOnUi {
        request: UiRequest,
        reply: Arc<Mutex<Reply>>,
        callback: Callback,
    }

    impl Task {
        fn execute(&self) {
            let answer = answer(&self.request);
            if let Ok(mut reply) = self.reply.lock() {
                *reply = answer;
            }
            self.callback.cont();
        }
    }
}
