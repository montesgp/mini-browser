//! The browser shell: CEF startup, the top-level window with its padding frame, tabs, address
//! box, top bar and settings box, keyboard shortcuts and the browser client. Startup is
//! derived from the cef-rs `cefsimple` example (https://github.com/tauri-apps/cef-rs),
//! licensed Apache-2.0 OR MIT.

use cef::*;

mod address_box;
mod app;
pub mod client;
mod commands;
pub mod data_dir;
mod hint;
mod loading_line;
mod settings_box;
mod tab_strip;
mod tabs;
mod top_bar;
mod window;

#[cfg(target_os = "macos")]
pub type Library = library_loader::LibraryLoader;

#[cfg(not(target_os = "macos"))]
pub struct Library;

/// Loads the CEF library (macOS only; elsewhere it is linked) and pins the CEF API version.
/// Keep the returned value alive until CEF has shut down.
pub fn load_cef() -> Library {
    #[cfg(target_os = "macos")]
    let library = {
        let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
        assert!(loader.load());
        loader
    };
    #[cfg(not(target_os = "macos"))]
    let library = Library;

    // Initialize the CEF API version.
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    #[cfg(target_os = "macos")]
    crate::mac::setup_application();

    library
}

/// Runs a CEF subprocess, or the browser process until its last window closes. Returns the
/// process exit code.
pub fn run_main(main_args: &MainArgs, cmd_line: &CommandLine, sandbox_info: *mut u8) -> i32 {
    let is_browser_process = cmd_line.has_switch(Some(&CefString::from("type"))) != 1;

    let code = execute_process(Some(main_args), None, sandbox_info);
    if !is_browser_process {
        // Renderer, GPU and utility subprocesses finish inside `execute_process`.
        return code;
    }
    if code != -1 {
        eprintln!("CEF failed to start the browser process ({code})");
        return 1;
    }

    // Own profile directory, isolated from the user's other browsers. Without one the app does
    // not start: a shared fallback such as the temp directory would mix profiles.
    let Some(root_cache_path) = data_dir::app_data_dir() else {
        report_startup_error(data_dir::MISSING_DIR_ERROR);
        return 1;
    };
    let cache_path = root_cache_path.join("profile");

    // Read now: CEF clears the browser process's global command line on `initialize`, because
    // `command_line_args_disabled` is set below.
    let url_input = app::url_input(cmd_line);
    let mut app = app::MiniApp::new(root_cache_path.clone(), url_input);
    let settings = Settings {
        no_sandbox: (!cfg!(feature = "sandbox")).into(),
        // Chromium and CEF switches given by the user (`--no-sandbox`,
        // `--remote-debugging-port`, ...) have no effect; the app sets its own switches in
        // `on_before_command_line_processing`.
        command_line_args_disabled: 1,
        root_cache_path: CefString::from(root_cache_path.to_string_lossy().as_ref()),
        cache_path: CefString::from(cache_path.to_string_lossy().as_ref()),
        background_color: window::BACKGROUND_COLOR,
        ..Default::default()
    };
    if initialize(
        Some(main_args),
        Some(&settings),
        Some(&mut app),
        sandbox_info,
    ) != 1
    {
        // The app already runs with this profile: CEF handed this launch to it
        // (`on_already_running_app_relaunch`), so this process is done.
        let notified = sys::cef_resultcode_t::CEF_RESULT_CODE_NORMAL_EXIT_PROCESS_NOTIFIED as i32;
        if get_exit_code() == notified {
            return 0;
        }
        eprintln!("CEF failed to initialize");
        return 1;
    }

    #[cfg(target_os = "macos")]
    let _delegate = crate::mac::setup_app_delegate();

    run_message_loop();

    shutdown();
    0
}

/// Prints `message` and, on Windows, where the app has no console, also shows it in a message
/// box.
fn report_startup_error(message: &str) {
    eprintln!("{message}");
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

        let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
        let (text, caption) = (wide(message), wide("mini-browser"));
        // SAFETY: both strings are NUL-terminated UTF-16 that outlive the call; a null owner
        // window is allowed.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                caption.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}
