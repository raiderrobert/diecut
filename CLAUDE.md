# Diecut

Rust CLI tool for generating projects from templates (like cookiecutter, but in Rust).

## Project Structure

Single crate with both library (`src/lib.rs`) and binary (`src/main.rs`) targets.
- `src/lib.rs` — library root (template resolution, rendering, caching, hooks)
- `src/main.rs` — CLI entry point using clap
- `src/commands/` — CLI subcommand handlers
- `tests/` — integration tests and fixtures

## Pre-Commit Validation

Run these checks before every commit. Fix failures before committing.

```
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Conventions

- Use `thiserror` for error types in `src/error.rs`
- `error.rs` has `#![allow(unused_assignments)]` — this is a known thiserror/clippy false positive, do not remove
- Template caching lives in `src/template/cache.rs`, cloning in `clone.rs`
- XDG-compliant cache dir: `~/.cache/diecut/templates/`, overridable via `DIECUT_CACHE_DIR`
