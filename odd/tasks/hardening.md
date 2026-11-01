# mini-browser post-v0.1.0 hardening

Work through the improvement backlog left after v0.1.0: smaller Linux release, pinned CI dependencies, and the non-blocking review findings. GitHub Actions minutes are exhausted until 2026-11-03, so PRs are verified locally and merged only once `ci-gate` can run.

## Specs

- **S1 — Do the whole improvements item.** User: "Y arranquemos con los que podemos. Dentro del 3 podemos avanzar todo solo tener en cuenta que GitHub actions no va a funcionar hasta el 3/11 por limite." Item 3 was: "reducir el paquete de Linux quitando símbolos de `libcef.so` (1,4 GB), fijar las actions de GitHub por SHA, y las observaciones no bloqueantes de las revisiones (registradas en `odd/tasks/`)". No PR is merged before its `ci-gate` passes; nothing may assume Actions runs before 2026-11-03.
- **S2 — Smaller Linux release.** The release workflow strips debug symbols from `libcef.so` and the `minibrowser` binary before archiving; behavior is unchanged.
- **S3 — Pinned CI dependencies.** Every third-party action in `.github/workflows/` is pinned to a full commit SHA with the version in a comment, staying on the major versions already proven in CI; Dependabot keeps actions and Rust crates current through PRs (the `cef` crate is excluded: it is bumped deliberately with testing).
- **S4 — Review findings.** Address the non-blocking findings recorded in `odd/tasks/mini-browser.md` (L9, L13) and the verifier advisories, each with a test where the logic is testable and a functional check on Windows otherwise:
  - popups opened as tabs are throttled so a page cannot flood the window with tabs;
  - a missing app data directory shows a visible error on Windows (GUI apps have no console), not only stderr;
  - a failed settings save is visible in the settings box instead of silent;
  - the tab strip gets its intended height;
  - the address box and the settings box never stack (opening one closes the other);
  - the focus ring that appears after `Ctrl+Tab` is investigated and removed if feasible;
  - browser-process command-line switches are disabled for users (`command_line_args_disabled`) so a launch with `--no-sandbox` or `--remote-debugging-port` has no effect, while `--url` and the app's own switches keep working;
  - smaller findings where cheap: closing view left visible, shortcut character reaching the text field, tab strip rebuilt on every title change, untested pure helpers.
- **S6 — Linux release must start (defect found while doing S2).** The v0.1.0 Linux executable has no RUNPATH, so `./minibrowser` fails with `libcef.so: cannot open shared object file`. Link it with `$ORIGIN` and make CI fail when the bundled executable cannot resolve its shared libraries; record the fix in `CHANGELOG.md` for the next patch release.
- **S5 — Testing on other platforms (question).** User: "El punto 2 no tengo forma de testearlo aun, podriamos tener una maquina virtual con mac y linux en esta PC para probar este tipo de aplicaciones que desarrollo con frecuencia?" Answered with options; nothing is installed without the owner's go-ahead.

## Tasks

- [x] T1 — S2: strip `libcef.so` and the binary in Linux release packaging — inline — commit: 36b059b (measured on the v0.1.0 artifact in WSL: libcef.so 1,476 MB → 277 MB, archive 373 MB → 158 MB)
- [x] T2 — S3: pin actions by SHA, add Dependabot — inline — commit: 8c0c79e (YAML parses; no `@vN` refs left)
- [ ] T6 — S6: `$ORIGIN` runpath for the Linux binary + CI library-resolution check + CHANGELOG — inline — commit: `fix(app): let the Linux build find libcef.so`
- [ ] T3 — S4: review findings in core/app with tests and Windows functional checks — worker — commit: —
- [ ] T4 — S1: PRs opened; merge each after `ci-gate` passes (not before 2026-11-03) — inline — merge: —
- [ ] T5 — S5: answer the VM question with options — inline — no commit

## Log

- L1 (user, original request): "Y arranquemos con los que podemos. Dentro del 3 podemos avanzar todo solo tener en cuenta que GitHub actions no va a funcionar hasta el 3/11 por limite. El punto 2 no tengo forma de testearlo aun, podriamos tener una maquina virtual con mac y linux en esta PC para probar este tipo de aplicaciones que desarrollo con frecuencia?"
- L2 (evidence): this PC is a Ryzen 5 2600X, 32 GB RAM, virtualization enabled, Windows 11 Pro, with WSL2 Ubuntu 26.04 already installed (plus docker-desktop). WSL Ubuntu has binutils `strip` 2.46.
- L3 (decision): actions are pinned to the newest release within the major already used (checkout v4.4.0, cache v4.3.0, upload-artifact v4.6.2, download-artifact v4.3.0, attest-build-provenance v2.4.0) instead of jumping to the latest majors (v7, v6, v7, v8, v4), so pinning changes no behavior; Dependabot proposes major upgrades separately.
- L4 (evidence): v0.1.0 Linux artifact in WSL: `readelf -d minibrowser` shows no RPATH/RUNPATH and `./minibrowser` fails with `libcef.so: cannot open shared object file`. With `LD_LIBRARY_PATH=.` (simulating `$ORIGIN`) only system libraries are missing: libasound.so.2, libnspr4.so, libnss3.so, libnssutil3.so, libsmime3.so. WSLg display is available (DISPLAY=:0). CI never caught it because it builds and bundles but never resolves or launches the executable.
