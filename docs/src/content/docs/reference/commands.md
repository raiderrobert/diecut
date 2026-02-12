---
title: Commands
description: Complete reference for all diecut CLI commands.
---

## diecut new

Generate a new project from a template.

```bash
diecut new <TEMPLATE> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<TEMPLATE>` | Template source — local path, Git URL, or abbreviation (`gh:`, `gl:`, `bb:`, `sr:`) |

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <PATH>` | Output directory |
| `-d, --data <KEY=VALUE>` | Override variable values (repeatable) |
| `--defaults` | Use default values without prompting |
| `--overwrite` | Overwrite output directory if it exists |
| `--no-hooks` | Skip running hooks |

### Examples

```bash
# Local template
diecut new ./my-template --output my-project

# GitHub shorthand
diecut new gh:user/template-repo --output my-project

# Skip prompts with defaults
diecut new gh:user/repo --defaults --output my-project

# Override specific variables
diecut new ./my-template -d project_name=foo -d license=MIT
```

---

## diecut check

Validate a template directory. Reports format detection, variable definitions, and any warnings or errors.

```bash
diecut check [PATH]
```

Exits with code 1 if errors are found.

### Examples

```bash
diecut check ./my-template
```

---

## diecut ready

Check if a template is ready for distribution. Validates template structure and provides distribution-specific warnings.

```bash
diecut ready [PATH]
```

Exits with code 1 if issues are found that would prevent distribution.

---

## diecut update

Update a previously generated project when the upstream template has changed. Reads `.diecut-answers.toml` from the project to recover original template choices, then performs a three-way merge.

```bash
diecut update <PATH> [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--ref <TAG>` | Update to a specific Git ref (tag, branch, or commit) |

### How it works

1. Reads `.diecut-answers.toml` to find the original template source and variables
2. Re-renders the template at the original ref (old snapshot)
3. Re-renders at the new ref (new snapshot)
4. Three-way merges against your actual files
5. Reports files updated, added, removed, and any conflicts (saved as `.rej` files)

---

## diecut migrate

Convert a cookiecutter template to native diecut format.

```bash
diecut migrate <PATH> [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--output <DIR>` | Write to a new directory instead of migrating in-place |
| `--dry-run` | Show what would change without writing |

### Examples

```bash
# Preview changes
diecut migrate ./cookiecutter-template --dry-run

# Migrate to a new directory
diecut migrate ./cookiecutter-template --output ./diecut-template
```

---

## diecut list

List all cached templates.

```bash
diecut list
```

Templates cloned from Git are cached at `~/.cache/diecut/templates/` (overridable via `DIECUT_CACHE_DIR`).
