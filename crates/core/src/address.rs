//! Turns what the user types in the address box into the URL to load.

use std::net::{Ipv4Addr, Ipv6Addr};

/// Placeholder that a search template replaces with the URL-encoded query.
pub const QUERY_PLACEHOLDER: &str = "{query}";

/// Default search engine template.
pub const DEFAULT_SEARCH_TEMPLATE: &str = "https://www.google.com/search?q={query}";

/// Resolves address-box input into a URL to load, or `None` when there is nothing to load.
///
/// - `http://` and `https://` input is loaded as typed.
/// - Host-like input gets `https://`; `localhost` and IP addresses get `http://`.
/// - Anything else, including any other scheme, becomes a search through `search_template`.
pub fn resolve_input(input: &str, search_template: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if has_web_scheme(input) {
        return Some(input.to_owned());
    }
    match host_kind(input) {
        Some(HostKind::Local) => Some(format!("http://{input}")),
        Some(HostKind::Domain) => Some(format!("https://{input}")),
        None => Some(search_url(input, search_template)),
    }
}

/// Builds the search URL for `query` from a template containing [`QUERY_PLACEHOLDER`].
pub fn search_url(query: &str, search_template: &str) -> String {
    let encoded: String = form_urlencoded::byte_serialize(query.as_bytes()).collect();
    search_template.replace(QUERY_PLACEHOLDER, &encoded)
}

fn has_web_scheme(input: &str) -> bool {
    let lower = input.get(..8).unwrap_or(input).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

enum HostKind {
    /// `localhost` or an IP address: served over plain HTTP by default.
    Local,
    /// A public domain name.
    Domain,
}

/// Classifies scheme-less input whose authority (before any `/`, `?` or `#`) is a host with an
/// optional port. Input with whitespace or user info (`@`) is never treated as a host.
fn host_kind(input: &str) -> Option<HostKind> {
    if input.contains(char::is_whitespace) || input.contains('@') {
        return None;
    }
    let authority = input.split(['/', '?', '#']).next().unwrap_or_default();

    if let Some(rest) = authority.strip_prefix('[') {
        let (ip, port) = rest.split_once(']')?;
        let port_ok = port.is_empty() || port.strip_prefix(':').is_some_and(is_port);
        return (port_ok && ip.parse::<Ipv6Addr>().is_ok()).then_some(HostKind::Local);
    }

    let host = match authority.rsplit_once(':') {
        Some((host, port)) if is_port(port) => host,
        Some(_) => return None,
        None => authority,
    };
    let host = host.to_ascii_lowercase();
    if host == "localhost" || host.parse::<Ipv4Addr>().is_ok() {
        Some(HostKind::Local)
    } else if is_domain(&host) {
        Some(HostKind::Domain)
    } else {
        None
    }
}

fn is_port(port: &str) -> bool {
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u16>().is_ok()
}

/// At least two dot-separated labels of letters, digits and inner hyphens, ending in an
/// alphabetic (or punycode) top-level domain, so `1.5` and `v1.2.3` stay searches.
fn is_domain(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    let valid_label = |label: &&str| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    let tld = labels.last().copied().unwrap_or_default();
    let valid_tld =
        tld.starts_with("xn--") || (tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()));
    labels.len() >= 2 && labels.iter().all(valid_label) && valid_tld
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(input: &str) -> Option<String> {
        resolve_input(input, DEFAULT_SEARCH_TEMPLATE)
    }

    fn search(encoded: &str) -> Option<String> {
        Some(format!("https://www.google.com/search?q={encoded}"))
    }

    #[test]
    fn empty_or_whitespace_input_loads_nothing() {
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   \t "), None);
    }

    #[test]
    fn http_and_https_urls_load_as_typed() {
        assert_eq!(
            resolve("https://rauchg.com"),
            Some("https://rauchg.com".into())
        );
        assert_eq!(
            resolve("  http://example.com/a?b=c#d "),
            Some("http://example.com/a?b=c#d".into())
        );
        assert_eq!(
            resolve("HTTPS://Example.com"),
            Some("HTTPS://Example.com".into())
        );
    }

    #[test]
    fn host_like_input_gets_https() {
        assert_eq!(resolve("rauchg.com"), Some("https://rauchg.com".into()));
        assert_eq!(
            resolve("vercel.com/home"),
            Some("https://vercel.com/home".into())
        );
        assert_eq!(
            resolve("docs.rs:443/cef?x=1"),
            Some("https://docs.rs:443/cef?x=1".into())
        );
        assert_eq!(
            resolve("xn--bcher-kva.example"),
            Some("https://xn--bcher-kva.example".into())
        );
    }

    #[test]
    fn localhost_and_ip_addresses_get_http() {
        assert_eq!(resolve("localhost"), Some("http://localhost".into()));
        assert_eq!(
            resolve("localhost:3000/api"),
            Some("http://localhost:3000/api".into())
        );
        assert_eq!(
            resolve("127.0.0.1:8080/x"),
            Some("http://127.0.0.1:8080/x".into())
        );
        assert_eq!(resolve("192.168.1.10"), Some("http://192.168.1.10".into()));
        assert_eq!(resolve("[::1]:3000"), Some("http://[::1]:3000".into()));
    }

    #[test]
    fn plain_words_become_a_search() {
        assert_eq!(resolve("rauch"), search("rauch"));
        assert_eq!(resolve("how to rust"), search("how+to+rust"));
        assert_eq!(resolve("foo.com bar"), search("foo.com+bar"));
    }

    #[test]
    fn inputs_that_only_look_like_hosts_become_a_search() {
        assert_eq!(resolve("1.5"), search("1.5"));
        assert_eq!(resolve("v1.2.3"), search("v1.2.3"));
        assert_eq!(resolve("user@example.com"), search("user%40example.com"));
        assert_eq!(resolve("localhost:abc"), search("localhost%3Aabc"));
        assert_eq!(resolve("a..com"), search("a..com"));
    }

    #[test]
    fn other_schemes_become_a_search_and_never_load() {
        assert_eq!(
            resolve("javascript:alert(1)"),
            search("javascript%3Aalert%281%29")
        );
        assert_eq!(
            resolve("file:///etc/passwd"),
            search("file%3A%2F%2F%2Fetc%2Fpasswd")
        );
        assert_eq!(
            resolve("data:text/html,hi"),
            search("data%3Atext%2Fhtml%2Chi")
        );
        assert_eq!(
            resolve("chrome://settings"),
            search("chrome%3A%2F%2Fsettings")
        );
    }

    #[test]
    fn search_uses_the_given_template() {
        assert_eq!(
            resolve_input("rust cef", "https://duckduckgo.com/?q={query}"),
            Some("https://duckduckgo.com/?q=rust+cef".into())
        );
    }
}
