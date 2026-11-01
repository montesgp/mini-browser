# mini-browser releases and open-source publication

Publish mini-browser as an open-source GitHub repository with versioned, downloadable builds per platform.
Owner `montesgp`; local clone stays at `R:\repositories\personal\mini-browser`. Branch `feat/v1` (continues the v1 feature).

## Specs

- **S1 — GitHub repository.** User: "Si, lo creamos en github y queda clonado en mi entorno local personal con el nombre que elegimos." Public repository `montesgp/mini-browser`; the existing local repository at `R:\repositories\personal\mini-browser` gets it as `origin` (no second clone).
- **S2 — Open-source license.** User: "la licencia como desees manejarlo. Es open-source cualquiera se puede hacer fork o bajarse la app y directamente utilizarla." Dual license `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, Cargo `license` field, README section). Third-party notices for CEF (BSD-3-Clause) and Chromium (`CREDITS.html`) ship in the repository docs and inside every release archive.
- **S3 — Versioning.** User: "Deberiamos manejar versionado para cuando existan actualizaciones y mejoras". Semantic Versioning; the workspace `version` in `Cargo.toml` is the single source; `CHANGELOG.md` (Keep a Changelog format) records every release; a release is a `vX.Y.Z` tag that must equal the workspace version (the release workflow fails otherwise). The README documents how to cut a release.
- **S4 — Downloads per version.** User: "se deberia poder descargar en las diferentes versiones desde github." Pushing a `vX.Y.Z` tag builds release bundles on Windows, macOS and Linux and publishes them to a GitHub Release named `vX.Y.Z` with archives `mini-browser-vX.Y.Z-<os>-<arch>.<zip|tar.gz>` plus a `SHA256SUMS.txt`; release notes come from the matching `CHANGELOG.md` section. Earlier releases stay downloadable from the Releases page. The README explains how to download and run each platform build, including that builds are unsigned (Windows SmartScreen, macOS Gatekeeper).

## Tasks

- [x] T1 — S2: license files, third-party notices, README license section — inline — commit: 725c952
- [x] T2 — S3: CHANGELOG, versioning and release process docs — inline — commit: fc7f68e
- [x] T3 — S4, S3: release workflow (tag check, 3-OS release bundles, archives, checksums, GitHub Release) — inline — commits: d3b26a6, d50dcbf (main-only tags + provenance attestation); local Windows release bundle built and run (6 processes)
- [x] T4 — S1: create `montesgp/mini-browser`, push `main` + `feat/v1`, PR, CI green on all 3 OS (fix macOS/Linux build issues it reveals), merge — inline — commits: 3196147 (cross-platform clippy fix); PR #1 merged as 2f8537a after ci-gate SUCCESS on d50dcbf; main protected (required ci-gate, no force-push)
- [x] T5 — S4: tag `v0.1.0`, release workflow green, assets and checksums verified by download — inline — release run 41330167704 green; Windows zip downloaded: SHA256 OK, SLSA provenance verified (release.yml@refs/tags/v0.1.0), contents clean, app starts (6 processes)

## Log

- L1 (user, original request): "Si, lo creamos en github y queda clonado en mi entorno local personal con el nombre que elegimos. La parte de la copie de CEF aun no se lo que es luego lo revisamos, y ademas, la licencia como desees manejarlo. Es open-source cualquiera se puede hacer fork o bajarse la app y directamente utilizarla. Deberiamos manejar versionado para cuando existan actualizaciones y mejoras y se deberia poder descargar en las diferentes versiones desde github."
- L2 (user decision): owner account "montesgp". The CEF copy question is deferred by the user ("luego lo revisamos").
- L3 (decision): license `MIT OR Apache-2.0`, matching cef-rs and the Rust ecosystem convention. Owner policy (AGENTS.md): agents merge their own PRs once `ci-gate` is SUCCESS on the head commit.
- L4 (evidence): first macOS/Linux CI run compiled and tested both; only a clippy `unnecessary_cast` (event flags are i32 on Windows, u32 elsewhere) failed, fixed in 3196147. Native reviews review-8e379b86146e8354 (risk+readability) and review-9875e5ee1eb7e72a (risk) approved and acknowledged; follow-ups applied: release only from main, build-provenance attestation. Remaining suggestions: pin actions to commit SHAs; dedupe the package file list.
- L5 (evidence): v0.1.0 published at https://github.com/montesgp/mini-browser/releases/tag/v0.1.0 (windows-x64 194 MB, macos-arm64 149 MB, linux-x64 373 MB, SHA256SUMS.txt). macOS app is `minibrowser.app`. Linux ships `chrome-sandbox` (README now documents the setuid step for userns-restricted distros) and an unstripped 1.4 GB `libcef.so` (follow-up: strip it to shrink the archive). macOS and Linux builds were never launched by a person.
