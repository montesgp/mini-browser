# AGENTS.md

Instructions for AI coding agents working in this repository. Human-oriented docs are in [README.md](README.md); contributor rules are in [CONTRIBUTING.md](CONTRIBUTING.md). When they disagree with this file, ask the maintainer instead of guessing.

## Project in one paragraph

mini-browser is a minimal, dark, chrome-less Chromium browser for screen sharing, written in Rust on the `cef` crate (CEF/Chromium 156) with CEF Views. Platform-independent logic lives in `crates/core` (`mini-core`) and is unit-tested; the CEF shell lives in `crates/app` (`minibrowser`); `xtask` bundles the app with CEF's runtime files.

## Invariants: never break these

1. **The Chromium sandbox stays on** (`sandbox` feature, `no_sandbox = 0`). Never add `--no-sandbox`, `--disable-web-security`, a remote debugging port, or similar flags.
   User-supplied switches stay disabled (`command_line_args_disabled = 1`); read app options such as `--url` before `cef::initialize`, which clears the global command line.
2. **The address box and `--url` only load `http(s)`.** Every other scheme becomes a search (`mini_core::address::resolve_input`).
3. **The profile stays isolated** in the app data directory; no fallback to shared temporary folders.
4. **No permanent browser chrome.** New UI is opt-in (shortcut or setting) and dark.
5. **One codebase for Windows, macOS and Linux.** Platform code goes behind `cfg`; no platform-only UI toolkit.
6. **Package name `minibrowser` has no hyphen:** on Windows the sandbox bootstrap loads `<name>.dll`.

## Commands

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p mini-core
cargo test -p minibrowser --lib
cargo run -p xtask --bin bundle -- minibrowser -o target/bundle            # debug bundle
cargo run -p xtask --bin bundle -- minibrowser --release -o target/bundle  # optimized bundle
```

Requirements: Rust stable, CMake and Ninja on `PATH`, a C++ toolchain. On Windows, Visual Studio bundles CMake and Ninja under `Common7\IDE\CommonExtensions\Microsoft\CMake\{CMake\bin,Ninja}`; prepend them to `PATH` if `cmake` is not found. Set `CEF_PATH` to an extracted CEF directory to avoid re-downloading about 400 MB after `cargo clean`.

## How to change code

- Put logic that can be tested without CEF in `crates/core`, write the test first, watch it fail, then implement. Test through the public functions.
- Keep `crates/app` thin: it maps CEF callbacks to core logic. It has no automated UI tests; verify UI changes by building the bundle and running it.
- If you drive the running app with simulated input, confirm the app window is in the foreground before **every** keystroke or click, and capture only the app window. Never send input to other windows.
- When behavior changes, update `README.md` (the "For humans" part for user-visible changes, the "For agents and contributors" part for technical ones) and add an entry under `## [Unreleased]` in `CHANGELOG.md`.
- English for code, comments, docs and commit messages. Conventional Commits (`feat(app): …`, `fix(core): …`, `docs: …`, `ci: …`).
- Keep pull requests focused (about 400 changed lines or less) with tests and docs in the same PR as the behavior.

## Pull requests and releases

- `main` is protected: changes land through PRs only, with `ci-gate` green on the head commit. Only the owner, [@montesgp](https://github.com/montesgp), merges. External contributors work from forks.
- Releases are tag-driven (`vX.Y.Z` on a commit already on `main`); the steps are in the README's [Versioning and releases](README.md#versioning-and-releases) section. Do not create tags or releases unless the maintainer asks.

## Known platform quirks

- **Windows:** frameless CEF windows stay invisible unless the browser process gets `--disable-features=CalculateNativeWinOcclusion` (already set in `crates/app/src/shell/app.rs`).
- **CEF Views:** one Chrome-style `BrowserView` per Chrome-style window, so tab browsers use the Alloy runtime style. CEF also replaces the window's draggable regions after page loads; the frame regions are re-applied from `DragHandler::on_draggable_regions_changed`.
- **`cef_event_flags_t`** wraps `i32` on Windows and `u32` on macOS and Linux; casts that look redundant on one platform are needed on another.
- **macOS:** cef-rs bundles only debug builds on macOS; `xtask` adds `--release`.
