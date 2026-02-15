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
| `<TEMPLATE>` | — | Template source: local path, `gh:user/repo`, `gl:user/repo`, any Git URL, or abbreviation with subpath (`gh:user/repo/subdir`) |
| `-o, --output <PATH>` | — | Output directory |
| `-d, --data <KEY=VALUE>` | — | Override variable values (repeatable) |
| `--defaults` | `false` | Use default values without prompting |
| `--overwrite` | `false` | Overwrite output directory if it exists |
| `--no-hooks` | `false` | Skip running hooks |
| `--dry-run` | `false` | Show what would be generated without writing files |
| `-v, --verbose` | `false` | Show file contents (with `--dry-run`) or detailed output |

### Examples

```bash
# From a multi-template repo (subpath)
diecut new gh:raiderrobert/diecut-templates/rust-cli --output my-project

# From a local template
diecut new ./my-template --output my-project

# Use defaults without prompting
diecut new gh:user/repo --defaults --output my-project

# Override specific variables
diecut new ./my-template -d project_name=foo -d license=MIT

# Preview what would be generated
diecut new ./my-template --dry-run --verbose
```

### Notes

- Git-hosted templates are cloned and cached locally on first use.
- The `--data` flag can be repeated to set multiple variables.
- When `--defaults` is set, any variable without a default value causes an error.
- Subpaths let you point to a template inside a larger repo (e.g., `gh:user/templates/python-pkg`).

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
