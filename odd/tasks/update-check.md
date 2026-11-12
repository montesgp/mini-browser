# mini-browser update check

Tell users who downloaded the app when a newer release exists, and send them to the download page. Notify and download only: no self-update (owner choice, L2). Lands after v0.5.0.

## Specs

- **S1 — Daily check, on by default.** User chose: "Aviso + descarga (recomendado)" — "Chequeo diario a GitHub, activado por defecto y desactivable en ajustes; aviso en inicio y ajustes con botón que abre la página de descargas". The app reads `https://api.github.com/repos/montesgp/mini-browser/releases/latest` once at startup (after the first window opens) and again every 24 hours while it runs. No state file: the check is per run. Only that fixed URL is requested, from the browser process with CEF's `UrlRequest` (no new HTTP crate). Failures (offline, rate limit, bad JSON, non-2xx, body over 1 MB) are silent and leave no notice.
- **S2 — Version comparison (core, tested).** `tag_name` must be exactly `vMAJOR.MINOR.PATCH` (decimal numbers); it is newer only when strictly greater than the running version (`CARGO_PKG_VERSION`) by numeric MAJOR, then MINOR, then PATCH. Anything else (pre-release suffixes, missing `v`, garbage, equal or older) means no update. Examples: running 0.5.0 → `v0.5.1` yes, `v0.10.0` yes, `v1.0.0` yes, `v0.5.0` no, `v0.4.9` no, `0.6.0` no, `v0.6.0-beta` no, `v0.6` no.
- **S3 — Setting.** `settings.json` gets `"checkForUpdates": true` by default. A missing or wrongly typed value counts as `true` without discarding the other settings (lenient, like `topBar`). The settings box gets a row to turn it on and off; turning it off stops future checks and hides any notice; turning it on checks right away.
- **S4 — Notice where the user already looks.** When a newer version is known: the home page (`mini://home/`) shows a small dark line "mini-browser X.Y.Z is available" with a "Download" button, and the settings box shows the same line and button. Nothing else appears (AGENTS.md invariant 4: no new permanent chrome, no popups). The home bridge keeps its rules (main frame of the active home tab only); its `state` reply carries the available version or nothing.
- **S5 — Download button.** It opens `https://montesgp.github.io/mini-browser/` in the system's default browser (mini-browser has no download support): `ShellExecuteW` on Windows, `open` on macOS, `xdg-open` on Linux. The URL is a constant; nothing from the GitHub response is ever opened or rendered except the validated version numbers.
- **S6 — Docs.** README For humans (how updates are announced, how to turn the check off, updating = replace the folder, settings and sessions stay in the app data directory), the privacy statement that the app's own code sends nothing (now: one daily request to GitHub's API, can be turned off), the settings file table; CHANGELOG `## [Unreleased]`.

## Tasks

- [ ] T1 — S2, S3, S4 (core parts): `mini-core` version parse/compare, `checkForUpdates` setting, home `state` JSON with the available version, home command for the download button — tests first — route: worker — commit: —
- [ ] T2 — S1, S3–S5 (app parts): UrlRequest client + 24 h timer, settings box row and notice, home page notice, open in system browser, docs (S6) — route: worker (same, after T1) — commit: —
- [ ] T3 — S1–S6: independent verify (network egress and launching an external program are security-relevant) — route: verify — evidence: —
- [ ] T4 — native review, PR into `dev`, ci-gate, merge — inline — merge: —

## Log

- L1 (user, original request): "Entonces tenemos pendiente el merge una vez finalizada la review? Avancemos. Ademas verifiquemos algo: Al descargar esta app queda como algo portable? Si existe algun update ya sea nueva funcion o mejoras sobre el browser, el usuario que la descargo se entera? Podemos lograr autodetect de actualizaciones? Y que bien se puedan hacer upgrade/update o ir a descargarla?"
- L2 (user decision): "Aviso + descarga (recomendado)" over "Aviso + descarga, desactivado por defecto" and "Auto-update completo".
- L3 (context): the app is extract-and-run (no installer, no registry); profile and settings live in the per-user app data directory, so replacing the folder keeps them. The app has no download support (README "No downloads… yet") and no outbound HTTP of its own today. Explore map: settings `core/src/settings.rs` + `app/src/shell/settings_box.rs` (`change()` → `home::refresh_state()`), home bridge `core/src/home.rs` (`HomeCommand`, `state_json`) + `app/src/shell/home.rs` (`answer()`, `refresh_state`, CSP `connect-src 'self'`), `cef::urlrequest_create` + `wrap_urlrequest_client!`, UI-thread timers via `post_delayed_task` (`loading_line.rs`), startup in `app.rs` `on_context_initialized`.
