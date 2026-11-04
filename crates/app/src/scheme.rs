//! Registers the app's `mini` scheme (the home page, `mini://home/`). Every process type must
//! register it with the same options, so the browser process does it from its own app and the
//! renderer, GPU and utility processes from [`SubprocessApp`].

use cef::{sys::cef_scheme_options_t, *};
use mini_core::home::SCHEME;

/// A standard, secure scheme: its pages are a normal origin with their own storage, and may
/// `fetch` their own origin. Display isolation keeps web pages from linking to or framing it,
/// so only the app opens it.
fn options() -> i32 {
    [
        cef_scheme_options_t::CEF_SCHEME_OPTION_STANDARD,
        cef_scheme_options_t::CEF_SCHEME_OPTION_SECURE,
        cef_scheme_options_t::CEF_SCHEME_OPTION_DISPLAY_ISOLATED,
        cef_scheme_options_t::CEF_SCHEME_OPTION_FETCH_ENABLED,
    ]
    .into_iter()
    .fold(0, |options, option| options | option as i32)
}

/// Adds the `mini` scheme to `registrar`; call it from `App::on_register_custom_schemes`.
pub fn register(registrar: Option<&mut SchemeRegistrar>) {
    if let Some(registrar) = registrar {
        registrar.add_custom_scheme(Some(&CefString::from(SCHEME)), options());
    }
}

// The app of the renderer, GPU and utility processes: it only registers the scheme.
wrap_app! {
    pub struct SubprocessApp;

    impl App {
        fn on_register_custom_schemes(&self, registrar: Option<&mut SchemeRegistrar>) {
            register(registrar);
        }
    }
}
