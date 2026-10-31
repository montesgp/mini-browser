# mini-browser

A minimal, dark, chrome-less Chromium browser for screen sharing, streaming, and quick tests without the bloat and state of your everyday browser.

There is no toolbar, no address bar and no title bar: just the page inside a thin dark frame. Press `Ctrl+L` (`Cmd+L` on macOS) to type an address.

## Credits

mini-browser is inspired by **Mini**, the personal web browser built by [Guillermo Rauch](https://rauchg.com/). His idea, a browser that gets out of the way when you share your screen, and his move from Electron to Rust with the `cef` crate shaped this project. Thank you, Guillermo.

Built on:

- [cef-rs](https://github.com/tauri-apps/cef-rs) (the `cef` crate, by the Tauri team). The app layout and the `xtask` bundler are derived from its `cefsimple` example and `bundle-cef-app` tool (Apache-2.0 OR MIT).
- [Chromium Embedded Framework (CEF)](https://bitbucket.org/chromiumembedded/cef) and Chromium.

## Download

Every version is published on the [Releases page](https://github.com/montesgp/mini-browser/releases); older versions stay available there.

| Platform | File |
| --- | --- |
| Windows (x64) | `mini-browser-vX.Y.Z-windows-x64.zip` |
| macOS (Apple Silicon) | `mini-browser-vX.Y.Z-macos-arm64.zip` |
| Linux (x64) | `mini-browser-vX.Y.Z-linux-x64.tar.gz` |

Each release also has `SHA256SUMS.txt`. Check your download before running it, for example `sha256sum -c SHA256SUMS.txt --ignore-missing` (Linux/macOS: `shasum -a 256 -c`) or `Get-FileHash` in PowerShell.

Builds are not code-signed yet, so the operating system warns on first launch:

- **Windows:** extract the zip and run `minibrowser.exe`. If SmartScreen says "Windows protected your PC", choose **More info → Run anyway**.
- **macOS:** extract the zip and move the app to Applications. If macOS refuses to open it, run `xattr -dr com.apple.quarantine /Applications/minibrowser.app` once, or allow it under **System Settings → Privacy & Security**.
- **Linux:** extract the archive and run `./minibrowser`. It needs the same system libraries as Google Chrome (NSS, GTK, X11/Wayland).

Keep the extracted folder together: the app loads Chromium from the files next to it.

## Using it

| Action | Windows / Linux | macOS |
| --- | --- | --- |
| Open the address box | `Ctrl+L` | `Cmd+L` |
| New tab (opens the address box) | `Ctrl+T` | `Cmd+T` |
| Close tab (the last one closes the window) | `Ctrl+W` | `Cmd+W` |
| Next / previous tab | `Ctrl+Tab` / `Ctrl+Shift+Tab` | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| Show / hide the tab strip | `Ctrl+Shift+F` | `Cmd+Shift+F` |
| Back / forward | `Alt+Left` / `Alt+Right` | `Cmd+[` / `Cmd+]` |
| Reload | `Ctrl+R` | `Cmd+R` |
| Settings (padding) | `Ctrl+,` | `Cmd+,` |
| Quit | `Ctrl+Q` | `Cmd+Q` |

In the address box, `Enter` navigates and `Esc` closes the box. What you type is resolved like this:

- `http://…` and `https://…` load as typed.
- Something that looks like a host (`rauchg.com`, `vercel.com/home`, `medium.com/@user`) gets `https://`. `localhost` and IP addresses get `http://`.
- Everything else is a search (Google by default), including any other scheme such as `javascript:`, `file:` or `data:`. The address box never runs script or opens local files.

Drag the dark frame around the page to move the window; on Windows you can also drag the window edges to resize it (not yet checked on macOS and Linux). Links that open a new window (`target=_blank`, `window.open`) open as a new tab instead.

You can also start on a page: `minibrowser --url=rauchg.com`. The `--url` value goes through the same resolution as the address box.

## Settings

Settings live in `settings.json` inside the app data directory:

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

- `padding`: frame width in pixels, 4 to 32 (default 8). Change it with the `−` / `+` buttons in the settings box (`Ctrl+,`); out-of-range values are clamped.
- `searchTemplate`: an `http(s)` URL containing `{query}`, for example `https://duckduckgo.com/?q={query}`. Anything else falls back to Google.

A corrupt settings file is ignored and defaults are used; the file is left as is until you change a setting, which rewrites it.

## Security

mini-browser keeps Chromium's protections in place and adds no shortcuts around them:

- **Sandbox on.** The Chromium sandbox is enabled on every platform. On Windows, the bundle runs through CEF's `bootstrap.exe`, which hosts the app's DLL so that renderer, GPU and utility processes start sandboxed.
- **Isolated profile.** Cookies, cache and storage live in the app data directory above, separate from your other browsers. If that directory cannot be determined, the app refuses to start rather than falling back to a shared temporary folder.
- **No debugging backdoors.** No remote debugging port, and no flag that weakens web security.
- **No permission prompts.** Tabs use CEF's Alloy runtime style, which has no permission prompt UI, and mini-browser registers no permission handler, so requests for camera, microphone, location and similar are not granted. That suits screen sharing; use your main browser to join calls.

Know the trade-offs:

- **No Google Safe Browsing.** CEF does not include Safe Browsing, so mini-browser does not warn about known phishing or malware sites.
- **You own the updates.** Chromium is bundled with the app, not updated by the operating system. Staying secure means bumping the `cef` crate (and rebuilding) whenever a new Chromium security release lands; watch the [cef-rs releases](https://github.com/tauri-apps/cef-rs/releases). An outdated build is an outdated browser.
- **Windows rendering flag.** On Windows the browser process runs with `--disable-features=CalculateNativeWinOcclusion`, because occlusion tracking kept the frameless window from being drawn. It affects rendering only (Chromium keeps drawing when the window is fully covered); it has no effect on web security.
- **No downloads, printing or find-in-page** in this version.

## Building

Requirements on every platform:

- Rust (stable), via [rustup](https://rustup.rs/).
- CMake and Ninja on `PATH` (used to compile CEF's C++ wrapper).
- A C++ toolchain: Visual Studio with the C++ workload on Windows (it also bundles CMake and Ninja under `Common7\IDE\CommonExtensions\Microsoft\CMake`), Xcode command-line tools on macOS, `gcc`/`g++` on Linux.

Build and bundle (the first build downloads CEF, about 400 MB, and compiles its wrapper):

```sh
cargo run -p xtask --bin bundle -- minibrowser -o target/bundle
```

Then run:

| OS | Command |
| --- | --- |
| Windows | `target\bundle\minibrowser.exe` |
| macOS | `open target/bundle/minibrowser.app` |
| Linux | `target/bundle/minibrowser` |

To avoid downloading CEF again after `cargo clean` or in other checkouts, extract it once and point `CEF_PATH` at it (see the [cef-rs README](https://github.com/tauri-apps/cef-rs#install-shared-cef-binaries)).

Checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p mini-core
cargo test -p minibrowser --lib   # app unit tests (Windows sandbox build only for now)
```

### Platform status

Windows is built and tested. macOS and Linux share the same code and build through the same commands, but they have not been built or tested yet. The CI workflow (`.github/workflows/build.yml`) builds all three once the repository has a GitHub remote; it has not run yet.

## Versioning and releases

mini-browser follows [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`. While the version is `0.x`, a minor bump may change behavior. A Chromium security update ships as a new patch or minor release, so staying on the latest release keeps you on a patched Chromium.

The version lives in one place, `version` under `[workspace.package]` in `Cargo.toml`, and every release is recorded in [CHANGELOG.md](CHANGELOG.md).

To cut a release:

1. Move the `Unreleased` notes in `CHANGELOG.md` under a new `## [X.Y.Z] - YYYY-MM-DD` heading and update the links at the bottom.
2. Set `version = "X.Y.Z"` in `Cargo.toml` and run `cargo check` so `Cargo.lock` follows.
3. Merge that change to `main`, then tag it and push the tag:

   ```sh
   git tag -a vX.Y.Z -m "mini-browser vX.Y.Z"
   git push origin vX.Y.Z
   ```

The [release workflow](.github/workflows/release.yml) checks that the tag matches the `Cargo.toml` version and that `CHANGELOG.md` has a section for it, builds optimized bundles on Windows, macOS and Linux, and publishes them with `SHA256SUMS.txt` as the GitHub Release `vX.Y.Z`, using that changelog section as the release notes.

## Project layout

| Path | What it holds |
| --- | --- |
| `crates/core` (`mini-core`) | Platform-independent logic with unit tests: address resolution, tab model, settings, shortcut table. |
| `crates/app` (`minibrowser`) | The CEF Views shell: window, address box, tabs, tab strip, settings box. |
| `xtask` | The `bundle` tool that builds the app and lays out a runnable CEF bundle. |

## License

mini-browser is free and open source, dual-licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

at your option. Fork it, change it, ship it.

Release builds bundle CEF and Chromium, which keep their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and the `CREDITS.html` file inside each release archive.

Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in mini-browser, as defined in the Apache-2.0 license, is dual-licensed as above, without any additional terms or conditions.
