<p align="center">
  <img src="img/diecut-logo.svg" alt="diecut logo" width="120">
</p>

<h1 align="center">diecut</h1>

<p align="center">A project template generator, written in Rust.</p>

- **Single binary.** No runtime dependencies. Download one file and go.
- **Easy to make.** A `diecut.toml` and a folder, and that's a template.
- **Multi-template repos.** One repo, many templates. Use subpaths to pick the one you want.
- **Any Git host.** GitHub, GitLab, Codeberg, or any Git URL.
- **Interactive prompts.** Variables with types, defaults, validation, and computed values.
- **Conditional files.** Include or exclude files based on user choices.
- **Post-generation hooks.** Run shell commands after generation (`git init`, `cargo build`, etc.).
- **Template caching.** Fetched templates are cached locally for instant re-use.

## Install

```bash
curl -fsSL https://diecut.dev/install.sh | sh
```

Or build from source:

```bash
cargo install --path .
```

Or grab a binary from [GitHub Releases](https://github.com/raiderrobert/diecut/releases).

## Quick Start

```bash
diecut new gh:raiderrobert/diecut-templates/rust-cli --output my-project
```

Diecut prompts you for variables and generates a ready-to-go project.

```bash
# Use defaults without prompting
diecut new gh:raiderrobert/diecut-templates/python-pkg --defaults --output my-project

# Generate from a local template
diecut new ./my-template --output my-project

# List cached templates
diecut list
```

Example templates: [diecut-templates](https://github.com/raiderrobert/diecut-templates)

### Protocol

Built-in shortcodes resolve to SSH URLs by default (e.g., `gh:user/repo` →
`git@github.com:user/repo.git`). To use HTTPS instead, pass `--protocol https`
or set `DIECUT_GIT_PROTOCOL=https` in your shell environment.

## Documentation

Full documentation: **[diecut.dev](https://diecut.dev/)**

- [Getting Started](https://diecut.dev/getting-started/): Install and generate your first project
- [Using Templates](https://diecut.dev/using-templates/): Sources, overrides, abbreviations
- [Creating Templates](https://diecut.dev/creating-templates/): Build your own templates
- [Commands Reference](https://diecut.dev/reference/commands/): All CLI commands and options
- [diecut.toml Reference](https://diecut.dev/reference/diecut-toml/): Complete config file reference

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and workflow.

## License

[PolyForm Shield 1.0.0](LICENSE)
