# Contributing to mini-browser

Thanks for helping. mini-browser is small on purpose, so the best contributions keep it minimal, fast and safe.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## How the project is run

- The owner, [@montesgp](https://github.com/montesgp), reviews and merges every external contribution. Nobody else has write access.
- Everyone else contributes through **issues** and **pull requests from a fork**.
- Work goes into `dev`; `main` holds released code and is what GitHub Pages and releases come from. Both branches are protected: changes land only through pull requests whose `ci-gate` check is green.
- Workflows from forks run after the owner approves them, so your first CI run may wait for a moment.

## Reporting a bug or asking for a feature

1. Search the [existing issues](https://github.com/montesgp/mini-browser/issues) first.
2. Open a new issue with the **Bug report** or **Feature request** template and fill in every section.
3. Security problems are different: never open a public issue for them. Follow [SECURITY.md](SECURITY.md).

Feature requests that add browser "chrome" (toolbars, menus, always-visible UI) go against the project's goal and will usually be declined; shortcuts and opt-in settings are welcome.

## Sending a pull request

1. For anything bigger than a small fix, open an issue first so we can agree on the approach.
2. Fork the repository and create a branch from `dev` (for example `fix/tab-strip-height`).
3. Make your change, with tests where the logic allows it (see below), and update the docs it affects (`README.md`, `CHANGELOG.md` under `Unreleased`).
4. Run the checks locally:

   ```sh
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test -p mini-core
   cargo test -p minibrowser --lib
   ```

5. Open a pull request against `dev` and fill in the template. Keep it focused: one change per PR, ideally under about 400 changed lines.
6. The owner reviews it. Address the feedback with new commits; the owner merges once it is approved and `ci-gate` is green.
7. When `dev` is ready to ship, the owner promotes it to `main` with a pull request from `dev`.

```
feature branch ──► dev ──► main (GitHub Pages, tagged vX.Y.Z)
```

Build prerequisites and commands are in the README's [For agents and contributors](README.md#for-agents-and-contributors) part.

## Code guidelines

- **Platform-independent logic goes in `crates/core` with unit tests.** Address parsing, tab state, settings and the shortcut table live there and are tested with plain `cargo test`. The CEF shell in `crates/app` stays thin.
- **Security is not negotiable.** Do not disable the Chromium sandbox, add debugging ports, weaken web security, or let the address box load schemes other than `http(s)`.
- **One codebase for every platform.** Platform-specific code is fine behind `cfg`, but no platform-only UI toolkit.
- English for code, comments, commit messages and docs.
- Match the surrounding style; `cargo fmt` and `clippy -D warnings` must pass.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `ci:`, `refactor:`, `test:`, `chore:`, with an optional scope such as `feat(app):` or `fix(core):`. Explain *why* in the body when it is not obvious.

## License of contributions

mini-browser is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion is dual-licensed the same way, without any additional terms or conditions.
