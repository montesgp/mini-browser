// Derived from the cef-rs `cefsimple` example (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

use cef::*;

use crate::shell;

/// Entry point called by CEF's `bootstrap.exe` for the browser process and every sandboxed
/// subprocess.
#[unsafe(no_mangle)]
unsafe extern "C" fn RunWinMain(
    instance: sys::HINSTANCE,
    _command_line: *const u8,
    _command_show: i32,
    sandbox_info: *mut u8,
) -> i32 {
    let _library = shell::load_cef();

    let main_args = MainArgs { instance };
    let args = args::Args::from(main_args);
    // cef-rs rebuilds the Windows command line by joining `env::args()` with spaces, which
    // drops the quotes around arguments such as `--url "rust cef"`. Parse the original
    // command line instead.
    let Some(cmd_line) = command_line_create() else {
        return 1;
    };
    cmd_line.init_from_string(Some(&CefString::from(original_command_line().as_str())));

    shell::run_main(args.as_main_args(), &cmd_line, sandbox_info)
}

/// The process command line exactly as Windows received it, quotes included.
fn original_command_line() -> String {
    // SAFETY: `GetCommandLineW` returns a pointer to the process's NUL-terminated command
    // line, valid for the life of the process.
    unsafe {
        let start = windows_sys::Win32::System::Environment::GetCommandLineW();
        let mut len = 0;
        while *start.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(start, len))
    }
}
