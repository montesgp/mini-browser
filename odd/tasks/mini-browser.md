# mini-browser v1

A minimal, dark, chrome-less Chromium browser for screen sharing, inspired by Guillermo Rauch's "Mini".
Stack: Rust + `cef` crate (tauri-apps/cef-rs, CEF/Chromium 156) + CEF Views. Branch `feat/v1`.

## Specs

- **S1 — No browser chrome.** User: "no tiene barra de navegacion ni toda la "basura" que suele verse en los navegadores" and "la interface bien limpia que es lo que quiero". The window is frameless: no title bar, no toolbar, no visible window buttons. The page fills the window except for the padding frame (S9).
- **S2 — Dark.** User: "me gustaria tener algo asi en mi entorno que sea tipo chrome negro". Window background and every overlay are dark; Chromium runs in its dark color scheme so sites that honor `prefers-color-scheme: dark` render dark.
- **S3 — Typing addresses.** User: "necesito escribir direcciones". `Ctrl+L` (`Cmd+L` on macOS) opens a centered floating box with placeholder `Enter URL or search…` over the current page; `Enter` navigates the current tab and closes the box; `Esc` closes it without navigating. A new empty tab opens with the box shown. Input resolution:
  - `http://` or `https://` input is loaded as typed.
  - A host-like input (no spaces and a dot, `localhost`, or an IP, optional port/path) gets `https://`; `localhost` and IP addresses get `http://`.
  - Anything else, including any other scheme (`javascript:`, `file:`, `data:`, `chrome:`), is sent as a search query to the configured search engine (default Google).
  - Empty or whitespace-only input does nothing.
- **S4 — Browser security.** User: "Obviamente respetando seguridad de los browsers". The Chromium sandbox is enabled on every platform; no remote debugging port; no flag that weakens web security; the profile is isolated from the user's other browsers (own data directory). The docs state the security maintenance cost (bump the `cef` crate on each Chromium security release) and that CEF has no Google Safe Browsing.
- **S5 — Cross-platform.** User: "Me gustaria que sea multi plataforma." One codebase for Windows, macOS and Linux (x64 and ARM64 where cef-rs supports them); no platform-only UI toolkit.
- **S6 — Rauch's tech.** User: "Intentemos imitar las techs". Rust + `cef` crate as in Rauch's port; Swift is replaced by CEF Views (decision L4). The embedded agent (`fx acp` + MCP) is out of v1 (L4).
- **S7 — Credits.** User: "cuando armemos la documentacion, damos los creditos a guillermo rauch y linkeamos no su tiktok o x sino su pagina https://rauchg.com/ para hacerle honores." The README credits Guillermo Rauch with a link to https://rauchg.com/ and no social-media link.
- **S8 — Tabs and focus mode** (from Rauch's demo video, L5). `Ctrl/Cmd+T` opens a new tab with the address box (S3); `Ctrl/Cmd+W` closes the current tab, and closing the last tab closes the window; `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle tabs; `Ctrl/Cmd+Shift+F` toggles a thin dark tab strip (hidden by default) showing each tab's title, the active one highlighted. Pages that open a popup (`window.open`, `target=_blank`) open it as a new tab in the same window.
- **S9 — Padding setting and window handling** (from the demo video, L5). The page is inset by a dark padding frame, default 8 px, range 4–32 px; dragging the frame moves the window. `Ctrl/Cmd+,` opens a small settings box with `−` / `+` buttons for padding; the value persists across runs in a settings file in the app data directory. A corrupt or out-of-range settings file falls back to defaults (out-of-range padding is clamped). `Alt+Left` / `Alt+Right` (`Cmd+[` / `Cmd+]` on macOS) go back/forward; `Ctrl/Cmd+R` reloads; `Ctrl/Cmd+Q` quits.

## Tasks

- [ ] T1 — S1, S2, S4, S6: crate scaffold, sandboxed bundle build, frameless dark window loading a page — inline — commit: —
- [ ] T2 — S3: address-input resolution (core, unit tests) — inline — commit: —
- [ ] T3 — S8: tab model (core, unit tests) — inline — commit: —
- [ ] T4 — S9: settings model and persistence (core, unit tests) — inline — commit: —
- [ ] T5 — S3, S8: shell — address overlay, shortcuts, tabs, tab strip, popups as tabs — inline — commit: —
- [ ] T6 — S9, S1: shell — padding frame, window drag, settings overlay — inline — commit: —
- [ ] T7 — S4, S5, S7: README (credits, build per OS, security and maintenance) — inline — commit: —
- [ ] T8 — S5: CI build matrix (Windows/macOS/Linux) — inline — commit: — (unverifiable until a remote exists)

Forecast: ~1,800 authored lines. Delivery: no remote yet; work-unit commits on `feat/v1`, delivery strategy decided when a remote exists.

## Log

- L1 (user, original request): "Podemos revisar el github de grauch el CEO de Vercel? Una vez lo escuche hablar sobre un navegador propio que se creo si mal no recuerdo se llama minimax que es un navegador tal como chrome, edge y similares simplemente que no tiene barra de navegacion ni toda la "basura" que suele verse en los navegadores. Lo utiliza para compartir en meetings, me gustaria tener algo asi en mi entorno que sea tipo chrome negro. Verifiquemos en https://github.com/rauchg si no lo tiene informame que tan costoso seria realizarlo. O puede ser algo sencillo? Simplemente es para entrar y navegar. Obviamente respetando seguridad de los browsers"
- L2 (evidence): rauchg has 143 public repos; none is a browser. Tweets: Mini v1 was Electron + Bun; v2 is Rust + Swift with the `cef` crate, plus an `fx acp` agent driving the browser over MCP.
- L3 (user): "Entonces hay que hacerlo, necesito escribir direcciones. Me gustaria que sea multi plataforma. [...] Intentemos imitar las techs [...] en todo caso cuando armemos la documentacion, damos los creditos a guillermo rauch y linkeamos no su tiktok o x sino su pagina https://rauchg.com/ para hacerle honores."
- L4 (user decision): stack "Rust + cef con CEF Views (recomendado)" — one cross-platform UI, no Swift/Liquid Glass. Agent/MCP deferred past v1 (parent proposal, not objected).
- L5 (user): "Adjunto el tweet la animacion se ve la interface bien limpia que es lo que quiero". Demo video frames show: frameless rounded window; empty tab with centered "Enter URL or search..." box; ⇧⌘F toggles a thin tab strip (title + URL); several tabs; a Settings box with a "Padding" slider. On-screen key badges belong to a screen-recording tool and are out of scope. Light theme in the video; ours is dark (S2).
- L6 (user decision): project name "mini-browser" at `R:\repositories\personal\mini-browser`.
- L7 (evidence): cef-rs needs cmake (Visual Studio 18 bundles cmake and ninja); with the `sandbox` feature on Windows the app builds as a cdylib loaded by CEF's `bootstrap.exe` via `bundle-cef-app`.
