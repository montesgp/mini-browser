//! CEF application callbacks for the browser process.

use cef::*;
use mini_core::{
    address::resolve_input,
    settings::{SETTINGS_FILE, Settings},
};

use super::{client::MiniClient, data_dir, window};

wrap_app! {
    pub struct MiniApp;

    impl App {
        fn on_before_command_line_processing(
            &self,
            process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let is_browser_process = process_type.is_none_or(|kind| kind.to_string().is_empty());
            if let (true, Some(command_line)) = (is_browser_process, command_line) {
                add_platform_switches(command_line);
            }
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(MiniBrowserProcessHandler::new())
        }
    }
}

/// On Windows, Chromium's native window occlusion tracking reports the frameless window as
/// occluded, so it never presents a frame and stays invisible. Disabling the feature only
/// affects rendering; it keeps any `--disable-features` the user passed.
#[cfg(target_os = "windows")]
fn add_platform_switches(command_line: &mut CommandLine) {
    const FEATURE: &str = "CalculateNativeWinOcclusion";
    let name = CefString::from("disable-features");
    let current = CefString::from(&command_line.switch_value(Some(&name))).to_string();
    let value = if current.is_empty() {
        FEATURE.to_owned()
    } else {
        format!("{current},{FEATURE}")
    };
    command_line.append_switch_with_value(Some(&name), Some(&CefString::from(value.as_str())));
}

#[cfg(not(target_os = "windows"))]
fn add_platform_switches(_command_line: &mut CommandLine) {}

wrap_browser_process_handler! {
    struct MiniBrowserProcessHandler;

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);

            // Chromium's dark color scheme, so pages see `prefers-color-scheme: dark`.
            if let Some(context) = request_context_get_global_context() {
                context.set_chrome_color_scheme(ColorVariant::DARK, 0);
            }

            let settings = Settings::load(&data_dir::app_data_dir().join(SETTINGS_FILE));
            let url = startup_url(&settings.search_template);
            window::open(MiniClient::new(), settings, url);
        }

        fn default_client(&self) -> Option<Client> {
            Some(MiniClient::new())
        }
    }
}

/// The `--url=` switch value resolved like address-box input, so it only ever loads a web
/// address or a search. `None` (no switch or an empty one) starts with an empty tab.
fn startup_url(search_template: &str) -> Option<String> {
    let input = command_line_get_global()
        .map(|command_line| {
            CefString::from(&command_line.switch_value(Some(&CefString::from("url")))).to_string()
        })
        .unwrap_or_default();
    resolve_input(&input, search_template)
}
