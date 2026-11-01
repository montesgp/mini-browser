## What and why

<!-- What does this change, and which issue does it address? Use "Closes #123" if it resolves one. -->

## How it was tested

<!-- Commands you ran and what you checked by hand, on which platform. -->

## Checklist

- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p mini-core` and `cargo test -p minibrowser --lib` pass
- [ ] Logic changes in `crates/core` come with tests
- [ ] `README.md` and `CHANGELOG.md` (`Unreleased`) are updated where behavior changed
- [ ] The Chromium sandbox, profile isolation and address-box scheme rules are untouched (or the change is explained above)
