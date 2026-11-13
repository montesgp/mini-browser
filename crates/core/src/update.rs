//! The update check: which release the app asks GitHub about, which tag counts as a newer
//! version and where the Download button leads. The app only announces a newer release; it
//! never downloads or installs one.

use std::fmt;

/// The only URL the update check requests: GitHub's latest release of the app.
pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/montesgp/mini-browser/releases/latest";
/// The download page the Download button opens in the system's default browser.
pub const DOWNLOAD_PAGE_URL: &str = "https://montesgp.github.io/mini-browser/";
/// Responses larger than this are dropped unread.
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
/// How often the app checks while it runs: once a day.
pub const CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;
/// A request still unanswered after this long is cancelled and counts as a failed check.
pub const REQUEST_TIMEOUT_MS: i64 = 30 * 1000;

/// What one check learned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    /// GitHub announced this release, newer than the running version.
    Newer(Version),
    /// GitHub answered and there is nothing newer to announce.
    UpToDate,
    /// No usable answer (offline, an HTTP error, a timeout, not GitHub's JSON): retry soon.
    Failed,
}

/// The outcome of a check whose request ended with `body`, or `None` when the request failed
/// (network error, non-2xx status, timeout, too large). An answer whose `tag_name` is not a
/// valid newer tag is still an answer: retrying would not change it.
pub fn check_outcome(running: &str, body: Option<&[u8]>) -> CheckOutcome {
    let tag = body
        .and_then(|body| serde_json::from_slice::<serde_json::Value>(body).ok())
        .and_then(|release| release.get("tag_name")?.as_str().map(str::to_owned));
    match tag {
        None => CheckOutcome::Failed,
        Some(tag) => {
            newer_release(running, &tag).map_or(CheckOutcome::UpToDate, CheckOutcome::Newer)
        }
    }
}

/// How long to wait before checking again after `failures` failed checks in a row (1 or more):
/// 1 minute, then 5, then 15, then every hour until a check succeeds.
pub fn retry_delay_ms(failures: u32) -> i64 {
    const MINUTE: i64 = 60 * 1000;
    match failures {
        0 | 1 => MINUTE,
        2 => 5 * MINUTE,
        3 => 15 * MINUTE,
        _ => 60 * MINUTE,
    }
}

/// A release version `MAJOR.MINOR.PATCH`, ordered by its numbers in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    /// Parses `MAJOR.MINOR.PATCH` with decimal numbers only, such as `CARGO_PKG_VERSION`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.').map(number);
        let (Some(major), Some(minor), Some(patch), None) =
            (parts.next()?, parts.next()?, parts.next()?, parts.next())
        else {
            return None;
        };
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// Parses a release tag, which must be exactly `vMAJOR.MINOR.PATCH`.
    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::parse(tag.strip_prefix('v')?)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The version of the release `tag` when it is strictly newer than the `running` version;
/// `None` for an equal or older release and for anything that is not a valid tag or version.
pub fn newer_release(running: &str, tag: &str) -> Option<Version> {
    let running = Version::parse(running)?;
    Version::from_tag(tag).filter(|release| *release > running)
}

/// The newer version announced by a latest-release response `body` (GitHub's JSON with a
/// `tag_name`), or `None` when the body is not such JSON or the release is not newer.
pub fn newer_release_in_response(running: &str, body: &[u8]) -> Option<Version> {
    match check_outcome(running, Some(body)) {
        CheckOutcome::Newer(version) => Some(version),
        CheckOutcome::UpToDate | CheckOutcome::Failed => None,
    }
}

/// A decimal number of ASCII digits only (no sign, no spaces), or `None`.
fn number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNNING: &str = "0.5.0";

    fn version(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    #[test]
    fn newer_tags_are_updates() {
        assert_eq!(newer_release(RUNNING, "v0.5.1"), Some(version(0, 5, 1)));
        assert_eq!(newer_release(RUNNING, "v0.10.0"), Some(version(0, 10, 0)));
        assert_eq!(newer_release(RUNNING, "v1.0.0"), Some(version(1, 0, 0)));
    }

    #[test]
    fn equal_older_and_invalid_tags_are_not_updates() {
        for tag in [
            "v0.5.0",
            "v0.4.9",
            "0.6.0",
            "v0.6.0-beta",
            "v0.6",
            "",
            "v",
            "V0.6.0",
            "v0.6.0.1",
            "v0.6.0+build",
            " v0.6.0",
            "v0.6.0 ",
            "v+0.6.0",
            "v0.-6.0",
            "v0..6",
            "v0.6.x",
            "v99999999999999999999.0.0",
            "garbage",
        ] {
            assert_eq!(newer_release(RUNNING, tag), None, "{tag:?}");
        }
    }

    #[test]
    fn numbers_compare_by_major_then_minor_then_patch() {
        assert_eq!(newer_release("0.9.9", "v1.0.0"), Some(version(1, 0, 0)));
        assert_eq!(newer_release("1.0.0", "v0.99.99"), None);
        assert_eq!(newer_release("0.5.9", "v0.6.0"), Some(version(0, 6, 0)));
        assert_eq!(newer_release("0.6.0", "v0.5.9"), None);
        assert_eq!(newer_release("0.5.9", "v0.5.10"), Some(version(0, 5, 10)));
    }

    #[test]
    fn an_invalid_running_version_announces_nothing() {
        assert_eq!(newer_release("0.5.0-dev", "v0.6.0"), None);
        assert_eq!(newer_release("", "v0.6.0"), None);
    }

    #[test]
    fn the_running_version_and_tags_parse_strictly() {
        assert_eq!(Version::parse("0.4.1"), Some(version(0, 4, 1)));
        assert_eq!(Version::parse("v0.4.1"), None);
        assert_eq!(Version::from_tag("v12.0.3"), Some(version(12, 0, 3)));
        assert_eq!(Version::from_tag("12.0.3"), None);
    }

    #[test]
    fn this_build_has_a_valid_version() {
        assert!(Version::parse(env!("CARGO_PKG_VERSION")).is_some());
    }

    #[test]
    fn only_the_validated_numbers_come_out_of_a_response() {
        assert_eq!(
            newer_release_in_response(RUNNING, br#"{"tag_name":" v0.6.0"}"#),
            None
        );
        let shown = newer_release_in_response(RUNNING, br#"{"tag_name":"v0.06.00"}"#).unwrap();
        assert_eq!(shown.to_string(), "0.6.0");
    }

    #[test]
    fn a_failed_request_or_an_unusable_answer_is_a_failed_check() {
        assert_eq!(check_outcome(RUNNING, None), CheckOutcome::Failed);
        for body in [
            b"not json".as_slice(),
            b"",
            b"\xff\xfe",
            br#"{"tag_name":"v0.6.0""#,
            br#"["v0.6.0"]"#,
            br#"{"name":"v0.6.0"}"#,
            br#"{"tag_name":6}"#,
            br#"{"message":"API rate limit exceeded"}"#,
        ] {
            assert_eq!(
                check_outcome(RUNNING, Some(body)),
                CheckOutcome::Failed,
                "{}",
                String::from_utf8_lossy(body)
            );
        }
    }

    #[test]
    fn an_answer_is_newer_or_up_to_date() {
        assert_eq!(
            check_outcome(RUNNING, Some(br#"{"tag_name":"v0.6.0"}"#)),
            CheckOutcome::Newer(version(0, 6, 0))
        );
        for body in [
            br#"{"tag_name":"v0.5.0"}"#.as_slice(),
            br#"{"tag_name":"v0.4.9"}"#,
            br#"{"tag_name":"v0.6.0-rc.1"}"#,
            br#"{"tag_name":"garbage"}"#,
        ] {
            assert_eq!(
                check_outcome(RUNNING, Some(body)),
                CheckOutcome::UpToDate,
                "{}",
                String::from_utf8_lossy(body)
            );
        }
        assert_eq!(
            check_outcome("0.5.0-dev", Some(br#"{"tag_name":"v0.6.0"}"#)),
            CheckOutcome::UpToDate
        );
    }

    #[test]
    fn retries_wait_1_5_15_minutes_then_every_hour() {
        const MINUTE: i64 = 60 * 1000;
        assert_eq!(retry_delay_ms(1), MINUTE);
        assert_eq!(retry_delay_ms(2), 5 * MINUTE);
        assert_eq!(retry_delay_ms(3), 15 * MINUTE);
        assert_eq!(retry_delay_ms(4), 60 * MINUTE);
        assert_eq!(retry_delay_ms(50), 60 * MINUTE);
        assert_eq!(retry_delay_ms(u32::MAX), 60 * MINUTE);
        assert_eq!(retry_delay_ms(0), MINUTE);
        assert_eq!(REQUEST_TIMEOUT_MS, 30_000);
    }

    #[test]
    fn versions_display_without_the_v() {
        assert_eq!(version(0, 10, 2).to_string(), "0.10.2");
    }

    #[test]
    fn the_response_tag_name_decides() {
        let body = br#"{"tag_name":"v0.6.0","name":"<b>x</b>","html_url":"https://evil.example/"}"#;
        assert_eq!(
            newer_release_in_response(RUNNING, body),
            Some(version(0, 6, 0))
        );
        for body in [
            br#"{"tag_name":"v0.5.0"}"#.as_slice(),
            br#"{"tag_name":"v0.6.0-rc.1"}"#,
            br#"{"tag_name":6}"#,
            br#"{"name":"v0.6.0"}"#,
            br#"["v0.6.0"]"#,
            br#"{"tag_name":"v0.6.0""#,
            b"not json",
            b"",
            b"\xff\xfe",
        ] {
            assert_eq!(
                newer_release_in_response(RUNNING, body),
                None,
                "{}",
                String::from_utf8_lossy(body)
            );
        }
    }

    #[test]
    fn the_urls_are_the_fixed_github_ones() {
        assert_eq!(
            LATEST_RELEASE_URL,
            "https://api.github.com/repos/montesgp/mini-browser/releases/latest"
        );
        assert_eq!(
            DOWNLOAD_PAGE_URL,
            "https://montesgp.github.io/mini-browser/"
        );
        assert_eq!(MAX_RESPONSE_BYTES, 1_048_576);
        assert_eq!(CHECK_INTERVAL_MS, 86_400_000);
    }
}
