# Changelog

All notable changes to mini-browser are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-11-12

### Added

- A build for Intel Macs: each release also carries `mini-browser-vX.Y.Z-macos-x64.zip` and its versionless copy `mini-browser-macos-x64.zip`. Like the Apple Silicon build, it needs macOS 13 or later.
- The download page tells Apple Silicon Macs from Intel Macs and offers each its file, with a link to the other. It also says that macOS 13 or later is needed and how to open the app on macOS 15, where right-click → Open no longer works for apps that are not signed.
- The download page shows its own "Page not found" page, in the same style, for addresses that do not exist, with a link back to the downloads.

## [0.4.1] - 2026-11-07

### Added

- On Windows, `Esc` cancels a tab drag, as in Chrome: a tab dragged along the bar goes back to its place, a tab dragged out into a new window goes back into its window at its place (the new window closes), and a window moved by its only tab goes back where it was. Nothing reloads.
- On Windows, a window you move by its tab snaps like any other window: let go with the mouse at the left or right edge of a screen and it fills that half, at the top edge it maximizes.
- A download page at <https://montesgp.github.io/mini-browser/>: one button for your system, links for the others, the animated tour and first-launch help. It always offers the latest release.
- Each release also carries versionless copies of its archives (`mini-browser-windows-x64.zip`, `mini-browser-macos-arm64.zip`, `mini-browser-linux-x64.tar.gz`), listed in `SHA256SUMS.txt` and attested, so `releases/latest/download/<name>` always gets the latest version.

### Changed

- The settings box's `×` is brighter at rest, so it is easier to find; it still lights up under the mouse.

### Fixed

- In `Auto` mode, the top bar of a window you had just moved or torn off by its tab (and then maximized) sometimes did not appear at the top edge: letting go of the drag counted as a click on the tab, which hides the bar until the mouse leaves the top.

## [0.4.0] - 2026-11-07

### Added

- Clicking the home page's padding row (anywhere but its `−` and `+`) opens the settings box, like `Ctrl/Cmd+,`.
- Open files from your computer: type a path (`C:\Users\you\page.html`, `C:/…`, `/home/you/page.html` or `~/page.html`) or a `file:///` address in the home page's field or the `Ctrl/Cmd+L` box, pass it with `--url`, or drop a file on the window to open it in the current tab. Names with spaces, `#` or accents work. Files on other computers (`\\server\share\…`, `file://host/…`) are never opened and become a search, and websites still cannot open your files.
- Move a tab to a new window with `Ctrl+Shift+N` (`Cmd+Shift+N` on Mac) or the home page's "Move tab to new window" row, and back with `Ctrl+Shift+M` (`Cmd+Shift+M`) or "Return tab to its window", as in Chrome: the page keeps running without reloading. The new window opens next to the old one with its own tabs, top bar, address box and settings box; a window's only tab stays put. A returned tab goes after the current tab of the window it came from (or the window you used last), and a window left without tabs closes.
- On Windows, drag a tab up or down out of the top bar to open it in its own window, which follows the mouse until you let go, and drag a tab onto another window's top bar to move it there at that spot, as in Chrome; the page keeps running without reloading. Dragging a window's only tab moves the window, a window left without tabs closes, and a tab dropped anywhere else stays in its window. macOS and Linux use `Cmd/Ctrl+Shift+N` and `Cmd/Ctrl+Shift+M` instead.
- With several windows, closing a window (its close button or `Alt+F4`) closes only its tabs, `Ctrl+W` on the last tab of an extra window closes that window, and `Ctrl/Cmd+Q` or the home page's quit row closes every window. Settings changes apply to every window, pop-ups open in their page's window and starting the app again brings the window you used last to the front.

### Fixed

- The settings box's `×` close button is visible again; it was drawn too small to show its glyph.

## [0.3.0] - 2026-11-05

### Added

- `Ctrl+1` to `Ctrl+8` (`Cmd` on Mac) go to the tab at that position and `Ctrl+9` to the last tab, as in Chrome; the home page's commands card lists them.
- A thin top bar with your tabs and minimize, maximize/restore and close buttons (on the left on macOS). It appears when the mouse is over the strip where the bar sits, moves the page down so it never covers it (without resizing the page, so the page does not flash; its bottom edge is out of view while the bar shows), and hides half a second after it leaves, so a shared screen stays clean.
- The bar shows for a moment when you open, close or switch tabs, so you see where you are, and says how many tabs do not fit (`+3`).
- Clicking a tab in the `Auto` top bar hides the bar at once, so the page is whole again; it shows again once the mouse leaves the top and comes back. A `Pinned` bar stays.
- A small `×` on the current tab and on the tab under the mouse closes that tab, following the `Ctrl/Cmd+W` rules. Closing the last tab leaves a home page; the app quits only from the window's close button, `Alt+F4`, `Ctrl/Cmd+Q` or the home page's quit row.
- A `+` after the last tab opens a new tab, and a pin in the bar switches it between `Auto` and `Pinned`.
- While the settings box is open, `+` and `-` widen and narrow the frame.
- On Windows, a middle click on a tab closes it, and dragging a tab left or right in the top bar reorders the tabs; a press without moving still switches to the tab. macOS and Linux do not have these yet.
- A click on the frame or the top bar also closes the settings box (Windows).
- The settings box has a `×` that closes it and also closes when you click outside it, on the page or a tab.
- The home page's commands card follows setting changes made elsewhere (the bar's pin, the settings box, `Ctrl/Cmd+Shift+F`) at once.
- A "Top bar" setting, `Auto` or `Pinned`, in the settings box (`Ctrl/Cmd+,`), saved in `settings.json` as `topBar`. `Pinned` keeps the bar visible with the page below it. On Linux the bar does not follow the mouse yet: in `Auto` it shows only when tabs change.
- A dark home page (`mini://home/`) in every new tab: a centered field to type an address or a search, and a keyboard button that opens a card with every shortcut. Click a row to run it: new tab, close tab, next or previous tab, back, forward, reload, top bar `Auto`/`Pinned`, padding `−`/`+`, quit. Only the app opens this page; web pages cannot link to it, frame it, redirect to it or ask it to run commands.

### Changed

- `Ctrl/Cmd+T`, the bar's `+` and the home page's "New tab" row open the tab at the end of the tabs instead of right after the current one; tabs opened by pages and `--url` still open right after it.
- The settings box opens in the bottom-right corner of the page instead of near the top, so it no longer covers search fields.
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

[Unreleased]: https://github.com/montesgp/mini-browser/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/montesgp/mini-browser/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/montesgp/mini-browser/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/montesgp/mini-browser/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/montesgp/mini-browser/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/montesgp/mini-browser/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/montesgp/mini-browser/releases/tag/v0.1.0
