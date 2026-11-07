# Polish v0.4.1 and download site

Four tab-drag and settings-box improvements released as v0.4.1, then a GitHub Pages download site.
Owner decisions (2026-11-07): improvements ship as v0.4.1; site on GitHub Pages; no promotion yet.

## Specs

- S1 — Esc cancels a tab drag, as in Chrome: "Que `Esc` cancele el arrastre de una pestaña." A tab reordered in its bar goes back to where it was; a tab torn off into a new window goes back into its source window at its original place and the new window closes; a window moved by its only tab goes back to where it was. Windows only (the drag is Windows only).
- S2 — Aero Snap while dragging a window by its tab: "El acople a los bordes de la pantalla (Aero Snap) al arrastrar una ventana." Releasing a window that follows the pointer (torn off, or moved by its only tab) with the pointer at the left or right edge of a monitor fills that half of the monitor's work area; at the top edge it maximizes. Elsewhere it stays where it was released.
- S3 — "La barra en modo Auto, que a veces no aparece en una ventana separada y maximizada." In Auto mode, a window opened by tear-off and then maximized shows its top bar when the pointer reaches the top edge, like any other window.
- S4 — "Una `×` más visible en reposo." The settings box `×` is easier to see at rest (brighter than the dim label color) and still brightens on hover.
- S5 — Release v0.4.1 with S1–S4 (CHANGELOG `## [0.4.1]`, `Cargo.toml` version, tag, verified assets), as for v0.4.0.
- S6 — Download site on GitHub Pages: "un github pages que muestre identidad visual del navegador y una opcion de download para las 3 plataformas que descargue el zip de release", "mas simple para los usuarios no tan tecnicos". Dark, matching the app's look (icon, name, tagline "A dark, chrome-less browser for screen sharing"), the quick demo shown on the page, one big Download button for the visitor's system (Windows, macOS, Linux detected) plus links for the other two, short first-launch help (Windows SmartScreen "More info → Run anyway"; macOS right-click → Open; Linux extract and run), links to GitHub, verifying downloads and the code signing policy. Downloads use `releases/latest/download/<stable name>`, so `release.yml` also publishes versionless copies of the three archives (included in `SHA256SUMS.txt`).
- S7 — Invariants untouched: sandbox, profile isolation, address-box scheme rules; no promotion posts.

## Tasks

- [ ] T1 — S1, S2, S3, S4: drag and settings polish (core rules with tests: snap target, cancel restore) — worker — commit: —
- [ ] T2 — S5: release v0.4.1 — parent — commit: —
- [ ] T3 — S6: GitHub Pages site + versionless release assets — worker — commit: —

## Log

- L1 (owner, 2026-11-07, verbatim): "Bueno inclui las mejoras y promovamos y distribuyamos. La parte de sighnpath sabes cuanto suelen tardar desde Microsoft por esto? Porque aun no recibi nada"
- L2 (owner answers): distribution: "Con distribuir me referia a como veniamos con github releases, pero puede ser una buena opcion usar scoop winget y demas, pero deberiamos cubrir todas las plataformas si hacemos para una. Sino, de que manera podemos tener tipo un github pages que muestre identidad visual del navegador y una opcion de download para las 3 plataformas que descargue el zip de release? Como lo ves a eso? Podria ser mas simple para los usuarios no tan tecnicos? Como esta haciendo el mundo con este tipo de productos lanzados"; promotion: "Nada de promoción por ahora"; version: "v0.4.1 (Recomendado)"; site: "Sí, hacer la página".
