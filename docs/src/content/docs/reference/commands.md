---
title: Commands
description: Complete reference for all diecut CLI commands.
---

## diecut new

Generate a new project from a template.

### Synopsis

```bash
diecut new <TEMPLATE> [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `<TEMPLATE>` | — | Template source: local path, `gh:user/repo`, `gl:user/repo`, `bb:user/repo`, `sr:user/repo`, or any Git URL |
| `-o, --output <PATH>` | — | Output directory |
| `-d, --data <KEY=VALUE>` | — | Override variable values (repeatable) |
| `--defaults` | `false` | Use default values without prompting |
| `--overwrite` | `false` | Overwrite output directory if it exists |
| `--no-hooks` | `false` | Skip running hooks |

### Examples

```bash
# From a local template
diecut new ./my-template --output my-project

# From a GitHub shorthand
diecut new gh:user/template-repo --output my-project

# Use defaults without prompting
diecut new gh:user/repo --defaults --output my-project

# Override specific variables
diecut new ./my-template -d project_name=foo -d license=MIT
```

### Notes

- Git-hosted templates are cloned and cached locally on first use.
- The `--data` flag can be repeated to set multiple variables.
- When `--defaults` is set, any variable without a default value causes an error.

---

## diecut list

List cached templates.

### Synopsis

```bash
diecut list
```

### Options

None.

### Examples

```bash
diecut list
```

### Notes

- Templates are cached at `~/.cache/diecut/templates/`. Override with the `DIECUT_CACHE_DIR` environment variable.

---

## diecut update

Update a previously generated project from its template.

### Synopsis

```bash
diecut update [PATH] [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `[PATH]` | `.` | Path to the project |
| `--ref <REF>` | latest | Git ref (branch, tag, commit) to update to |

### Examples

```bash
# Update the project in the current directory
diecut update

# Update a project at a specific path
diecut update ./my-project

# Pin the update to a tag
diecut update ./my-project --ref v2.0.0
```

### Notes

- Requires a `.diecut-answers.toml` file in the project directory. This file is written during `diecut new` and records the template source and variable values.

---

## diecut check

Validate a template directory.

### Synopsis

```bash
diecut check [PATH]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `[PATH]` | `.` | Path to the template |

### Examples

```bash
diecut check ./my-template
```

### Notes

- Exits with code 1 if errors are found.
- Reports format detection, variable definitions, and any warnings.

---

## diecut ready

Check if a template is ready for distribution.

### Synopsis

```bash
diecut ready [PATH]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `[PATH]` | `.` | Path to the template |

### Examples

```bash
diecut ready ./my-template
```

### Notes

- Stricter than `check`. Includes additional distribution-specific validations.
- Exits with code 1 if issues are found.

---

## diecut migrate

Convert a cookiecutter template to native diecut format.

### Synopsis

```bash
diecut migrate <PATH> [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `<PATH>` | — | Path to the cookiecutter template |
| `-o, --output <DIR>` | — | Write to a new directory instead of migrating in-place |
| `--dry-run` | `false` | Show planned changes without writing |

### Examples

```bash
# Preview what would change
diecut migrate ./cookiecutter-template --dry-run

# Migrate to a new directory
diecut migrate ./cookiecutter-template --output ./diecut-template

# Migrate in-place
diecut migrate ./cookiecutter-template
```

### Notes

- Use `--dry-run` first to review planned changes before writing.
- Without `--output`, the template is converted in-place.
