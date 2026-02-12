# rust-cli

A diecut template for generating a minimal Rust CLI application using clap.

## Usage

```bash
diecut new ./examples/rust-cli -o my-project
```

## Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `project_name` | string | `my-cli` | Project name |
| `description` | string | `A command-line application` | Short description |
| `author` | string | | Author name |
| `license` | select | `MIT` | License (MIT, Apache-2.0, or dual) |
| `rust_edition` | select | `2021` | Rust edition (2021 or 2024) |
