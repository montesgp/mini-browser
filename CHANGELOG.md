# Changelog

All notable changes to mini-browser are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A thin top bar with your tabs and minimize, maximize/restore and close buttons (on the left on macOS). It appears when the mouse is over the strip where the bar sits, pushes the page down so it never covers it, and hides half a second after it leaves, so a shared screen stays clean.
- The bar shows for a moment when you open, close or switch tabs, so you see where you are, and says how many tabs do not fit (`+3`).
- Clicking a tab in the `Auto` top bar hides the bar at once, so the page is whole again; it shows again once the mouse leaves the top and comes back. A `Pinned` bar stays.
- A small `×` on the current tab and on the tab under the mouse closes that tab, following the `Ctrl/Cmd+W` rules. Closing the last tab leaves a home page; the app quits only from the window's close button, `Alt+F4`, `Ctrl/Cmd+Q` or the home page's quit row.
- A `+` after the last tab opens a new tab, and a pin in the bar switches it between `Auto` and `Pinned`.
- While the settings box is open, `+` and `-` widen and narrow the frame.
- The home page's commands card follows setting changes made elsewhere (the bar's pin, the settings box, `Ctrl/Cmd+Shift+F`) at once.
- A "Top bar" setting, `Auto` or `Pinned`, in the settings box (`Ctrl/Cmd+,`), saved in `settings.json` as `topBar`. `Pinned` keeps the bar visible with the page below it. On Linux the bar does not follow the mouse yet: in `Auto` it shows only when tabs change.
- A dark home page (`mini://home/`) in every new tab: a centered field to type an address or a search, and a keyboard button that opens a card with every shortcut. Click a row to run it: new tab, close tab, next or previous tab, back, forward, reload, top bar `Auto`/`Pinned`, padding `−`/`+`, quit. Only the app opens this page; web pages cannot link to it, frame it, redirect to it or ask it to run commands.

### Changed

- New tabs open the home page instead of an empty tab with the address box; `Ctrl/Cmd+L` on the home page focuses its field. The faint `Ctrl+L to enter an address` hint is gone.
- `Ctrl/Cmd+Shift+F` now switches the top bar between `Auto` and `Pinned` and remembers it; before, it showed or hid the tab strip until restart.

### Fixed

- A maximized window has no frame, like any other window; restored, the saved frame comes back.
- Going back from the first page of a tab (`Alt+Left`, the mouse back button or the right-click menu) shows the home page; before, it showed an empty black page.

## [0.2.0] - 2026-11-03

### Added

- A thin animated line in the top of the frame while the current tab loads. It hides when the page has loaded or when you switch to a tab that is not loading.
- New tabs show a faint centered hint, `Ctrl+L to enter an address` (`Cmd+L` on macOS), until a page loads.
- An app icon: a dark frame around a window. It shows on the Windows executable, the window and the taskbar, and on the macOS app.

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

[Unreleased]: https://github.com/montesgp/mini-browser/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/montesgp/mini-browser/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/montesgp/mini-browser/releases/tag/v0.1.0
