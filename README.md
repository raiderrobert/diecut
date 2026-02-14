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
# Generate from a local template
diecut new ./my-template --output my-project

# Generate from a GitHub repo
diecut new gh:user/template-repo --output my-project

# Use defaults without prompting
diecut new gh:user/template-repo --defaults --output my-project
```

## Documentation

Full documentation: **[diecut docs](https://diecut.dev/)**

- [Getting Started](https://diecut.dev/getting-started/) — install and generate your first project
- [Using Templates](https://diecut.dev/using-templates/) — sources, overrides, updates, cookiecutter compatibility
- [Creating Templates](https://diecut.dev/creating-templates/) — build your own templates
- [Commands Reference](https://diecut.dev/reference/commands/) — all CLI commands and options
- [diecut.toml Reference](https://diecut.dev/diecut/reference/diecut-toml/) — complete config file reference
- [Hooks Reference](https://diecut.dev/diecut/reference/hooks/) — Rhai scripting for templates

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and workflow.

## License

[MIT](LICENSE)
