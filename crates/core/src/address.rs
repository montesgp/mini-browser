//! Turns what the user types in the address box into the URL to load.

use std::net::{Ipv4Addr, Ipv6Addr};

/// Placeholder that a search template replaces with the URL-encoded query.
pub const QUERY_PLACEHOLDER: &str = "{query}";

/// Default search engine template.
pub const DEFAULT_SEARCH_TEMPLATE: &str = "https://www.google.com/search?q={query}";

/// How the platform writes local paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStyle {
    /// Windows: drive paths, `C:\Users\…` or `C:/Users/…`.
    Windows,
    /// macOS and Linux: absolute paths, `/home/…`, and `~/…` in the user's home.
    Unix,
}

impl PathStyle {
    /// The path style of the platform the app is built for.
    pub const CURRENT: Self = if cfg!(windows) {
        Self::Windows
    } else {
        Self::Unix
    };
}

/// What typed local paths resolve against: the platform's path style and the user's home
/// directory, which `~/…` expands to on macOS and Linux.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFiles {
    pub style: PathStyle,
    /// The user's home directory; without one, `~/…` is a search.
    pub home: Option<String>,
}

/// Resolves address-box input into a URL to load, or `None` when there is nothing to load.
///
/// - `http://` and `https://` input is loaded as typed.
/// - Local files load as `file:///` URLs: `file:` URLs with an empty or `localhost` host, and
///   absolute paths in the style of `files`. Files on other machines (UNC paths,
///   `file://host/…`) are searches.
/// - Host-like input gets `https://`; `localhost` and IP addresses get `http://`.
/// - Anything else, including any other scheme, becomes a search through `search_template`.
pub fn resolve_input(input: &str, search_template: &str, files: &LocalFiles) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if has_web_scheme(input) {
        return Some(input.to_owned());
    }
    if let Some(url) = local_file_url(input, files) {
        return Some(url);
    }
    match host_kind(input) {
        Some(HostKind::Local) => Some(format!("http://{input}")),
        Some(HostKind::Domain) => Some(format!("https://{input}")),
        None => Some(search_url(input, search_template)),
    }
}

/// Whether `url` is a `file:` URL.
pub fn is_file_url(url: &str) -> bool {
    strip_prefix_ignore_case(url, "file:").is_some()
}

/// Whether `url` is a `file:` URL of a file on another machine: its host is neither empty nor
/// `localhost`, or its path starts with `//` (`file:////server/share` is a UNC path on Windows).
/// On Windows, Chromium opens such files over the network, which can send the user's
/// credentials to that host.
pub fn is_remote_file_url(url: &str) -> bool {
    let Some(rest) = strip_prefix_ignore_case(url, "file:") else {
        return false;
    };
    // Encoded slashes and backslashes (`%2F`, `%5C`) count too: a page can write
    // `file:///%5C%5Cserver/share`, which some layers decode into a UNC path.
    let rest = rest
        .replace("%5C", "/")
        .replace("%5c", "/")
        .replace("%2F", "/")
        .replace("%2f", "/")
        .replace('\\', "/");
    let Some(authority) = rest.strip_prefix("//") else {
        return false;
    };
    let end = authority.find(['/', '?', '#']).unwrap_or(authority.len());
    let (host, path) = authority.split_at(end);
    !(host.is_empty() || host.eq_ignore_ascii_case("localhost")) || path.starts_with("//")
}

/// Builds the search URL for `query` from a template containing [`QUERY_PLACEHOLDER`].
pub fn search_url(query: &str, search_template: &str) -> String {
    let encoded: String = form_urlencoded::byte_serialize(query.as_bytes()).collect();
    search_template.replace(QUERY_PLACEHOLDER, &encoded)
}

/// The `file:///` URL of typed input naming a local file, or `None` when it names none: a
/// `file:` URL with an empty or `localhost` host, or an absolute path in the style of `files`.
fn local_file_url(input: &str, files: &LocalFiles) -> Option<String> {
    if let Some(rest) = strip_prefix_ignore_case(input, "file:") {
        return file_url_path(rest, files.style);
    }
    match files.style {
        PathStyle::Windows => windows_path(input).map(|path| file_url(&path, "")),
        PathStyle::Unix => unix_path(input, files.home.as_deref()).map(|path| file_url(&path, "")),
    }
}

/// Resolves the part of a `file:` URL after the scheme. Like Chromium, backslashes before the
/// query or fragment count as slashes. The path is decoded and encoded again, so the URL is in
/// canonical form and a path that decodes to another machine's (`%5C%5Cserver`) is refused.
fn file_url_path(rest: &str, style: PathStyle) -> Option<String> {
    let (location, suffix) = rest.split_at(rest.find(['?', '#']).unwrap_or(rest.len()));
    let location = location.replace('\\', "/");
    let authority = location.strip_prefix("//")?;
    let (host, path) = authority.split_at(authority.find('/')?);
    if !(host.is_empty() || host.eq_ignore_ascii_case("localhost")) {
        return None;
    }
    let path = percent_decode(path)?;
    let path = match style {
        PathStyle::Windows => windows_path(path.strip_prefix('/')?)?,
        PathStyle::Unix => unix_path(&path, None)?,
    };
    Some(file_url(&path, suffix))
}

/// A Windows drive path (`C:\x` or `C:/x`) with forward slashes and an uppercase drive letter.
fn windows_path(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let is_drive_path = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    is_drive_path.then(|| {
        let path = input.replace('\\', "/");
        format!("{}{}", path[..1].to_ascii_uppercase(), &path[1..])
    })
}

/// An absolute macOS or Linux path, with `~` and `~/…` expanded to `home` when given. A path
/// starting with `//` is refused: POSIX leaves its meaning to the system.
fn unix_path(input: &str, home: Option<&str>) -> Option<String> {
    let path = match input.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            format!("{}{rest}", home?.trim_end_matches('/'))
        }
        Some(_) => return None,
        None => input.to_owned(),
    };
    (path.starts_with('/') && !path.starts_with("//")).then_some(path)
}

/// The `file:///` URL of an absolute `path` with forward slashes, percent-encoded, followed by
/// `suffix` (a query or fragment typed in a `file:` URL).
fn file_url(path: &str, suffix: &str) -> String {
    let path = path.strip_prefix('/').unwrap_or(path);
    let mut url = String::from("file:///");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@/".contains(&byte) {
            url.push(char::from(byte));
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url + suffix
}

/// Decodes `%XX` escapes; a `%` without two hex digits stays as it is. `None` when the result is
/// not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                i += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
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
/// optional port. Input with whitespace or with user info (`@` in the authority) is never
/// treated as a host.
fn host_kind(input: &str) -> Option<HostKind> {
    if input.contains(char::is_whitespace) {
        return None;
    }
    let authority = input.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return None;
    }

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

    #[test]
    fn encoded_unc_file_urls_are_remote() {
        for url in [
            "file:///%5C%5Cserver/x",
            "file:///%2F%2Fserver/x",
            "file:///%5c%5cserver",
            "file://%2F%2Fserver/x",
        ] {
            assert!(is_remote_file_url(url), "{url}");
        }
        assert!(!is_remote_file_url("file:///C:/Users/me/page%20one.html"));
        assert!(!is_remote_file_url("file:///C:/a%2Fb.html"));
    }

    #[test]
    fn more_files_on_other_machines_become_a_search() {
        for input in [
            r"\.\UNC\server\share",
            r"\?\C:\x",
            r"\/server",
            "file://[::1]/C:/x",
            "file://localhost./C:/x",
            "file://localhost//server/share",
            "C:\u{FF3C}x",
            "file:\u{FF0F}\u{FF0F}server/x",
        ] {
            assert_search(input, &windows());
        }
    }

    fn windows() -> LocalFiles {
        LocalFiles {
            style: PathStyle::Windows,
            home: None,
        }
    }

    fn unix() -> LocalFiles {
        LocalFiles {
            style: PathStyle::Unix,
            home: Some("/home/ana".into()),
        }
    }

    fn resolve(input: &str) -> Option<String> {
        resolve_on(input, &windows())
    }

    fn resolve_on(input: &str, files: &LocalFiles) -> Option<String> {
        resolve_input(input, DEFAULT_SEARCH_TEMPLATE, files)
    }

    fn assert_search(input: &str, files: &LocalFiles) {
        let url = resolve_on(input, files).unwrap();
        assert!(
            url.starts_with("https://www.google.com/search?q="),
            "{input} -> {url}"
        );
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
            resolve("https://github.com"),
            Some("https://github.com".into())
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
        assert_eq!(resolve("github.com"), Some("https://github.com".into()));
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
        assert_eq!(resolve("github"), search("github"));
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
    fn user_info_in_the_authority_becomes_a_search() {
        assert_eq!(resolve("user@example.com"), search("user%40example.com"));
        assert_eq!(
            resolve("user@example.com/path"),
            search("user%40example.com%2Fpath")
        );
    }

    #[test]
    fn an_at_sign_after_the_authority_keeps_the_input_a_host() {
        assert_eq!(
            resolve("medium.com/@user"),
            Some("https://medium.com/@user".into())
        );
        assert_eq!(
            resolve("example.com?by=@me"),
            Some("https://example.com?by=@me".into())
        );
        assert_eq!(
            resolve("localhost:3000/#@x"),
            Some("http://localhost:3000/#@x".into())
        );
    }

    #[test]
    fn other_schemes_become_a_search_and_never_load() {
        assert_eq!(
            resolve("javascript:alert(1)"),
            search("javascript%3Aalert%281%29")
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
            resolve_input("rust cef", "https://duckduckgo.com/?q={query}", &windows()),
            Some("https://duckduckgo.com/?q=rust+cef".into())
        );
    }

    #[test]
    fn more_non_web_schemes_never_load() {
        for input in [
            "JavaScript:alert(1)",
            "about:blank",
            "view-source:https://a.com",
            "blob:https://a.com/x",
            "ftp://a.com",
            "http:/a.com",
            "mini://home",
            "mini://home/",
            "mini://home/cmd/quit",
            "MINI://HOME/",
            "mini:home",
        ] {
            assert_search(input, &windows());
            assert_search(input, &unix());
        }
    }

    #[test]
    fn windows_drive_paths_load_as_file_urls() {
        assert_eq!(
            resolve(r"C:\Users\ana\Downloads\page.html"),
            Some("file:///C:/Users/ana/Downloads/page.html".into())
        );
        assert_eq!(
            resolve("C:/Users/ana/page.html"),
            Some("file:///C:/Users/ana/page.html".into())
        );
        assert_eq!(
            resolve(r"d:\notes.pdf"),
            Some("file:///D:/notes.pdf".into())
        );
        assert_eq!(resolve(r"  E:\  "), Some("file:///E:/".into()));
    }

    #[test]
    fn paths_are_percent_encoded() {
        assert_eq!(
            resolve(r"C:\My Files\page one.html"),
            Some("file:///C:/My%20Files/page%20one.html".into())
        );
        assert_eq!(
            resolve(r"C:\x\café#1?.html"),
            Some("file:///C:/x/caf%C3%A9%231%3F.html".into())
        );
        assert_eq!(
            resolve(r"C:\100%\a&b (1).html"),
            Some("file:///C:/100%25/a&b%20(1).html".into())
        );
        assert_eq!(
            resolve_on("/tmp/page one#2.html", &unix()),
            Some("file:///tmp/page%20one%232.html".into())
        );
    }

    #[test]
    fn local_file_urls_load_normalized() {
        assert_eq!(
            resolve("file:///C:/Users/ana/Downloads/ruido.html"),
            Some("file:///C:/Users/ana/Downloads/ruido.html".into())
        );
        assert_eq!(
            resolve(r" FILE:///c:\x\a b.html"),
            Some("file:///C:/x/a%20b.html".into())
        );
        assert_eq!(
            resolve("file://localhost/C:/x.html"),
            Some("file:///C:/x.html".into())
        );
        assert_eq!(
            resolve("file:///C:/My%20Files/caf%C3%A9.html"),
            Some("file:///C:/My%20Files/caf%C3%A9.html".into())
        );
        assert_eq!(
            resolve("file:///C:/doc.html#part-2"),
            Some("file:///C:/doc.html#part-2".into())
        );
        assert_eq!(
            resolve_on("file:///etc/hosts", &unix()),
            Some("file:///etc/hosts".into())
        );
        assert_eq!(
            resolve_on("file://LocalHost/home/ana/a%23b.html?x=1", &unix()),
            Some("file:///home/ana/a%23b.html?x=1".into())
        );
    }

    #[test]
    fn unix_absolute_and_home_paths_load_as_file_urls() {
        assert_eq!(
            resolve_on("/home/ana/page.html", &unix()),
            Some("file:///home/ana/page.html".into())
        );
        assert_eq!(resolve_on("/etc", &unix()), Some("file:///etc".into()));
        assert_eq!(
            resolve_on("~/Downloads/a.pdf", &unix()),
            Some("file:///home/ana/Downloads/a.pdf".into())
        );
        assert_eq!(resolve_on("~", &unix()), Some("file:///home/ana".into()));
        let trailing_slash = LocalFiles {
            style: PathStyle::Unix,
            home: Some("/Users/ana/".into()),
        };
        assert_eq!(
            resolve_on("~/x.html", &trailing_slash),
            Some("file:///Users/ana/x.html".into())
        );
    }

    #[test]
    fn paths_of_the_other_platform_become_a_search() {
        assert_eq!(resolve("/etc/passwd"), search("%2Fetc%2Fpasswd"));
        assert_eq!(resolve("/word"), search("%2Fword"));
        assert_search("~/Downloads/a.pdf", &windows());
        assert_search(r"C:\Users\a.html", &unix());
        assert_search("C:/Users/a.html", &unix());
        assert_search("file:///etc/passwd", &windows());
        // On macOS and Linux this is the local path `/C:/x.html`.
        assert_eq!(
            resolve_on("file:///C:/x.html", &unix()),
            Some("file:///C:/x.html".into())
        );
    }

    #[test]
    fn home_paths_without_a_home_directory_become_a_search() {
        let no_home = LocalFiles {
            style: PathStyle::Unix,
            home: None,
        };
        assert_search("~/a.html", &no_home);
        let relative_home = LocalFiles {
            style: PathStyle::Unix,
            home: Some("home/ana".into()),
        };
        assert_search("~/a.html", &relative_home);
        assert_search("~ana/a.html", &unix());
    }

    #[test]
    fn relative_and_drive_relative_paths_become_a_search() {
        for input in [
            r"Downloads\x.html",
            "Downloads/x.html",
            "C:",
            "C:x.html",
            r"1:\x",
            r".\x.html",
            "../x.html",
        ] {
            assert_search(input, &windows());
            assert_search(input, &unix());
        }
    }

    #[test]
    fn files_on_other_machines_become_a_search() {
        for input in [
            r"\\server\share\x.html",
            r"\\localhost\c$\x.html",
            r"\\?\UNC\server\share",
            r"\\.\pipe\x",
            "//server/share/x.html",
            "file://server/share/x.html",
            "file://example.com/x",
            "file://127.0.0.1/c$/x",
            "file://localhost:80/C:/x",
            "file://%6Cocalhost/C:/x",
            "file://user@localhost/C:/x",
            "file:////server/share/x.html",
            r"file:\\server\share\x.html",
            r"file:///\\server\share",
            "file:server/share",
            "file:/C:/x",
            "file://",
        ] {
            assert_search(input, &windows());
            assert_search(input, &unix());
        }
        assert_search("file:///%5C%5Cserver/share", &windows());
        assert_search("file:///", &windows());
        assert_search("//etc/passwd", &unix());
        assert_search("file:////etc/passwd", &unix());
    }

    #[test]
    fn escapes_that_are_not_hex_stay_as_typed() {
        assert_eq!(
            resolve("file:///C:/a%+1b%2.html"),
            Some("file:///C:/a%25+1b%252.html".into())
        );
    }

    #[test]
    fn file_urls_with_invalid_utf8_become_a_search() {
        assert_search("file:///C:/%FF.html", &windows());
        assert_search("file:///tmp/%C3.html", &unix());
    }

    #[test]
    fn remote_file_urls_are_recognized() {
        for url in [
            "file://server/share/x.html",
            "FILE://Example.com/x",
            r"file:\\server\share",
            "file:////server/share",
            "file://127.0.0.1/c$/x",
        ] {
            assert!(is_remote_file_url(url), "{url}");
        }
        for url in [
            "file:///C:/x.html",
            "file://localhost/C:/x.html",
            "file:///etc/hosts",
            "https://server/share",
            "mini://home/",
        ] {
            assert!(!is_remote_file_url(url), "{url}");
        }
    }

    #[test]
    fn file_urls_are_recognized() {
        assert!(is_file_url("file:///C:/x.html"));
        assert!(is_file_url("FILE://server/x"));
        assert!(!is_file_url("https://a.com/file:///x"));
        assert!(!is_file_url("filesystem:https://a.com/x"));
    }
}
