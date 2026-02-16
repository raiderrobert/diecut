---
title: "diecut.toml"
description: "Complete reference for the diecut template configuration file."
---

The `diecut.toml` file at the root of your template directory configures metadata, variables, file handling, hooks, and the answers file.

```toml
[template]
name = "rust-cli"
version = "0.1.0"
description = "A minimal Rust CLI application"

[variables.project_name]
type = "string"
prompt = "Project name"
default = "my-cli"

[variables.license]
type = "select"
prompt = "License"
choices = ["MIT", "Apache-2.0", "MIT OR Apache-2.0"]
default = "MIT"

[files]
exclude = [".git/", "*.swp"]

[hooks]
post_create = "cargo build && git init"

[answers]
file = ".diecut-answers.toml"
```

## Summary

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| **[template]** | | | Template metadata |
| `name` | string | *required* | Template name |
| `version` | string | --- | Template version |
| `description` | string | --- | Short description |
| `min_diecut_version` | string | --- | Minimum diecut version required |
| `templates_suffix` | string | `".tera"` | File suffix that triggers template rendering |
| **[variables.NAME]** | | | Variable definitions |
| `type` | enum | *required* | One of: `string`, `bool`, `int`, `float`, `select`, `multiselect` |
| `prompt` | string | --- | Text shown to the user |
| `default` | varies | --- | Default value |
| `choices` | string[] | --- | Options for select/multiselect (required for those types) |
| `validation` | string | --- | Regex pattern for input validation |
| `validation_message` | string | --- | Message shown when validation fails |
| `when` | string | --- | Tera expression; if false, variable is skipped |
| `computed` | string | --- | Tera expression; value is derived, never prompted |
| `secret` | bool | `false` | If true, value is not saved to answers file |
| **[files]** | | | File handling rules |
| `exclude` | string[] | `[]` | Glob patterns to exclude from output |
| `copy_without_render` | string[] | `[]` | Glob patterns to copy without Tera rendering |
| `conditional` | object[] | `[]` | Conditional file inclusion rules |
| **[files.conditional] items** | | | |
| `pattern` | string | *required* | Glob pattern matching files |
| `when` | string | *required* | Tera expression; if false, matched files are excluded |
| **[hooks]** | | | Hook scripts |
| `post_create` | string | --- | Shell command to run after generation |
| **[answers]** | | | Answers file config |
| `file` | string | `".diecut-answers.toml"` | Filename for answers file in generated project |

## [template]

Template metadata. Only `name` is required.

- **`templates_suffix`** -- Change from `.tera` to something else if you prefer (e.g., `.j2`, `.tmpl`). Files matching this suffix are rendered through the [Tera](https://keats.github.io/tera/) engine; others are copied as-is.
- **`min_diecut_version`** -- If set, diecut will refuse to process the template if the installed version is too old.

## [variables]

Variables are prompted in declaration order. Each variable is a TOML table under `[variables.NAME]`.

Key behaviors:

- `select` and `multiselect` require `choices` to be set.
- `computed` variables must **not** have a `prompt`. They are derived from other variables using [Tera expressions](https://keats.github.io/tera/docs/#expressions).
- `when` controls conditional prompting. Uses [Tera expression](https://keats.github.io/tera/docs/#expressions) syntax (e.g., `"{{ use_ci }}"` or just `"use_ci"`).
- `validation` is a regex pattern. The entire input must match (anchored).
- `secret` variables are prompted but excluded from `.diecut-answers.toml`.
- Variables are available in templates as `{{ variable_name }}`.

```toml
[variables.project_name]
type = "string"
prompt = "Project name"
default = "my-project"
validation = '^[a-z][a-z0-9_-]*$'
validation_message = "Lowercase letters, numbers, hyphens, underscores only"

[variables.use_ci]
type = "bool"
prompt = "Set up CI?"
default = true

[variables.ci_provider]
type = "select"
prompt = "CI provider"
choices = ["github-actions", "gitlab-ci"]
when = "{{ use_ci }}"

[variables.project_slug]
type = "string"
computed = "{{ project_name | slugify }}"
```

## [files]

Control which files are included and how they're processed.

- **`exclude`** -- Glob patterns. Matched files are not written to output. Useful for build artifacts, OS files.
- **`copy_without_render`** -- Glob patterns. Matched files skip [Tera](https://keats.github.io/tera/) rendering and are copied verbatim. Use for binaries, images, or files that contain `{{ }}` syntax that isn't meant for Tera.
- **`conditional`** -- Array of `{ pattern, when }` objects. Files matching `pattern` are included only when `when` evaluates to true.

```toml
[files]
exclude = ["*.pyc", ".DS_Store", "__pycache__/**"]
copy_without_render = ["assets/**/*.png", "fonts/**"]
conditional = [
    { pattern = ".github/**", when = "use_ci and ci_provider == 'github-actions'" },
    { pattern = "src/cli.py*", when = "use_cli" },
]
```

## [hooks]

Shell commands that run after project generation.

- **`post_create`** -- A shell command run in the generated project directory via `sh -c`. Use for installing dependencies, initializing git, or running setup scripts.

See [Hooks reference](/reference/hooks/) for examples.

## [answers]

Controls the answers file written into generated projects.

- **`file`** -- The filename. Default is `.diecut-answers.toml`. Set to `""` to disable.
- The answers file stores the template source, version, and all non-secret variable values.
