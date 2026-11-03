//! What a launch asks for: the `--url` input of its command line and, for a second launch while
//! the app is running, what the running window does with it.

use crate::address::resolve_input;

/// What the running window does when the app is launched again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relaunch {
    /// Only bring the window to the front.
    Focus,
    /// Bring the window to the front and open this URL in a new tab.
    OpenTab(String),
}

/// The `--url` input of a command line. `switch` is the value of the `--url` switch, `None`
/// without one; `arguments` are the command line's plain arguments. `--url=<input>` gives the
/// input directly; `--url <input>` gives an empty switch value, so the first plain argument is
/// the input. Empty when there is no input.
pub fn url_input(switch: Option<&str>, arguments: &[String]) -> String {
    match switch {
        Some("") => arguments.first().cloned().unwrap_or_default(),
        Some(value) => value.to_owned(),
        None => String::new(),
    }
}

/// What a second launch with `url_input` does, resolved with the same rules as the first launch
/// and the address box: only web addresses load, anything else becomes a search, and empty input
/// only brings the window to the front.
pub fn relaunch(url_input: &str, search_template: &str) -> Relaunch {
    match resolve_input(url_input, search_template) {
        Some(url) => Relaunch::OpenTab(url),
        None => Relaunch::Focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::DEFAULT_SEARCH_TEMPLATE;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn url_with_equals_sign_is_the_input() {
        // First-launch behavior before this module: the switch value, as `--url=<input>`.
        assert_eq!(
            url_input(Some("https://github.com"), &[]),
            "https://github.com"
        );
    }

    #[test]
    fn url_with_equals_sign_ignores_plain_arguments() {
        assert_eq!(
            url_input(Some("github.com"), &args(&["other"])),
            "github.com"
        );
    }

    #[test]
    fn url_followed_by_its_value_takes_the_first_plain_argument() {
        assert_eq!(
            url_input(Some(""), &args(&["https://example.org", "second"])),
            "https://example.org"
        );
    }

    #[test]
    fn url_without_any_value_is_empty() {
        assert_eq!(url_input(Some(""), &[]), "");
    }

    #[test]
    fn no_url_switch_is_empty_even_with_plain_arguments() {
        assert_eq!(url_input(None, &args(&["https://example.org"])), "");
    }

    #[test]
    fn relaunch_without_input_only_focuses() {
        assert_eq!(relaunch("", DEFAULT_SEARCH_TEMPLATE), Relaunch::Focus);
        assert_eq!(relaunch("   ", DEFAULT_SEARCH_TEMPLATE), Relaunch::Focus);
    }

    #[test]
    fn relaunch_with_a_web_address_opens_it_in_a_tab() {
        assert_eq!(
            relaunch("https://example.org", DEFAULT_SEARCH_TEMPLATE),
            Relaunch::OpenTab("https://example.org".to_owned())
        );
        assert_eq!(
            relaunch("example.org", DEFAULT_SEARCH_TEMPLATE),
            Relaunch::OpenTab("https://example.org".to_owned())
        );
    }

    #[test]
    fn relaunch_with_another_scheme_opens_a_search() {
        assert_eq!(
            relaunch("javascript:alert(1)", DEFAULT_SEARCH_TEMPLATE),
            Relaunch::OpenTab(
                "https://www.google.com/search?q=javascript%3Aalert%281%29".to_owned()
            )
        );
        assert_eq!(
            relaunch("file:///etc/passwd", DEFAULT_SEARCH_TEMPLATE),
            Relaunch::OpenTab(
                "https://www.google.com/search?q=file%3A%2F%2F%2Fetc%2Fpasswd".to_owned()
            )
        );
    }

    #[test]
    fn relaunch_uses_the_configured_search_template() {
        assert_eq!(
            relaunch("rust cef", "https://duckduckgo.com/?q={query}"),
            Relaunch::OpenTab("https://duckduckgo.com/?q=rust+cef".to_owned())
        );
    }
}
