// Derived from the cef-rs `cefsimple` example (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

#![cfg_attr(
    all(not(debug_assertions), not(feature = "sandbox"), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[cfg(not(all(feature = "sandbox", target_os = "windows")))]
mod scheme;
#[cfg(not(all(feature = "sandbox", target_os = "windows")))]
mod shell;

#[cfg(target_os = "macos")]
mod mac;

#[cfg(not(all(feature = "sandbox", target_os = "windows")))]
fn main() -> std::process::ExitCode {
    let _library = shell::load_cef();

    let args = cef::args::Args::new();
    let Some(cmd_line) = args.as_cmd_line() else {
        eprintln!("Failed to parse command line arguments");
        return std::process::ExitCode::FAILURE;
    };

    let code = shell::run_main(args.as_main_args(), &cmd_line, std::ptr::null_mut());
    std::process::ExitCode::from(u8::try_from(code).unwrap_or(1))
}

#[cfg(all(feature = "sandbox", target_os = "windows"))]
fn main() -> Result<(), &'static str> {
    Err(
        "With the sandbox on Windows, run the bundle: cargo run -p xtask --bin bundle -- minibrowser",
    )
}
