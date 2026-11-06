# mini-browser local files

Let the user open local files (typed, passed with `--url`, or dropped on the window) while web pages still cannot reach local files and every other non-web scheme stays a search. This relaxes AGENTS.md invariant 2 on the owner's decision.

## Specs

- **S1 — Typed local files.** User: "Me pregunto, que pasa cuando quiera abrir por ejemplo algun archivo local con este browser? Podria? Ejemplo quiero abrir file:///C:/Users/patri/Downloads/ruido_visual_navegador.html". User chose: "Sí, y también arrastrar archivos a la ventana" — "Lo anterior más soltar un .html/.pdf/imagen sobre la ventana para abrirlo en la pestaña." (where "lo anterior" is: "Lo que el usuario escribe explícitamente (file:///..., C:\..., ~/...) o pasa con --url abre el archivo. Páginas web siguen sin poder abrir archivos; javascript:/data:/chrome: siguen siendo búsqueda. Se actualiza el invariante 2."). In the address box, the home page field and `--url` (first launch and relaunch), `file:///…` URLs and absolute local paths (`C:\…` or `C:/…` on Windows, `/…` on macOS/Linux, `~/…` expanded to the user's home) open that file. Paths are converted to `file:///` URLs with proper percent-encoding (spaces, `#`, `%`, non-ASCII).
- **S2 — Drop a file on the window.** Dropping a local file (e.g. `.html`, `.pdf`, image) on the window opens it in the current tab.
- **S3 — Safety limits (parent decision, security).** Only local files: UNC paths (`\\server\share\…`) and `file://host/…` with a non-empty, non-`localhost` host stay a search and are never loaded (on Windows they can send the user's credentials to that host). Web pages still cannot navigate, redirect or link to `file:` (Chromium's own rule must stay in force; add an app check only if needed). `javascript:`, `data:`, `chrome:`, `mini:` and every other non-web scheme stay a search. The `mini://home` bridge keeps refusing requests from any non-home page, including `file:` pages. Local pages keep Chromium's default file restrictions (no `allow_file_access_from_file_urls`).
- **S4 — Docs.** AGENTS.md invariant 2 states the new rule; README (For humans: opening local files; Address resolution; Security model) and CHANGELOG `## [Unreleased]`.

## Tasks

- [ ] T1 — S1–S4: local file resolution (core, tests first), drop handling, navigation policy, docs — worker — commit: —
- [ ] T2 — S1–S3: independent verify on Windows incl. security probes — verify — evidence: —
- [ ] T3 — native review (security), PR, ci-gate, merge — inline — merge: —

## Log

- L1 (user, original request): "No habria problema con eso, para el objetivo que tenemos es suficiente como esta. Me pregunto, que pasa cuando quiera abrir por ejemplo algun archivo local con este browser? Podria? Ejemplo quiero abrir file:///C:/Users/patri/Downloads/ruido_visual_navegador.html"
- L2 (context): today the address box, the home field and `--url` only load http(s); `file:` becomes a search (AGENTS.md invariant 2, `mini_core::address::resolve_input`).
- L3 (user decision): "Sí, y también arrastrar archivos a la ventana".
- L4 (parent decision): UNC and remote-host `file:` URLs are refused (NTLM credential leak risk on Windows); only local drives/paths open.
- L5 (carry-over): the settings box `×` fix (180fd7f on `fix/settings-close-visible`) waits for a runtime check; the promo video (D:\personal\projects\mini-browser\mini-browser-promo-1080p30.mp4) is accepted as is.
