//! CEF application callbacks for the browser process.

use cef::*;
use mini_core::{
    address::resolve_input,
    launch::{self, Relaunch},
    settings::{SETTINGS_FILE, Settings},
};

use std::path::PathBuf;

use super::{client::MiniClient, home, tabs, window};

/// The `--url` input of `command_line` (`--url=<input>` or `--url <input>`), empty without one.
pub fn url_input(command_line: &CommandLine) -> String {
    let name = CefString::from("url");
    let switch = (command_line.has_switch(Some(&name)) == 1)
        .then(|| CefString::from(&command_line.switch_value(Some(&name))).to_string());
    let mut arguments = CefStringList::new();
    command_line.arguments(Some(&mut arguments));
    let arguments: Vec<String> = arguments.into_iter().collect();
    launch::url_input(switch.as_deref(), &arguments)
}

// `data_dir` is the app data directory holding the profile and the settings file;
// `url_input` is the `--url` input of the command line, empty without one.
wrap_app! {
    pub struct MiniApp {
        data_dir: PathBuf,
        url_input: String,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let is_browser_process = process_type.is_none_or(|kind| kind.to_string().is_empty());
            if let (true, Some(command_line)) = (is_browser_process, command_line) {
                add_platform_switches(command_line);
                // User switches are cleared by now. When the app already runs, CEF forwards this
                // command line to it, so the `--url` input read before `initialize` goes back in.
                if !self.url_input.is_empty() {
                    command_line.append_switch_with_value(
                        Some(&CefString::from("url")),
                        Some(&CefString::from(self.url_input.as_str())),
                    );
                }
            }
        }

        fn on_register_custom_schemes(&self, registrar: Option<&mut SchemeRegistrar>) {
            crate::scheme::register(registrar);
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(MiniBrowserProcessHandler::new(
                self.data_dir.clone(),
                self.url_input.clone(),
            ))
        }
    }
}

/// On Windows, Chromium's native window occlusion tracking reports the frameless window as
/// occluded, so it never presents a frame and stays invisible. Disabling the feature only
/// affects rendering. User-supplied switches are ignored (`command_line_args_disabled`), but
/// switches added here still apply, merged with any `--disable-features` already present.
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
    struct MiniBrowserProcessHandler {
        data_dir: PathBuf,
        url_input: String,
    }

    impl BrowserProcessHandler {
        fn on_context_initialized(&self) {
            debug_assert_ne!(currently_on(ThreadId::UI), 0);

            // Chromium's dark color scheme, so pages see `prefers-color-scheme: dark`.
            if let Some(context) = request_context_get_global_context() {
                context.set_chrome_color_scheme(ColorVariant::DARK, 0);
            }

            home::register_handler();
            let settings_path = self.data_dir.join(SETTINGS_FILE);
            let settings = Settings::load(&settings_path);
            // `--url=` is resolved like address-box input, so it only ever loads a web address
            // or a search. `None` (no switch or an empty one) starts with the home page.
            let url = resolve_input(&self.url_input, &settings.search_template);
            window::open(MiniClient::new(), settings, settings_path, url);
        }

        fn on_already_running_app_relaunch(
            &self,
            command_line: Option<&mut CommandLine>,
            _current_directory: Option<&CefString>,
        ) -> i32 {
            // The app was launched again while it runs. `command_line` is the new launch's,
            // already cleared of user switches, plus its `--url` input (see
            // `on_before_command_line_processing`); only that input is used, with the first
            // launch's rules. Always handled, so Chromium never opens its default browser window
            // with tabs and toolbar.
            let url_input = command_line.map(|line| url_input(line)).unwrap_or_default();
            let settings = Settings::load(&self.data_dir.join(SETTINGS_FILE));
            if let Relaunch::OpenTab(url) = launch::relaunch(&url_input, &settings.search_template) {
                tabs::open_tab(Some(&url));
            }
            window::bring_to_front();
            1
        }

        fn default_client(&self) -> Option<Client> {
            Some(MiniClient::new())
        }
    }
}
