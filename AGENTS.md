# AGENTS.md

Instructions for AI coding agents working in this repository. Human-oriented docs are in [README.md](README.md); contributor rules are in [CONTRIBUTING.md](CONTRIBUTING.md). When they disagree with this file, ask the maintainer instead of guessing.

## Project in one paragraph

mini-browser is a minimal, dark, chrome-less Chromium browser for screen sharing, written in Rust on the `cef` crate (CEF/Chromium 156) with CEF Views. Platform-independent logic lives in `crates/core` (`mini-core`) and is unit-tested; the CEF shell lives in `crates/app` (`minibrowser`); `xtask` bundles the app with CEF's runtime files.

## Invariants: never break these

1. **The Chromium sandbox stays on** (`sandbox` feature, `no_sandbox = 0`). Never add `--no-sandbox`, `--disable-web-security`, a remote debugging port, or similar flags.
   User-supplied switches stay disabled (`command_line_args_disabled = 1`); read app options such as `--url` before `cef::initialize`, which clears the global command line.
2. **The address box, the home page's field and `--url` only load `http(s)` and local files.** Local files are `file:` URLs with an empty or `localhost` host and absolute local paths (`C:\…` on Windows, `/…` and `~/…` on macOS and Linux); UNC paths and `file://host/…` stay searches, and the app cancels navigations and requests to `file:` on another machine and redirects into `file:`. Web pages cannot open local files (Chromium's rule; never set `allow_file_access_from_file_urls` or similar). Every other scheme becomes a search (`mini_core::address::resolve_input`), `mini:` included: only the app itself loads its home page `mini://home/` (new tabs). The home page's command bridge answers only `fetch` requests from the main frame of the active tab while it shows `mini://home/` (`crates/app/src/shell/home.rs`); keep its CSP, the scheme's display isolation and the cancelling of redirects into `mini:` (`on_before_browse` in `crates/app/src/shell/client.rs`).
3. **The profile stays isolated** in the app data directory; no fallback to shared temporary folders.
4. **No permanent browser chrome by default.** The top bar (tabs and window buttons) appears only when the mouse reaches the top edge, when tabs change, or when the user pins it in settings. Other new UI is opt-in (shortcut or setting) and dark; no toolbars, bookmarks or extensions.
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

- Branches: feature branch → PR into `dev` → promote `dev` to `main` with a PR once the changes are final, then fast-forward `dev` to `main`. `main` is the final version and the only source of GitHub Pages and releases. The one exception to targeting `dev` is a `hotfix/...` branch from `main` for a broken release; see [CONTRIBUTING.md § Hotfixes](CONTRIBUTING.md#hotfixes). Both `dev` and `main` are protected: changes land through PRs only, with `ci-gate` green on the head commit. Only the owner, [@montesgp](https://github.com/montesgp), merges; the owner's coding agents act for the owner and merge their own PRs once `ci-gate` is SUCCESS on the exact head commit; never merge into a branch whose checks did not run. External contributors work from forks.
- Releases are tag-driven (`vX.Y.Z` on a commit already on `main`); the steps are in the README's [Versioning and releases](README.md#versioning-and-releases) section. Do not create tags or releases unless the maintainer asks.

## Known platform quirks

- **Windows:** frameless CEF windows stay invisible unless the browser process gets `--disable-features=CalculateNativeWinOcclusion` (already set in `crates/app/src/shell/app.rs`).
- **CEF Views:** one Chrome-style `BrowserView` per Chrome-style window, so tab browsers use the Alloy runtime style. CEF also replaces the window's draggable regions after page loads; the frame regions are re-applied from `DragHandler::on_draggable_regions_changed`.
- **`cef_event_flags_t`** wraps `i32` on Windows and `u32` on macOS and Linux; casts that look redundant on one platform are needed on another.
- **macOS:** cef-rs bundles only debug builds on macOS; `xtask` adds `--release`.
