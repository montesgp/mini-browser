//! The app data directory: holds the browser profile (CEF cache) and the settings file.

use std::{env, ffi::OsString, path::PathBuf};

/// Name of the app's directory inside the OS app data directory.
pub const APP_DIR_NAME: &str = "minibrowser";

/// Printed when [`app_data_dir`] finds no directory and the app refuses to start.
#[cfg(target_os = "windows")]
pub const MISSING_DIR_ERROR: &str =
    "minibrowser: cannot find the app data directory: set LOCALAPPDATA to an absolute path";
#[cfg(target_os = "macos")]
pub const MISSING_DIR_ERROR: &str =
    "minibrowser: cannot find the app data directory: set HOME to an absolute path";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const MISSING_DIR_ERROR: &str = "minibrowser: cannot find the app data directory: set \
     XDG_DATA_HOME or HOME to an absolute path";

/// `%LOCALAPPDATA%\minibrowser` on Windows, `~/Library/Application Support/minibrowser` on
/// macOS and `$XDG_DATA_HOME/minibrowser` (default `~/.local/share/minibrowser`) elsewhere.
/// `None` when the OS directory cannot be determined: the app then refuses to start rather than
/// share a profile in the temp directory.
pub fn app_data_dir() -> Option<PathBuf> {
    resolve(|name| env::var_os(name))
}

/// [`app_data_dir`] with the environment lookup passed in.
fn resolve(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let absolute = |name: &str| {
        var(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    os_data_dir(absolute).map(|dir| dir.join(APP_DIR_NAME))
}

#[cfg(target_os = "windows")]
fn os_data_dir(absolute: impl Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    absolute("LOCALAPPDATA")
}

#[cfg(target_os = "macos")]
fn os_data_dir(absolute: impl Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    absolute("HOME").map(|home| home.join("Library").join("Application Support"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn os_data_dir(absolute: impl Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    absolute("XDG_DATA_HOME")
        .or_else(|| absolute("HOME").map(|home| home.join(".local").join("share")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An environment holding only `vars`.
    fn env_of(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let vars: Vec<(String, OsString)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |name| {
            vars.iter()
                .find(|(var, _)| var == name)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn no_os_directory_means_no_data_dir() {
        assert_eq!(resolve(env_of(&[])), None);
        assert_eq!(
            resolve(env_of(&[("TEMP", "/tmp"), ("TMPDIR", "/tmp")])),
            None
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_uses_an_absolute_localappdata() {
        assert_eq!(
            resolve(env_of(&[("LOCALAPPDATA", r"C:\Users\a\AppData\Local")])),
            Some(PathBuf::from(r"C:\Users\a\AppData\Local\minibrowser"))
        );
        assert_eq!(resolve(env_of(&[("LOCALAPPDATA", r"relative\dir")])), None);
        assert_eq!(resolve(env_of(&[("LOCALAPPDATA", "")])), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_uses_application_support_under_home() {
        assert_eq!(
            resolve(env_of(&[("HOME", "/Users/a")])),
            Some(PathBuf::from(
                "/Users/a/Library/Application Support/minibrowser"
            ))
        );
        assert_eq!(resolve(env_of(&[("HOME", "relative")])), None);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    #[test]
    fn linux_prefers_xdg_data_home_then_home() {
        assert_eq!(
            resolve(env_of(&[("XDG_DATA_HOME", "/data"), ("HOME", "/home/a")])),
            Some(PathBuf::from("/data/minibrowser"))
        );
        assert_eq!(
            resolve(env_of(&[
                ("XDG_DATA_HOME", "relative"),
                ("HOME", "/home/a")
            ])),
            Some(PathBuf::from("/home/a/.local/share/minibrowser"))
        );
        assert_eq!(resolve(env_of(&[("XDG_DATA_HOME", "relative")])), None);
    }
}
