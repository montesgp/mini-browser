# mini-browser top bar: visible tabs and window controls

Make open tabs and window management discoverable without breaking the chrome-less screen-sharing idea: a thin top bar that appears on mouse hover (or stays pinned by a setting), plus the small leftovers from the v0.2.0 review.

## Specs

- **S1 — Window controls and visible tabs.** User: "Actualmente no se visualizan pestañas abiertas ni manejo de la ventana, quedo todo por comandos de teclados. Debemos tener la posibilidad de miniza maximizar y cerrar tal como todas las ventantanas de todos los S.O. Quiza todo muy fino al maximo de reduccion para respetar la idea de mini, pero tenerlo. Al menos en windows no veo gestion de la ventana ni pestañas dentro del browser. No se donde estoy parado ni que es todo lo que tengo abierto. Siempre mantenemos la idea minimalista y obviamente sin todo lo que son toolbars, favorites, extensions, etc etc. Muy limpio debe ser." A thin dark top bar (about 28 px) shows the open tabs (the existing tab strip: click to activate, active one highlighted) and minimize, maximize/restore and close buttons, right-aligned on Windows and Linux, left-aligned on macOS (platform convention). No toolbars, bookmarks, extensions or other chrome.
- **S2 — Auto-hide by default, pin by setting.** User chose: "Al pasar el mouse + opción de fijarla" — "Por defecto: aparece al pasar el mouse. Configuración: Padding − 8 px + / Barra superior [Auto | Fija]". By default the bar is hidden and appears when the mouse reaches the top edge of the window, hiding again shortly after the mouse leaves it; while hidden nothing of it shows (screen sharing stays chrome-less). The settings box (`Ctrl/Cmd+,`) gets a "Top bar" choice `Auto | Pinned` that persists in the settings file; `Pinned` keeps it always visible. `Ctrl/Cmd+Shift+F` toggles between the two and is saved the same way. A corrupt or missing value falls back to `Auto`.
- **S3 — Orientation.** "No se donde estoy parado ni que es todo lo que tengo abierto": in Auto mode the bar also shows briefly (about 1.5 s) when the tab set or the active tab changes (`Ctrl+T`, `Ctrl+W`, `Ctrl+Tab`, a popup opening as a tab), so the user sees where they are. When more tabs are open than fit, the bar says how many are hidden (e.g. `+3`).
- **S4 — Native window behavior.** Double-clicking the top frame maximizes/restores (as every OS window); dragging the frame still moves the window and its edges still resize it.
- **S5 — Policy docs.** `AGENTS.md` invariant 4 states the new rule (no permanent chrome by default; the top bar appears on hover or when the user pins it), and the "Only the owner merges" lines in `AGENTS.md` and `README.md` say that external contributions are merged by the owner while the owner's own agents merge their PRs once `ci-gate` is green on the head commit (owner decision 2026-10-22).

## Tasks

- [ ] T1 — S1, S2, S3, S4: top bar with window buttons, hover reveal, pin setting, orientation flash, hidden-tab count, double-click maximize (tests for core logic, Windows functional checks) — worker — commit: —
- [ ] T2 — S5: AGENTS.md and README policy wording — inline — commit: —
- [ ] T3 — S1–S4: independent verify on Windows (no UI tests) — verify — evidence: —
- [ ] T4 — PR(s), ci-gate, merge — inline — merge: —

## Log

- L1 (user, original request): "Bien entonces podemos tratar los pendientes aunque no sean bloqueantes y: 1) Actualmente no se visualizan pestañas abiertas ni manejo de la ventana, quedo todo por comandos de teclados. Debemos tener la posibilidad de miniza maximizar y cerrar tal como todas las ventantanas de todos los S.O. Quiza todo muy fino al maximo de reduccion para respetar la idea de mini, pero tenerlo. Al menos en windows no veo gestion de la ventana ni pestañas dentro del browser. No se donde estoy parado ni que es todo lo que tengo abierto. Siempre mantenemos la idea minimalista y obviamente sin todo lo que son toolbars, favorites, extensions, etc etc. Muy limpio debe ser. 2) Si me estoy confundiendo con algo en esto que te indico que se este alejando de la idea principal marcamelo asi lo discutimos."
- L2 (discussion): a permanent bar conflicts with invariant 4 ("No permanent browser chrome") and the screen-sharing purpose; the discoverability problem is real (tabs hidden behind `Ctrl+Shift+F`, window actions only via OS shortcuts). Proposed a hover-revealed bar.
- L3 (user decision): "Al pasar el mouse + opción de fijarla".
- L4 (pending items folded in): hidden-tab indicator (S3) and the AGENTS.md merge wording (S5) from the v0.2.0 report.
