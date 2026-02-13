---
title: Migrating from Cookiecutter
description: Convert cookiecutter templates to native diecut format.
---

## You might not need to migrate

diecut auto-detects cookiecutter templates. If you just want to **use** a cookiecutter template:

```bash
diecut new gh:audreyfeldroy/cookiecutter-pypackage -o my-package
```

diecut finds `cookiecutter.json`, translates the format on the fly, and prompts you as usual. Migration is only for template authors who want to convert to native diecut format permanently.

## Preview first

Always dry-run before migrating:

```bash
diecut migrate ./my-cookiecutter-template --dry-run
```

This shows what would change without writing anything.

## Migrate

Two options:

```bash
# To a new directory (safer)
diecut migrate ./my-cookiecutter-template --output ./my-diecut-template

# In-place
diecut migrate ./my-cookiecutter-template
```

## What gets translated

| Cookiecutter | diecut |
|---|---|
| `cookiecutter.json` | `diecut.toml` |
| `{{cookiecutter.var}}` in filenames and content | `{{var}}` |
| Array values in JSON (choices) | `type = "select"` with `choices` |
| Boolean-like strings (`"n"`, `"y"`) | `type = "bool"` |
| Plain string values | `type = "string"` with `default` |

### Example

A `cookiecutter.json`:

```json
{
    "project_name": "my-project",
    "license": ["MIT", "Apache-2.0", "GPL-3.0"],
    "use_docker": "n"
}
```

Becomes this `diecut.toml`:

```toml
[template]
name = "my-project"

[variables.project_name]
type = "string"
default = "my-project"

[variables.license]
type = "select"
choices = ["MIT", "Apache-2.0", "GPL-3.0"]
default = "MIT"

[variables.use_docker]
type = "bool"
default = false
```

## What doesn't translate

**Python hooks** -- Cookiecutter hooks are Python scripts (`hooks/pre_gen_project.py`, `hooks/post_gen_project.py`). These cannot be auto-converted to Rhai. You need to rewrite them. See the [Hooks reference](/reference/hooks/) for Rhai documentation.

**Jinja2 extensions** -- Cookiecutter supports custom Jinja2 extensions via `cookiecutter.extensions`. diecut has no equivalent. Most common extension functionality (slugify, random strings) is available as built-in Tera filters.

**`_copy_without_render`** -- Cookiecutter's mechanism for skipping template rendering maps to diecut's `[files] copy_without_render`, but may need manual adjustment of glob patterns.
