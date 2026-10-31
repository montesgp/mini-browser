//! The app data directory: holds the browser profile (CEF cache) and the settings file.

use std::{env, path::PathBuf};

/// Name of the app's directory inside the OS app data directory.
pub const APP_DIR_NAME: &str = "minibrowser";

/// `%LOCALAPPDATA%\minibrowser` on Windows, `~/Library/Application Support/minibrowser` on
/// macOS and `$XDG_DATA_HOME/minibrowser` (default `~/.local/share/minibrowser`) elsewhere.
/// Falls back to the temp directory when the OS directory cannot be determined.
pub fn app_data_dir() -> PathBuf {
    os_data_dir()
        .unwrap_or_else(env::temp_dir)
        .join(APP_DIR_NAME)
}

/// Reads an environment variable holding an absolute path.
fn absolute_path_var(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

#[cfg(target_os = "windows")]
fn os_data_dir() -> Option<PathBuf> {
    absolute_path_var("LOCALAPPDATA")
}

#[cfg(target_os = "macos")]
fn os_data_dir() -> Option<PathBuf> {
    absolute_path_var("HOME").map(|home| home.join("Library").join("Application Support"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn os_data_dir() -> Option<PathBuf> {
    absolute_path_var("XDG_DATA_HOME")
        .or_else(|| absolute_path_var("HOME").map(|home| home.join(".local").join("share")))
}
