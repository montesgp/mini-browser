// Derived from the cef-rs `bundle-cef-app` tool (https://github.com/tauri-apps/cef-rs),
// licensed Apache-2.0 OR MIT.

//! Builds the app and lays out a runnable CEF bundle:
//! `cargo run -p xtask --bin bundle -- minibrowser -o target/bundle`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    linux::main()
}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    mac::main()
}

#[cfg(target_os = "windows")]
fn main() -> anyhow::Result<()> {
    win::main()
}
