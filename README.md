# diecut

A single-binary, language-agnostic project template generator — like cookiecutter, but in Rust. No runtime dependencies required.

## Install

```bash
cargo install --path crates/diecut-cli
```

## Quick Start

```bash
# Generate from a local template
diecut new ./my-template --output my-project

# Generate from a GitHub repo
diecut new gh:user/template-repo --output my-project

# Use defaults without prompting
diecut new gh:user/template-repo --defaults --output my-project

# Override specific variables
diecut new ./my-template -d project_name=foo -d license=MIT
```

## Commands

### `diecut new <template> [OPTIONS]`

Generate a new project from a template.

```
Options:
  -o, --output <PATH>       Output directory
  -d, --data <KEY=VALUE>    Override variable values (repeatable)
      --defaults            Use default values without prompting
      --overwrite           Overwrite output directory if it exists
      --no-hooks            Skip running hooks
```

**Template sources:**

| Source | Example |
|--------|---------|
| Local path | `diecut new ./my-template` |
| GitHub | `diecut new gh:user/repo` |
| GitLab | `diecut new gl:user/repo` |
| Bitbucket | `diecut new bb:user/repo` |
| Sourcehut | `diecut new sr:user/repo` |
| Any git URL | `diecut new https://git.example.com/repo.git` |

Git templates are cached locally at `~/.cache/diecut/templates/` (override with `DIECUT_CACHE_DIR`).

### `diecut check [PATH]`

Validate a template directory. Reports format detection, variable definitions, and any warnings/errors.

```bash
diecut check ./my-template
```

### `diecut migrate [PATH] [OPTIONS]`

Migrate a cookiecutter template to native diecut format.

```bash
# Preview what would change
diecut migrate ./cookiecutter-template --dry-run

# Migrate to a new directory
diecut migrate ./cookiecutter-template --output ./diecut-template
```

## Template Format

A diecut template is a directory with a `diecut.toml` config and a `template/` subdirectory:

```
my-template/
  diecut.toml              # Config (required)
  template/                # Template content (required)
    {{project_name}}/
      README.md.tera       # .tera files are rendered, suffix stripped
      src/
        main.rs.tera
      .gitignore            # No .tera suffix = copied verbatim
  hooks/                   # Optional Rhai scripts
    pre_generate.rhai
    post_generate.rhai
```

Files ending in `.tera` are rendered through the [Tera](https://keats.github.io/tera/) template engine (Jinja2-like syntax), then have the suffix stripped. Other files are copied verbatim.

### `diecut.toml`

```toml
[template]
name = "rust-cli"
version = "1.0.0"
description = "A Rust CLI project template"

# Variables are prompted in declaration order
[variables.project_name]
type = "string"
prompt = "Project name"
default = "my-project"
validation = '^[a-z][a-z0-9_-]*$'

[variables.use_ci]
type = "bool"
prompt = "Set up CI?"
default = true

[variables.license]
type = "select"
prompt = "License"
choices = ["MIT", "Apache-2.0", "GPL-3.0"]
default = "MIT"

[variables.features]
type = "multiselect"
prompt = "Features"
choices = ["logging", "docker", "nix"]
default = ["logging"]

# Only asked when use_ci is true
[variables.ci_provider]
type = "select"
prompt = "CI provider"
choices = ["github-actions", "gitlab-ci"]
when = "{{ use_ci }}"

# Computed (never prompted)
[variables.project_slug]
type = "string"
computed = '{{ project_name | slugify }}'

# Conditional file inclusion
[[files.conditional]]
pattern = ".github/**"
when = "{{ use_ci and ci_provider == 'github-actions' }}"

[files]
exclude = ["*.pyc", ".DS_Store"]
copy_without_render = ["assets/**/*.png"]

[hooks]
pre_generate = ["hooks/pre_generate.rhai"]
post_generate = ["hooks/post_generate.rhai"]
```

**Variable types:** `string`, `bool`, `int`, `float`, `select`, `multiselect`

**Special fields:** `when` (conditional), `computed` (derived), `secret` (not saved to answers file), `validation` (regex)

## Cookiecutter Compatibility

diecut auto-detects cookiecutter templates and generates from them directly:

```bash
diecut new gh:audreyfeldroy/cookiecutter-pypackage --output my-package
```

Cookiecutter's `cookiecutter.json` format is translated on-the-fly. For a permanent migration, use `diecut migrate`.

## Hooks

Hooks are written in [Rhai](https://rhai.rs/) — a sandboxed scripting language compiled into the binary. No shell, no runtime dependencies, works identically on all platforms.

```rhai
// hooks/post_generate.rhai
let project = variable("project_name");
print(`Project ${project} created!`);
```

## Development

```bash
# Run tests
cargo test

# Check formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

## License

MIT
