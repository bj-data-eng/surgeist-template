# surgeist-template

Template and DSL-facing contracts for Surgeist.

This crate owns template-layer contracts for Surgeist. Keep authored template
concepts typed, host-adapter-agnostic, and independent from root facade wiring.
Root `surgeist` owns cross-crate lowering from template-facing output into
style, retained, layout, text, render, window, and task integration.

## Baseline Checks

Run these before handing off crate-local template work:

```sh
cargo test -p surgeist-template
cargo clippy -p surgeist-template --all-targets -- -D warnings
cargo fmt --check
```
