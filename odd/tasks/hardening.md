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
- **S7 — Linux tested here; macOS assumed.** User: "Si avancemos por lado linux y mac no nos preocupamos. Asumimos que funciona hasta que alguien me de feedback. O yo mismo en algun momento cuando compre pc MAC testeo. La opcion 3 si es gratuita la podemos aceptar." Run the Linux build in the existing WSL2 + WSLg Ubuntu (installing only its runtime libraries) and check rendering and sandboxing; add a free CI smoke test that launches the real app on macOS (and Windows/Linux), keeps screenshots, and on Linux asserts the sandbox is active.
- **S5 — Testing on other platforms (question).** User: "El punto 2 no tengo forma de testearlo aun, podriamos tener una maquina virtual con mac y linux en esta PC para probar este tipo de aplicaciones que desarrollo con frecuencia?" Answered with options; nothing is installed without the owner's go-ahead.

## Tasks

- [x] T1 — S2: strip `libcef.so` and the binary in Linux release packaging — inline — commit: 36b059b (measured on the v0.1.0 artifact in WSL: libcef.so 1,476 MB → 277 MB, archive 373 MB → 158 MB)
- [x] T2 — S3: pin actions by SHA, add Dependabot — inline — commit: 8c0c79e (YAML parses; no `@vN` refs left)
- [x] T6 — S6: `$ORIGIN` runpath for the Linux binary + CI library-resolution check + CHANGELOG — inline — commit: 121d921 (CI `build (ubuntu-latest)` with the library-resolution check passed on 121d921)
- [x] T3 — S4: review findings in core/app with tests and Windows functional checks — worker — commits: 0342a50 (core, RED 12 → GREEN 65), a4cfecc (app), 02afe47 (docs), `fix(app): share the pop-up budget across the window` (review follow-up)
- [x] T4 — S1: PRs opened; merge each after `ci-gate` passes — inline — merge: #5 as b9a859e (ci-gate SUCCESS on 121d921), #6 as e432de3 (ci-gate SUCCESS on f14d1d9)
- [x] T5 — S5: answer the VM question with options — inline — answered: Linux via existing WSL2+WSLg (Hyper-V Ubuntu VM for pre-release checks); macOS only on Apple hardware (Mac mini, cloud Mac, or CI runner)
- [x] T7 — S7: Linux run in WSLg + CI smoke test on 3 OS — inline — commit: `ci: launch the real app in a smoke test on every OS` (WSLg: renders, zygote children in their own user/PID namespaces, renderers Seccomp 2; smoke script Windows path passed locally: 6 processes)

## Log

- L1 (user, original request): "Y arranquemos con los que podemos. Dentro del 3 podemos avanzar todo solo tener en cuenta que GitHub actions no va a funcionar hasta el 3/11 por limite. El punto 2 no tengo forma de testearlo aun, podriamos tener una maquina virtual con mac y linux en esta PC para probar este tipo de aplicaciones que desarrollo con frecuencia?"
- L2 (evidence): this PC is a Ryzen 5 2600X, 32 GB RAM, virtualization enabled, Windows 11 Pro, with WSL2 Ubuntu 26.04 already installed (plus docker-desktop). WSL Ubuntu has binutils `strip` 2.46.
- L3 (decision): actions are pinned to the newest release within the major already used (checkout v4.4.0, cache v4.3.0, upload-artifact v4.6.2, download-artifact v4.3.0, attest-build-provenance v2.4.0) instead of jumping to the latest majors (v7, v6, v7, v8, v4), so pinning changes no behavior; Dependabot proposes major upgrades separately.
- L4 (evidence): v0.1.0 Linux artifact in WSL: `readelf -d minibrowser` shows no RPATH/RUNPATH and `./minibrowser` fails with `libcef.so: cannot open shared object file`. With `LD_LIBRARY_PATH=.` (simulating `$ORIGIN`) only system libraries are missing: libasound.so.2, libnspr4.so, libnss3.so, libnssutil3.so, libsmime3.so. WSLg display is available (DISPLAY=:0). CI never caught it because it builds and bundles but never resolves or launches the executable.
- L5 (evidence): worker fixed all S4 findings with Windows functional checks (popup cap 3/2 s, MessageBox for missing data dir, settings save warning, strip height, overlay exclusivity, focus ring, `--no-sandbox`/`--remote-debugging-port` ignored with a negative control, `--url` still works). Native review review-fd55909b16b50f54 (risk+resilience+reliability) over 1ba2356..02afe47 approved and acknowledged; three lenses flagged that a per-opener popup budget is bypassed by popup chains, so the budget is now window-wide (unit key) and refusals are logged.
- L6 (evidence): owner approved Linux testing; installed libnss3 and libasound2t64 in WSL Ubuntu as root (no other changes; test files under ~/mini-browser-test removed afterwards). v0.1.0 Linux build with LD_LIBRARY_PATH=. (equivalent to the $ORIGIN fix) rendered rauchg.com dark in a WSLg window; zygote-forked children ran in user namespace 4026533243 (host 4026531837) with own PID namespaces and Seccomp 2 (filter). macOS stays untested by decision until user feedback or owner hardware. GitHub-hosted standard runners, including macOS, are free for public repositories.
- L7 (evidence, 2026-11-02): the Actions limit never applied to this public repository: its minutes are fully discounted (November: 249 min, gross $6.01, net $0) and CI ran normally on 2026-11-02, so the 2026-11-03 wait in S1 was unnecessary. The limit the owner saw is most likely on the Incoders-Tools account (private repositories). PR #5 and PR #6 were merged after `ci-gate` passed on their exact head commits, with no conflicts against `main` (which already had PR #7).
