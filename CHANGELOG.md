# Changelog

All notable changes to mini-browser are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
