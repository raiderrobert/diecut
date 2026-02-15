# diecut

A project template generator written in Rust.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/raiderrobert/diecut/main/install.sh | sh
```

Or build from source:

```bash
cargo install --path .
```

Or grab a binary from [GitHub Releases](https://github.com/raiderrobert/diecut/releases).

## Quick Start

```bash
# Generate from a GitHub repo (with subpath for multi-template repos)
diecut new gh:raiderrobert/diecut-templates/rust-cli --output my-project

# Use defaults without prompting
diecut new gh:raiderrobert/diecut-templates/python-pkg --defaults --output my-project

# Generate from a local template
diecut new ./my-template --output my-project

# List cached templates
diecut list
```

### Starter templates

[diecut-templates](https://github.com/raiderrobert/diecut-templates) has ready-to-use templates:

```bash
diecut new gh:raiderrobert/diecut-templates/python-pkg --output my-project
```

## Documentation

Full documentation: **[diecut docs](https://diecut.dev/)**

- [Getting Started](https://diecut.dev/getting-started/) — install and generate your first project
- [Using Templates](https://diecut.dev/using-templates/) — sources, overrides, abbreviations
- [Creating Templates](https://diecut.dev/creating-templates/) — build your own templates
- [Commands Reference](https://diecut.dev/reference/commands/) — all CLI commands and options
- [diecut.toml Reference](https://diecut.dev/reference/diecut-toml/) — complete config file reference

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and workflow.

## License

[MIT](LICENSE)
