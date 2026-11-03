# mini-browser

A minimal, dark, chrome-less Chromium browser for screen sharing, streaming and quick tests, without the bloat and state of your everyday browser.

![mini-browser: typing an address, opening a tab, showing the tab strip and changing the padding](docs/demo.gif)

**[Download](https://github.com/montesgp/mini-browser/releases/latest)** · [For humans](#for-humans) · [For agents and contributors](#for-agents-and-contributors) · [Contributing](CONTRIBUTING.md)

---

# For humans

## What it is

- **Just the page.** No toolbar, no address bar, no title bar: the website sits inside a thin dark frame, so what you share is only what matters.
- **Type an address when you need one.** Press `Ctrl+L` (`Cmd+L` on Mac), type, press `Enter`. It disappears again.
- **Separate from your browser.** Its own cookies and history, nothing from your everyday browser shows up in a meeting.
- **Real Chromium, with its security on.** Same engine as Chrome, with Chromium's sandbox enabled.

## Download and run

Get the file for your computer from the **[latest release](https://github.com/montesgp/mini-browser/releases/latest)** (every older version stays on the [Releases page](https://github.com/montesgp/mini-browser/releases)):

| Your computer | Download | Then |
| --- | --- | --- |
| Windows | `…-windows-x64.zip` | Extract it and open `minibrowser.exe`. |
| Mac (Apple Silicon) | `…-macos-arm64.zip` | Extract it, move `minibrowser.app` to Applications and open it. |
| Linux | `…-linux-x64.tar.gz` | Extract it and run `./minibrowser`. |

The first time, your system may warn you because the app is not signed by a paid certificate yet:

- **Windows:** "Windows protected your PC" → **More info → Run anyway**.
- **Mac:** if it will not open, allow it under **System Settings → Privacy & Security**.
- **Linux:** if it stops with a sandbox error, see [Linux sandbox setup](#linux-sandbox-setup).

Keep the extracted folder together: the app needs the files next to it.

## Using it

| To | Windows / Linux | Mac |
| --- | --- | --- |
| Type an address or search | `Ctrl+L` | `Cmd+L` |
| Open a new tab | `Ctrl+T` | `Cmd+T` |
| Close the tab (the last one closes the window) | `Ctrl+W` | `Cmd+W` |
| Switch tabs | `Ctrl+Tab` / `Ctrl+Shift+Tab` | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| Show or hide the tab list | `Ctrl+Shift+F` | `Cmd+Shift+F` |
| Back / forward | `Alt+Left` / `Alt+Right` | `Cmd+[` / `Cmd+]` |
| Reload | `Ctrl+R` | `Cmd+R` |
| Change the frame size | `Ctrl+,` | `Cmd+,` |
| Quit | `Ctrl+Q` | `Cmd+Q` |

Drag the dark frame to move the window. Type a website (`github.com`) or any words to search.

- **A new tab shows a faint hint**, `Ctrl+L to enter an address` (`Cmd+L` on Mac), until a page loads.
- **A thin blue line moves along the top of the frame** while the page loads.
- **The window is always called `mini-browser`**, so the taskbar, `Alt+Tab` and a meeting's share picker never show the title of the page you have open.
- **The tab list fits the window.** Long titles end in `…`; with more tabs than fit, it shows the ones around the current tab.
- **Starting it again brings the open window to the front** instead of opening a second one. Started with an address (`--url=github.com`), it also opens that address in a new tab.

## Good to know

- **Camera, microphone and location requests are not granted.** It is made for showing pages, not for joining calls; use your main browser for those.
- **Pop-ups open as tabs, a few at a time.** Pages can open at most 3 pop-up tabs every 2 seconds in total, so nothing can flood your window.
- **No downloads, printing or find-in-page** yet.
- **Keep it updated.** Each release ships a recent Chromium; newer releases carry its security fixes.

## Credits

mini-browser is inspired by **Mini**, the personal web browser built by [Guillermo Rauch](https://rauchg.com/). His idea, a browser that gets out of the way when you share your screen, and his move from Electron to Rust with the `cef` crate shaped this project. Thank you, Guillermo.

Built on [cef-rs](https://github.com/tauri-apps/cef-rs) (the `cef` crate, by the Tauri team), the [Chromium Embedded Framework](https://bitbucket.org/chromiumembedded/cef) and Chromium.

## License and contributing

Free and open source under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Found a bug or have an idea? [Open an issue](https://github.com/montesgp/mini-browser/issues/new/choose). Want to help? Read [CONTRIBUTING.md](CONTRIBUTING.md).

---

# For agents and contributors

This part is written for AI coding agents and developers working on or with this repository. Agents should also read [AGENTS.md](AGENTS.md), which lists the rules to follow when changing code here.

## Repository map

| Path | What it holds |
| --- | --- |
| `crates/core` (`mini-core`) | Platform-independent logic with unit tests: address resolution, tab model (with per-tab loading and blank state), settings, shortcut table, pop-up throttle, window geometry, tab strip layout. |
| `crates/app` (`minibrowser`) | The CEF Views shell: window, address box, tabs, tab strip, settings box, loading line, empty-tab hint. |
| `xtask` | The `bundle` tool that builds the app and lays out a runnable CEF bundle (port of cef-rs `bundle-cef-app`, plus `--release` on macOS). |
| `assets/icon` | The app icon: `icon.svg` is the master; `icon.ico`, `icon.icns` and the PNGs are rasters of it. |
| `.github/workflows/build.yml` | CI on Windows, macOS and Linux; `ci-gate` is the single required check. |
| `.github/workflows/release.yml` | Tag-triggered release pipeline. |
| `odd/tasks/` | Feature specs and decision logs from development. |

## Build, test and bundle

Prerequisites on every platform:

- Rust (stable), via [rustup](https://rustup.rs/).
- CMake and Ninja on `PATH` (they compile CEF's C++ wrapper).
- A C++ toolchain: Visual Studio with the C++ workload on Windows (it bundles CMake and Ninja under `Common7\IDE\CommonExtensions\Microsoft\CMake`), Xcode command-line tools on macOS, `gcc`/`g++` on Linux.

```sh
# Checks (the same ones CI runs)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p mini-core
cargo test -p minibrowser --lib   # app unit tests; they compile on the Windows sandbox build only for now

# Runnable bundle (debug); add --release for an optimized one
cargo run -p xtask --bin bundle -- minibrowser -o target/bundle
```

The first build downloads CEF (about 400 MB) into `target/`. To reuse one copy across `cargo clean` and checkouts, extract it once and set `CEF_PATH` (see the [cef-rs README](https://github.com/tauri-apps/cef-rs#install-shared-cef-binaries)).

| OS | Run the bundle |
| --- | --- |
| Windows | `target\bundle\minibrowser.exe` (CEF's `bootstrap.exe`, which loads `minibrowser.dll` sandboxed) |
| macOS | `open target/bundle/minibrowser.app` |
| Linux | `target/bundle/minibrowser` |

Start on a page with `minibrowser --url=<input>` (or `--url <input>`); the value goes through the same resolution as the address box (`mini_core::launch::url_input`). Launching again while the app runs is handled in `on_already_running_app_relaunch`: CEF forwards the new launch's command line, already cleared of user switches, to the running process; the app re-adds its own `--url` input in `on_before_command_line_processing` so it survives, and the running process only reads that input, opens it in a new tab when it resolves (`mini_core::launch::relaunch`) and brings the window to the front; the second process exits with code 0.

### App icon

The icon (a dark frame around a window, with the blue loading line on top) lives in `assets/icon`. `icon.svg` is the master, and the rasters are committed next to it: regenerate `icon.ico` (16 to 256 px), `icon.icns` (16 to 1024 px), `icon.png` (512 px), `icon-16.png` and `icon-32.png` from the SVG whenever it changes. Where each one goes:

- **Window and taskbar, every OS:** `crates/app/src/shell/window.rs` embeds the PNGs and sets them as the CEF window icon (16 px, with 32 px for 2x screens) and app icon (512 px).
- **Windows executable:** the bundle's `minibrowser.exe` is CEF's prebuilt `bootstrap.exe`, so the app's own build cannot give it an icon. `xtask` writes `icon.ico` into the copied executable's resources as the named icon group `MINIBROWSER`, which Explorer, shortcuts and pinned taskbar buttons pick before the bootstrap's generic icon.
- **macOS app bundle:** `xtask` copies `icon.icns` to `minibrowser.app/Contents/Resources` and sets `CFBundleIconFile` in `Info.plist` with `plutil`.

### Platform status

Windows is built, tested and exercised by hand. Linux (x64) was run by hand in WSL2 with WSLg (Ubuntu 26.04): it renders, and its renderers run inside Chromium's namespace and seccomp sandbox. macOS (Apple Silicon) compiles, passes tests and is bundled, but nobody has used it by hand yet. CI launches the real app on all three systems (`.github/scripts/smoke-test.sh`): it must stay up with its helper processes, and on Linux with seccomp-sandboxed children. Windows ARM64, Linux ARM64 and Intel Macs are supported by cef-rs but not built.

## Address resolution

Implemented in `crates/core/src/address.rs`:

- `http://` and `https://` input loads as typed.
- Host-like input gets `https://`: at least two dot-separated labels ending in an alphabetic or punycode TLD, no `@` in the authority (`github.com`, `vercel.com/home`, `medium.com/@user`). `localhost` and IP addresses get `http://`.
- Everything else is a search through the configured template, including any other scheme (`javascript:`, `file:`, `data:`, `chrome:`). Empty input does nothing.

## Settings file

`settings.json` in the app data directory (`crates/core/src/settings.rs`):

| OS | App data directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%\minibrowser` |
| macOS | `~/Library/Application Support/minibrowser` |
| Linux | `$XDG_DATA_HOME/minibrowser` (default `~/.local/share/minibrowser`) |

```json
{
  "padding": 8,
  "searchTemplate": "https://www.google.com/search?q={query}"
}
```

- `padding`: frame width in pixels, 4 to 32 (default 8, 2 px per settings-box step). Out-of-range integers are clamped.
- `searchTemplate`: an `http(s)` URL containing `{query}`. Anything else falls back to Google.
- A corrupt file yields defaults and stays untouched until a setting changes, which rewrites it atomically.

The same directory holds the browser profile (`profile/`). If no app data directory can be determined, the app exits with an error (a dialog on Windows) instead of using a shared temporary folder. If saving settings fails, the settings box says so and the change applies until restart.

## Security model

- **Sandbox on** on every platform. On Windows the bundle runs through CEF's `bootstrap.exe`, which hosts the app's DLL so renderer, GPU and utility processes start sandboxed.
- **Isolated profile** in the app data directory.
- **No debugging backdoors:** no remote debugging port, no flag that weakens web security, and Chromium/CEF switches given on the command line are ignored (`Settings.command_line_args_disabled`), so a launch with `--no-sandbox` or `--remote-debugging-port` has no effect, also when it is a second launch handed to the running app. The app reads its own `--url` before CEF initializes; its built-in switches are added in `on_before_command_line_processing`.
- **Pop-up throttle:** pop-ups become tabs, at most 3 per window every 2 seconds, shared by all pages so a chain of pop-ups cannot reset it (`mini_core::popups::PopupThrottle` with one key); extra ones are cancelled and logged to stderr.
- **No permission prompts:** tabs use CEF's Alloy runtime style (CEF allows one Chrome-style browser per window), which has no permission UI, and no permission handler is registered, so camera, microphone, location and similar requests are not granted.

Trade-offs:

- **No Google Safe Browsing** in CEF.
- **Updates are on us.** Chromium is bundled; staying secure means bumping the `cef` crate and releasing whenever a Chromium security release lands ([cef-rs releases](https://github.com/tauri-apps/cef-rs/releases)).
- **Windows rendering flag:** the browser process runs with `--disable-features=CalculateNativeWinOcclusion`, because occlusion tracking kept the frameless window from being drawn. Rendering only; no web-security effect.

Report vulnerabilities privately: see [SECURITY.md](SECURITY.md).

### Linux sandbox setup

On distributions that restrict unprivileged user namespaces (for example Ubuntu 23.10 and later), Chromium's sandbox helper must be owned by root and setuid. If the app exits with a sandbox error, run this in the extracted folder; do not disable the sandbox instead:

```sh
sudo chown root:root chrome-sandbox && sudo chmod 4755 chrome-sandbox
```

## Verifying downloads

Each release has `SHA256SUMS.txt` and a signed build-provenance attestation per archive:

```sh
sha256sum -c SHA256SUMS.txt --ignore-missing         # macOS: shasum -a 256 -c SHA256SUMS.txt
gh attestation verify mini-browser-vX.Y.Z-linux-x64.tar.gz -R montesgp/mini-browser
```

On Windows, compare `Get-FileHash <file>` with the matching line. On macOS, `xattr -dr com.apple.quarantine /Applications/minibrowser.app` removes the download quarantine flag if Gatekeeper refuses the unsigned app.

## Versioning and releases

[Semantic Versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`; while `0.x`, a minor bump may change behavior). The single version source is `version` under `[workspace.package]` in `Cargo.toml`; every release is recorded in [CHANGELOG.md](CHANGELOG.md).

To cut a release:

1. Move the `Unreleased` notes in `CHANGELOG.md` under `## [X.Y.Z] - YYYY-MM-DD` and update the links at the bottom.
2. Set `version = "X.Y.Z"` in `Cargo.toml` and run `cargo check` so `Cargo.lock` follows.
3. Merge to `main` through a PR, then tag and push:

   ```sh
   git tag -a vX.Y.Z -m "mini-browser vX.Y.Z"
   git push origin vX.Y.Z
   ```

[`release.yml`](.github/workflows/release.yml) checks that the tagged commit is on `main`, that the tag matches `Cargo.toml` and that `CHANGELOG.md` has the section, builds optimized bundles on Windows (x64), macOS (arm64) and Linux (x64), attests their provenance, and publishes them with `SHA256SUMS.txt` as GitHub Release `vX.Y.Z`, using the changelog section as release notes.

## Contribution workflow

Only the owner ([@montesgp](https://github.com/montesgp)) can merge. Everyone else forks, branches from `main`, and opens a pull request; `main` accepts changes only through PRs with a green `ci-gate`. Details in [CONTRIBUTING.md](CONTRIBUTING.md); conduct rules in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. Release builds bundle CEF and Chromium under their own licenses: see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and the `CREDITS.html` inside each archive. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion, as defined in the Apache-2.0 license, is dual-licensed as above, without any additional terms or conditions.
