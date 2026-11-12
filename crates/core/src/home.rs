//! The home page `mini://home/`: which file or command a request URL asks for. The app serves
//! the files and runs the commands; only it ever loads the home page (the address box and
//! `--url` turn `mini:` input into a search, see [`crate::address::resolve_input`]).

use crate::{settings::TopBarMode, shortcuts::Command, update::Version};

/// The scheme of the app's own pages.
pub const SCHEME: &str = "mini";
/// The host of the home page under [`SCHEME`].
pub const HOST: &str = "home";
/// The URL new tabs open.
pub const HOME_URL: &str = "mini://home/";

/// Whether `url` uses the app's own [`SCHEME`].
pub fn is_app_url(url: &str) -> bool {
    url.get(..SCHEME.len() + 1)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("mini:"))
}

/// Whether `url` is the home page document (`mini://home/` or `mini://home/index.html`, with
/// an optional query or fragment); its files, `state` and commands are not.
pub fn is_home_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix(HOME_URL) else {
        return false;
    };
    let path = rest.split(['?', '#']).next().unwrap_or_default();
    matches!(path, "" | "index.html")
}

/// A static file of the home page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asset {
    Html,
    Css,
    Script,
}

impl Asset {
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Html => "text/html",
            Self::Css => "text/css",
            Self::Script => "text/javascript",
        }
    }
}

/// A command the home page asks the app to run, from the fixed list below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeCommand {
    /// Load the typed input in the requesting tab, resolved like address-box input.
    Go(String),
    /// Run a keyboard shortcut's command.
    Run(Command),
    /// Move the padding by one step, `-1` or `1`.
    Padding(i32),
    /// Open the download page in the system's default browser.
    Download,
}

/// What a request under [`HOME_URL`] asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Asset(Asset),
    /// The current values the page shows (see [`state_json`]).
    State,
    Command(HomeCommand),
}

/// The route of `url`, or `None` for a URL outside the home page or an unknown path, command
/// or argument.
pub fn route(url: &str) -> Option<Route> {
    let rest = url.strip_prefix(HOME_URL)?;
    let rest = rest.split('#').next().unwrap_or_default();
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    match path {
        "" | "index.html" => Some(Route::Asset(Asset::Html)),
        "home.css" => Some(Route::Asset(Asset::Css)),
        "home.js" => Some(Route::Asset(Asset::Script)),
        "state" => Some(Route::State),
        _ => command(path.strip_prefix("cmd/")?, query).map(Route::Command),
    }
}

/// The command `name` with the arguments in `query`.
fn command(name: &str, query: &str) -> Option<HomeCommand> {
    let argument = |key: &str| {
        form_urlencoded::parse(query.as_bytes())
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.into_owned())
    };
    let run = |command| Some(HomeCommand::Run(command));
    match name {
        "go" => Some(HomeCommand::Go(argument("input").unwrap_or_default())),
        "padding" => match argument("step")?.as_str() {
            "-1" => Some(HomeCommand::Padding(-1)),
            "1" => Some(HomeCommand::Padding(1)),
            _ => None,
        },
        "new-tab" => run(Command::NewTab),
        "close-tab" => run(Command::CloseTab),
        "next-tab" => run(Command::NextTab),
        "previous-tab" => run(Command::PreviousTab),
        "back" => run(Command::Back),
        "forward" => run(Command::Forward),
        "reload" => run(Command::Reload),
        "top-bar" => run(Command::ToggleTopBar),
        "quit" => run(Command::Quit),
        "last-tab" => run(Command::GoToLastTab),
        "move-to-new-window" => run(Command::MoveTabToNewWindow),
        "return-tab" => run(Command::ReturnTab),
        "settings" => run(Command::OpenSettings),
        "download" => Some(HomeCommand::Download),
        _ => None,
    }
}

/// The platform name the page uses to show `Ctrl` or `Cmd`.
pub const fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// The `state` reply: `{"platform": …, "padding": …, "topBar": "auto" | "pinned",
/// "update": "X.Y.Z" | null}`, where `update` is the newer release the page announces.
pub fn state_json(
    platform: &str,
    padding: u32,
    top_bar: TopBarMode,
    update: Option<Version>,
) -> String {
    serde_json::json!({
        "platform": platform,
        "padding": padding,
        "topBar": top_bar,
        "update": update.map(|version| version.to_string()),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command_of(url: &str) -> Option<HomeCommand> {
        match route(url) {
            Some(Route::Command(command)) => Some(command),
            _ => None,
        }
    }

    #[test]
    fn the_home_page_and_its_files_are_assets() {
        assert_eq!(route("mini://home/"), Some(Route::Asset(Asset::Html)));
        assert_eq!(
            route("mini://home/index.html"),
            Some(Route::Asset(Asset::Html))
        );
        assert_eq!(route("mini://home/#x"), Some(Route::Asset(Asset::Html)));
        assert_eq!(
            route("mini://home/home.css"),
            Some(Route::Asset(Asset::Css))
        );
        assert_eq!(
            route("mini://home/home.js?v=1"),
            Some(Route::Asset(Asset::Script))
        );
        assert_eq!(Asset::Html.mime_type(), "text/html");
        assert_eq!(Asset::Css.mime_type(), "text/css");
        assert_eq!(Asset::Script.mime_type(), "text/javascript");
    }

    #[test]
    fn state_is_its_own_route() {
        assert_eq!(route("mini://home/state"), Some(Route::State));
    }

    #[test]
    fn every_listed_command_maps_to_the_shortcut_command() {
        for (name, command) in [
            ("new-tab", Command::NewTab),
            ("close-tab", Command::CloseTab),
            ("next-tab", Command::NextTab),
            ("previous-tab", Command::PreviousTab),
            ("back", Command::Back),
            ("forward", Command::Forward),
            ("reload", Command::Reload),
            ("top-bar", Command::ToggleTopBar),
            ("quit", Command::Quit),
            ("last-tab", Command::GoToLastTab),
            ("move-to-new-window", Command::MoveTabToNewWindow),
            ("return-tab", Command::ReturnTab),
            ("settings", Command::OpenSettings),
        ] {
            assert_eq!(
                command_of(&format!("mini://home/cmd/{name}")),
                Some(HomeCommand::Run(command)),
                "{name}"
            );
        }
    }

    #[test]
    fn download_opens_the_download_page_and_takes_no_url() {
        assert_eq!(
            command_of("mini://home/cmd/download"),
            Some(HomeCommand::Download)
        );
        assert_eq!(
            command_of("mini://home/cmd/download?url=https%3A%2F%2Fevil.example%2F"),
            Some(HomeCommand::Download)
        );
    }

    #[test]
    fn go_carries_the_decoded_input() {
        assert_eq!(
            command_of("mini://home/cmd/go?input=github.com"),
            Some(HomeCommand::Go("github.com".into()))
        );
        assert_eq!(
            command_of("mini://home/cmd/go?input=rust%20cef+book&x=1"),
            Some(HomeCommand::Go("rust cef book".into()))
        );
        assert_eq!(
            command_of("mini://home/cmd/go?input=mini%3A%2F%2Fhome%2Fcmd%2Fquit"),
            Some(HomeCommand::Go("mini://home/cmd/quit".into()))
        );
        assert_eq!(
            command_of("mini://home/cmd/go"),
            Some(HomeCommand::Go(String::new()))
        );
    }

    #[test]
    fn padding_moves_one_step_either_way_only() {
        assert_eq!(
            command_of("mini://home/cmd/padding?step=-1"),
            Some(HomeCommand::Padding(-1))
        );
        assert_eq!(
            command_of("mini://home/cmd/padding?step=1"),
            Some(HomeCommand::Padding(1))
        );
        for url in [
            "mini://home/cmd/padding",
            "mini://home/cmd/padding?step=2",
            "mini://home/cmd/padding?step=-5",
            "mini://home/cmd/padding?step=0",
            "mini://home/cmd/padding?step=+1",
            "mini://home/cmd/padding?step=x",
        ] {
            assert_eq!(route(url), None, "{url}");
        }
    }

    #[test]
    fn urls_of_the_app_scheme_are_recognized() {
        assert!(is_app_url("mini://home/"));
        assert!(is_app_url("mini://home/cmd/quit"));
        assert!(is_app_url("MINI://other"));
        assert!(!is_app_url("https://mini.dev/"));
        assert!(!is_app_url("minix://home/"));
        assert!(!is_app_url("about:blank"));
        assert!(!is_app_url(""));
    }

    #[test]
    fn the_first_padding_step_wins() {
        assert_eq!(
            route("mini://home/cmd/padding?step=1&step=-1"),
            Some(Route::Command(HomeCommand::Padding(1)))
        );
    }

    #[test]
    fn unknown_paths_and_commands_have_no_route() {
        for url in [
            "mini://home/cmd/",
            "mini://home/cmd/settings/x",
            "mini://home/cmd/open-settings",
            "mini://home/cmd/focus-address",
            "mini://home/cmd/NEW-TAB",
            "mini://home/cmd/new-tab/x",
            "mini://home/cmd/new-window",
            "mini://home/cmd/return-tab/x",
            "mini://home/cmd/download/x",
            "mini://home/cmd/DOWNLOAD",
            "mini://home/cmd/update",
            "mini://home/cmd/open-url",
            "mini://home/other.html",
            "mini://home/cmd",
            "mini://homes/",
            "mini://other/",
            "https://home/cmd/quit",
            "https://example.com/mini://home/cmd/quit",
            "",
        ] {
            assert_eq!(route(url), None, "{url}");
        }
    }

    #[test]
    fn only_the_home_document_is_the_home_page() {
        assert!(is_home_url("mini://home/"));
        assert!(is_home_url("mini://home/#x"));
        assert!(is_home_url("mini://home/?a=1"));
        assert!(is_home_url("mini://home/index.html"));
        assert!(is_home_url("mini://home/index.html#x"));
        assert!(!is_home_url("mini://home/cmd/quit"));
        assert!(!is_home_url("mini://home/state"));
        assert!(!is_home_url("mini://home/home.css"));
        assert!(!is_home_url("mini://home/home.js"));
        assert!(!is_home_url("mini://home/index.htmlx"));
        assert!(!is_home_url("mini://homes/"));
        assert!(!is_home_url("https://home/"));
        assert!(!is_home_url("about:blank"));
        assert!(!is_home_url(""));
    }

    #[test]
    fn state_json_names_the_platform_padding_top_bar_and_update() {
        let value: serde_json::Value =
            serde_json::from_str(&state_json("macos", 12, TopBarMode::Pinned, None)).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "platform": "macos",
                "padding": 12,
                "topBar": "pinned",
                "update": null,
            })
        );
        let update = Version {
            major: 0,
            minor: 10,
            patch: 2,
        };
        let value: serde_json::Value =
            serde_json::from_str(&state_json("windows", 8, TopBarMode::Auto, Some(update)))
                .unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "platform": "windows",
                "padding": 8,
                "topBar": "auto",
                "update": "0.10.2",
            })
        );
    }

    #[test]
    fn the_platform_name_is_one_the_page_knows() {
        assert!(["windows", "macos", "linux"].contains(&platform_name()));
    }
}
