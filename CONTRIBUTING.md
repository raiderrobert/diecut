# Contributing to diecut

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [just](https://github.com/casey/just) task runner

## Development

All tasks are managed through the justfile. Run `just` to see available recipes:

```bash
just          # list all recipes
just check    # run fmt, clippy, and tests
just test     # run tests (supports extra args: just test -- --nocapture)
just build    # build release binary
just fmt      # auto-format code
just install  # install diecut locally
```

## Before Submitting a PR

Run the full check suite:

```bash
just check
```

This runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.

## Docs Site

The documentation site lives in `docs/` and uses pnpm:

```bash
just docs       # start dev server
just docs-build # build site
```
