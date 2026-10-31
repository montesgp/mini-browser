# mini-browser releases and open-source publication

Publish mini-browser as an open-source GitHub repository with versioned, downloadable builds per platform.
Owner `montesgp`; local clone stays at `R:\repositories\personal\mini-browser`. Branch `feat/v1` (continues the v1 feature).

## Specs

- **S1 — GitHub repository.** User: "Si, lo creamos en github y queda clonado en mi entorno local personal con el nombre que elegimos." Public repository `montesgp/mini-browser`; the existing local repository at `R:\repositories\personal\mini-browser` gets it as `origin` (no second clone).
- **S2 — Open-source license.** User: "la licencia como desees manejarlo. Es open-source cualquiera se puede hacer fork o bajarse la app y directamente utilizarla." Dual license `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`, Cargo `license` field, README section). Third-party notices for CEF (BSD-3-Clause) and Chromium (`CREDITS.html`) ship in the repository docs and inside every release archive.
- **S3 — Versioning.** User: "Deberiamos manejar versionado para cuando existan actualizaciones y mejoras". Semantic Versioning; the workspace `version` in `Cargo.toml` is the single source; `CHANGELOG.md` (Keep a Changelog format) records every release; a release is a `vX.Y.Z` tag that must equal the workspace version (the release workflow fails otherwise). The README documents how to cut a release.
- **S4 — Downloads per version.** User: "se deberia poder descargar en las diferentes versiones desde github." Pushing a `vX.Y.Z` tag builds release bundles on Windows, macOS and Linux and publishes them to a GitHub Release named `vX.Y.Z` with archives `mini-browser-vX.Y.Z-<os>-<arch>.<zip|tar.gz>` plus a `SHA256SUMS.txt`; release notes come from the matching `CHANGELOG.md` section. Earlier releases stay downloadable from the Releases page. The README explains how to download and run each platform build, including that builds are unsigned (Windows SmartScreen, macOS Gatekeeper).

## Tasks

- [ ] T1 — S2: license files, third-party notices, README license section — inline — commit: —
- [ ] T2 — S3: CHANGELOG, versioning and release process docs — inline — commit: —
- [ ] T3 — S4, S3: release workflow (tag check, 3-OS release bundles, archives, checksums, GitHub Release) — inline — commit: —
- [ ] T4 — S1: create `montesgp/mini-browser`, push `main` + `feat/v1`, PR, CI green on all 3 OS (fix macOS/Linux build issues it reveals), merge — inline/worker — commit: —
- [ ] T5 — S4: tag `v0.1.0`, release workflow green, assets and checksums verified by download — inline — commit: —

## Log

- L1 (user, original request): "Si, lo creamos en github y queda clonado en mi entorno local personal con el nombre que elegimos. La parte de la copie de CEF aun no se lo que es luego lo revisamos, y ademas, la licencia como desees manejarlo. Es open-source cualquiera se puede hacer fork o bajarse la app y directamente utilizarla. Deberiamos manejar versionado para cuando existan actualizaciones y mejoras y se deberia poder descargar en las diferentes versiones desde github."
- L2 (user decision): owner account "montesgp". The CEF copy question is deferred by the user ("luego lo revisamos").
- L3 (decision): license `MIT OR Apache-2.0`, matching cef-rs and the Rust ecosystem convention. Owner policy (AGENTS.md): agents merge their own PRs once `ci-gate` is SUCCESS on the head commit.
