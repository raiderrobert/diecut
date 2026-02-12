# Diecut

Rust CLI tool for generating projects from templates (like cookiecutter, but in Rust).

## Project Structure

Two-crate workspace:
- `crates/diecut-core` — library crate (template resolution, rendering, caching, hooks)
- `crates/diecut-cli` — thin CLI wrapper using clap

## Pre-Commit Validation

Run these checks before every commit. Fix failures before committing.

```
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Conventions

- Use `thiserror` for error types in `diecut-core/src/error.rs`
- `error.rs` has `#![allow(unused_assignments)]` — this is a known thiserror/clippy false positive, do not remove
- Template caching lives in `diecut-core/src/template/cache.rs`, cloning in `clone.rs`
- XDG-compliant cache dir: `~/.cache/diecut/templates/`, overridable via `DIECUT_CACHE_DIR`
