# Changelog

All notable changes to mini-browser are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- The window title is always `mini-browser`; before, the window had no title, so the taskbar, `Alt+Tab` and share pickers showed a nameless window.
- The tab strip fits the window: labels share its width evenly, long titles end in an ellipsis instead of overflowing, and with more tabs than fit the active tab stays visible. Labels use the app font, and inactive tabs stand out from the strip.
- The settings box's `−` and `+` no longer reverse direction from the keyboard at 4 or 32 px: the button at its bound is dimmed but keeps the focus, and pressing it does nothing. `−` and `+` look equally heavy.
- The address box no longer sits on a square dark slab that showed as a black block on light pages; only the rounded field shows. The settings box has a thin edge, so it stands out on dark pages.
- Starting the app while it runs no longer opens a Chrome-style window with tabs, an address bar and the page title: the running window comes to the front, and a `--url` given to the new launch opens in a new tab, resolved like the first launch. `--url <input>` now works as well as `--url=<input>`.

- Linux: the app now starts. The v0.1.0 Linux build could not find `libcef.so` next to the executable; it is now linked with an `$ORIGIN` runpath, and CI fails if the bundle cannot resolve its libraries.
- Windows: when the app data directory cannot be found, an error dialog says so; before, the app quit without a visible message.
- The settings box shows "Not saved" when the settings file cannot be written, until a later change saves.
- Switching tabs with `Ctrl+Tab` no longer outlines the first link of the page.
- The address box and the settings box no longer stack: opening one closes the other.
- The tab strip always has its intended 28 px height, whatever the height of its labels.

### Security

- Pages can open at most 3 popups (which become tabs) within 2 seconds, shared across the window so a chain of popups cannot reset the count; further popups are blocked. Tabs you open with `Ctrl/Cmd+T` are not limited.
- Chromium and CEF command-line switches such as `--no-sandbox`, `--disable-web-security` or `--remote-debugging-port` are ignored; `--url` still works.

### Changed

- Linux release archive is much smaller (about 158 MB instead of 373 MB): debug symbols are stripped from `libcef.so` and the app binary.
- CI actions are pinned to commit SHAs and kept current by Dependabot.
- CI now launches the real app on Windows, macOS and Linux (smoke test with screenshots); on Linux it also checks that Chromium's sandbox is active.

## [0.1.0] - 2026-10-31

First public release.

### Added

- Frameless, dark Chromium window (CEF 156 via the `cef` crate) with no toolbar, address bar or title bar.
- Address box (`Ctrl/Cmd+L`) that loads `http(s)` URLs, adds `https://` to host-like input and searches everything else, including other schemes such as `javascript:` and `file:`.
- Tabs (`Ctrl/Cmd+T`, `Ctrl/Cmd+W`, `Ctrl+Tab`), a toggleable tab strip (`Ctrl/Cmd+Shift+F`) and popups opening as tabs.
- Draggable padding frame and a settings box (`Ctrl/Cmd+,`) for its width, persisted in `settings.json`.
- Chromium sandbox on every platform and an isolated profile in the app data directory.
- Release builds for Windows (x64), macOS (Apple Silicon) and Linux (x64) on GitHub Releases.

[Unreleased]: https://github.com/montesgp/mini-browser/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/montesgp/mini-browser/releases/tag/v0.1.0
