//! The update check: while the setting is on, asks GitHub's API for the latest release at
//! startup and then once a day, from the browser process with CEF's `UrlRequest`. A newer
//! version shows a notice on the home page and in the settings box; its Download button opens
//! the download page in the system's default browser. Any failure is silent. The version rules,
//! the URLs and the size limit live in `mini_core::update`.

use std::{
    cell::{Cell, RefCell},
    sync::{Arc, Mutex},
};

use cef::*;
use mini_core::update::{self, CHECK_INTERVAL_MS, LATEST_RELEASE_URL, MAX_RESPONSE_BYTES, Version};

use super::{home, settings_box};

/// Identifies the app to GitHub's API, which refuses requests without a `User-Agent`.
const USER_AGENT: &str = concat!(
    "mini-browser/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/montesgp/mini-browser)"
);

thread_local! {
    /// The newest release known to be newer than this build.
    static AVAILABLE: Cell<Option<Version>> = const { Cell::new(None) };
    /// Changes whenever checks start or stop, so the daily timer and the answer of an earlier
    /// round do nothing.
    static ROUND: Cell<u64> = const { Cell::new(0) };
    /// The request in flight, kept alive until it completes.
    static PENDING: RefCell<Option<Urlrequest>> = const { RefCell::new(None) };
}

/// The newer release to announce, if any.
pub fn available() -> Option<Version> {
    AVAILABLE.get()
}

/// Starts the checks when the setting is on. Call it once, after the first window opened.
pub fn start() {
    if enabled() {
        begin();
    }
}

/// Follows a change of the setting: on checks right away and then daily; off stops the checks
/// and hides any notice.
pub fn set_enabled(enabled: bool) {
    if enabled {
        begin();
    } else {
        next_round();
        cancel_pending();
        set_available(None);
    }
}

fn enabled() -> bool {
    settings_box::settings().is_some_and(|settings| settings.check_for_updates)
}

fn next_round() -> u64 {
    let round = ROUND.get().wrapping_add(1);
    ROUND.set(round);
    round
}

/// Starts a new round: drops the request of the earlier one, checks now and schedules the daily
/// check.
fn begin() {
    let round = next_round();
    cancel_pending();
    check(round);
    schedule(round);
}

fn schedule(round: u64) {
    let mut task = DailyCheck::new(round);
    post_delayed_task(ThreadId::UI, Some(&mut task), CHECK_INTERVAL_MS);
}

fn cancel_pending() {
    if let Some(request) = PENDING.take() {
        request.cancel();
    }
}

/// Asks for the latest release unless a request is still in flight; the answer arrives in
/// [`UpdateClient`] on this (UI) thread.
fn check(round: u64) {
    if PENDING.with_borrow(Option::is_some) {
        return;
    }
    let Some(mut request) = request_create() else {
        return;
    };
    request.set_url(Some(&CefString::from(LATEST_RELEASE_URL)));
    request.set_method(Some(&CefString::from("GET")));
    for (name, value) in [
        ("Accept", "application/vnd.github+json"),
        ("User-Agent", USER_AGENT),
    ] {
        request.set_header_by_name(
            Some(&CefString::from(name)),
            Some(&CefString::from(value)),
            1,
        );
    }
    // No cookies or credentials are sent or stored (no `UR_FLAG_ALLOW_STORED_CREDENTIALS`), and
    // a redirect ends the request, so only the fixed URL is ever fetched.
    #[allow(clippy::unnecessary_cast)]
    let flags = sys::cef_urlrequest_flags_t::UR_FLAG_STOP_ON_REDIRECT.0 as i32;
    request.set_flags(flags);
    let mut client = UpdateClient::new(round, Arc::new(Mutex::new(Some(Vec::new()))));
    if let Some(pending) = urlrequest_create(Some(&mut request), Some(&mut client), None) {
        PENDING.set(Some(pending));
    }
}

/// Keeps `version` when it is newer than the known one (or `None` to forget it) and shows the
/// change on the open home pages and settings boxes.
fn set_available(version: Option<Version>) {
    let current = AVAILABLE.get();
    let next = match (version, current) {
        (Some(version), Some(current)) if version <= current => return,
        (version, _) => version,
    };
    if next == current {
        return;
    }
    AVAILABLE.set(next);
    home::refresh_state();
    settings_box::show_update_everywhere();
}

/// Opens the download page in the system's default browser; the URL is the fixed constant.
pub fn open_download_page() {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

        let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
        let (operation, url) = (wide("open"), wide(update::DOWNLOAD_PAGE_URL));
        // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call; null is allowed
        // for the owner window, the parameters and the directory.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                url.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // Values up to 32 are errors.
        if result as usize <= 32 {
            eprintln!("minibrowser: cannot open {}", update::DOWNLOAD_PAGE_URL);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let opener = if cfg!(target_os = "macos") {
            "/usr/bin/open"
        } else {
            "xdg-open"
        };
        match std::process::Command::new(opener)
            .arg(update::DOWNLOAD_PAGE_URL)
            .spawn()
        {
            // Reap the opener when it exits, so it leaves no zombie process.
            Ok(mut child) => {
                std::thread::spawn(move || child.wait());
            }
            Err(error) => eprintln!(
                "minibrowser: cannot open {}: {error}",
                update::DOWNLOAD_PAGE_URL
            ),
        }
    }
}

wrap_task! {
    struct DailyCheck {
        round: u64,
    }

    impl Task {
        fn execute(&self) {
            if ROUND.get() == self.round && enabled() {
                // A request still unanswered after a day is stuck: drop it so the check runs.
                cancel_pending();
                check(self.round);
                schedule(self.round);
            }
        }
    }
}

// Collects the response body of round `round`; `body` becomes `None` once it is too large.
wrap_urlrequest_client! {
    struct UpdateClient {
        round: u64,
        body: Arc<Mutex<Option<Vec<u8>>>>,
    }

    impl UrlrequestClient {
        fn on_download_data(
            &self,
            request: Option<&mut Urlrequest>,
            data: *const u8,
            data_length: usize,
        ) {
            let Ok(mut body) = self.body.lock() else {
                return;
            };
            let Some(bytes) = body.as_mut() else {
                return;
            };
            if data.is_null() || bytes.len().saturating_add(data_length) > MAX_RESPONSE_BYTES {
                *body = None;
                if let Some(request) = request {
                    request.cancel();
                }
                return;
            }
            // SAFETY: CEF passes `data_length` readable bytes at `data`, valid for this call.
            bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(data, data_length) });
        }

        fn on_request_complete(&self, request: Option<&mut Urlrequest>) {
            if ROUND.get() != self.round {
                return;
            }
            PENDING.set(None);
            let Some(request) = request else {
                return;
            };
            let succeeded = request.request_status() == UrlrequestStatus::SUCCESS
                && request
                    .response()
                    .is_some_and(|response| (200..300).contains(&response.status()));
            let body = self.body.lock().ok().and_then(|mut body| body.take());
            let version = body
                .filter(|_| succeeded)
                .and_then(|body| update::newer_release_in_response(env!("CARGO_PKG_VERSION"), &body));
            if version.is_some() {
                set_available(version);
            }
        }
    }
}
