//! User settings persisted as JSON in the app data directory.

use std::{fs, io, path::Path};

use serde::{Deserialize, Deserializer, Serialize, de::Error};

use crate::address::{DEFAULT_SEARCH_TEMPLATE, QUERY_PLACEHOLDER};

pub const PADDING_MIN: u32 = 4;
pub const PADDING_MAX: u32 = 32;
pub const PADDING_DEFAULT: u32 = 8;
/// Pixels added or removed by one press of the settings box `+` / `-` buttons.
pub const PADDING_STEP: u32 = 2;

/// File name of the settings file inside the app data directory.
pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Width in pixels of the dark frame around the page; dragging it moves the window.
    #[serde(deserialize_with = "clamped_padding")]
    pub padding: u32,
    /// Search URL with a `{query}` placeholder, used for address input that is not a URL.
    pub search_template: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            padding: PADDING_DEFAULT,
            search_template: DEFAULT_SEARCH_TEMPLATE.to_owned(),
        }
    }
}

impl Settings {
    /// Parses settings JSON. Corrupt JSON yields the defaults; an out-of-range padding is
    /// clamped and an invalid search template is replaced by the default one.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str::<Self>(text)
            .map(Self::sanitized)
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("settings always serialize")
    }

    /// Returns a copy with padding moved by `steps` presses of [`PADDING_STEP`], clamped.
    pub fn with_padding_steps(&self, steps: i32) -> Self {
        let delta = i64::from(steps) * i64::from(PADDING_STEP);
        let padding =
            (i64::from(self.padding) + delta).clamp(i64::from(PADDING_MIN), i64::from(PADDING_MAX));
        Self {
            padding: padding as u32,
            ..self.clone()
        }
    }

    /// Loads settings from `path`; a missing or unreadable file yields the defaults.
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .map(|text| Self::from_json(&text))
            .unwrap_or_default()
    }

    /// Writes settings to `path` atomically (temporary file, then rename), creating parent
    /// directories as needed.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, self.to_json())?;
        fs::rename(&temporary, path)
    }

    fn sanitized(self) -> Self {
        let search_template = if is_valid_search_template(&self.search_template) {
            self.search_template
        } else {
            DEFAULT_SEARCH_TEMPLATE.to_owned()
        };
        Self {
            padding: self.padding.clamp(PADDING_MIN, PADDING_MAX),
            search_template,
        }
    }
}

/// Reads any JSON integer (negative or beyond `u32`) as a padding clamped to the valid range.
/// Other values, including fractions and integers too large for JSON parsers to keep exact,
/// are an error, so the whole file counts as corrupt.
fn clamped_padding<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    let padding = if let Some(value) = number.as_i64() {
        value.clamp(i64::from(PADDING_MIN), i64::from(PADDING_MAX))
    } else if number.as_u64().is_some() {
        i64::from(PADDING_MAX)
    } else {
        return Err(D::Error::custom("padding must be an integer"));
    };
    Ok(padding as u32)
}

/// A search template must be an http(s) URL with a `{query}` placeholder.
fn is_valid_search_template(template: &str) -> bool {
    let lower = template.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && template.contains(QUERY_PLACEHOLDER)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("mini-core-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn defaults_are_8px_padding_and_google() {
        let settings = Settings::default();
        assert_eq!(settings.padding, 8);
        assert_eq!(
            settings.search_template,
            "https://www.google.com/search?q={query}"
        );
    }

    #[test]
    fn valid_json_round_trips() {
        let settings = Settings {
            padding: 12,
            search_template: "https://duckduckgo.com/?q={query}".into(),
        };
        let json = settings.to_json();
        assert!(json.contains("\"padding\": 12"), "{json}");
        assert!(json.contains("\"searchTemplate\""), "{json}");
        assert_eq!(Settings::from_json(&json), settings);
    }

    #[test]
    fn corrupt_json_falls_back_to_defaults() {
        assert_eq!(Settings::from_json("{not json"), Settings::default());
        assert_eq!(Settings::from_json(""), Settings::default());
        assert_eq!(
            Settings::from_json(r#"{"padding":"wide"}"#),
            Settings::default()
        );
    }

    #[test]
    fn missing_fields_take_defaults_and_unknown_fields_are_ignored() {
        let settings = Settings::from_json(r#"{"padding":10,"theme":"light"}"#);
        assert_eq!(settings.padding, 10);
        assert_eq!(settings.search_template, DEFAULT_SEARCH_TEMPLATE);
    }

    #[test]
    fn out_of_range_padding_is_clamped() {
        assert_eq!(Settings::from_json(r#"{"padding":0}"#).padding, 4);
        assert_eq!(Settings::from_json(r#"{"padding":3}"#).padding, 4);
        assert_eq!(Settings::from_json(r#"{"padding":4}"#).padding, 4);
        assert_eq!(Settings::from_json(r#"{"padding":32}"#).padding, 32);
        assert_eq!(Settings::from_json(r#"{"padding":33}"#).padding, 32);
        assert_eq!(Settings::from_json(r#"{"padding":-5}"#).padding, 4);
        assert_eq!(
            Settings::from_json(r#"{"padding":-9223372036854775808}"#).padding,
            4
        );
        assert_eq!(
            Settings::from_json(r#"{"padding":18446744073709551615}"#).padding,
            32
        );
    }

    #[test]
    fn negative_padding_is_clamped_and_keeps_the_other_fields() {
        let settings = Settings::from_json(
            r#"{"padding":-5,"searchTemplate":"https://duckduckgo.com/?q={query}"}"#,
        );
        assert_eq!(settings.padding, 4);
        assert_eq!(
            settings.search_template,
            "https://duckduckgo.com/?q={query}"
        );
    }

    #[test]
    fn non_integer_padding_is_corrupt() {
        for padding in ["10.5", "1e3", "99999999999999999999999", "null"] {
            let json = format!(
                r#"{{"padding":{padding},"searchTemplate":"https://duckduckgo.com/?q={{query}}"}}"#
            );
            assert_eq!(Settings::from_json(&json), Settings::default(), "{padding}");
        }
    }

    #[test]
    fn invalid_search_templates_fall_back_to_the_default() {
        for template in [
            "https://example.com/?q=",
            "javascript:alert('{query}')",
            "file:///{query}",
            "",
        ] {
            let json = serde_json::json!({ "searchTemplate": template }).to_string();
            assert_eq!(
                Settings::from_json(&json).search_template,
                DEFAULT_SEARCH_TEMPLATE,
                "{template}"
            );
        }
    }

    #[test]
    fn padding_steps_move_by_2px_within_range() {
        let settings = Settings::default();
        assert_eq!(settings.with_padding_steps(1).padding, 10);
        assert_eq!(settings.with_padding_steps(-1).padding, 6);
        assert_eq!(settings.with_padding_steps(-10).padding, 4);
        assert_eq!(settings.with_padding_steps(100).padding, 32);
        assert_eq!(settings.padding, 8, "the original is unchanged");
    }

    #[test]
    fn load_of_a_missing_file_yields_defaults() {
        let dir = scratch_dir("missing");
        assert_eq!(
            Settings::load(&dir.join(SETTINGS_FILE)),
            Settings::default()
        );
    }

    #[test]
    fn save_then_load_round_trips_and_creates_directories() {
        let dir = scratch_dir("save");
        let path = dir.join("nested").join(SETTINGS_FILE);
        let settings = Settings::default().with_padding_steps(3);
        settings.save(&path).expect("save settings");
        assert_eq!(Settings::load(&path), settings);
        let leftovers: Vec<String> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(leftovers, vec![SETTINGS_FILE], "no temporary file is left");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn load_of_a_corrupt_file_yields_defaults_and_leaves_it_untouched() {
        let dir = scratch_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(SETTINGS_FILE);
        fs::write(&path, "{oops").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{oops");
        fs::remove_dir_all(dir).unwrap();
    }
}
