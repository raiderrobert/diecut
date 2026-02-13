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

## Incompatibilities

These are the things that won't work, or won't work the way you expect. No surprises.

### Won't work at all

**Python hooks** -- Cookiecutter hooks are Python scripts (`hooks/pre_gen_project.py`, `hooks/post_gen_project.py`). diecut uses [Rhai](/reference/hooks/), a sandboxed scripting language compiled into the binary. No auto-conversion is possible. You have to rewrite them by hand.

**Jinja2 extensions** -- Cookiecutter supports custom Jinja2 extensions via `_extensions`. diecut ignores them with a warning. Most common extension functionality (slugify, random strings) is available as built-in [Tera filters](https://keats.github.io/tera/docs/#built-in-filters).

**Dict/object values** -- If your `cookiecutter.json` has nested objects as variable values, diecut doesn't know what to do with them. They get flattened to a JSON string and you get a warning. You'll need to restructure these as separate variables.

### Tera is not Jinja2

This is the big one. Cookiecutter uses Jinja2. diecut uses [Tera](https://keats.github.io/tera/docs/). They look similar but they're different template engines.

diecut auto-rewrites `.replace()` calls:

```text
# Jinja2 (cookiecutter)
{{ cookiecutter.name.replace('-', '_') }}
{{ cookiecutter.name | replace(' ', '-') }}

# Tera (what diecut rewrites it to)
{{ name | replace(from="-", to="_") }}
{{ name | replace(from=" ", to="-") }}
```

**Other Jinja2 methods are NOT rewritten.** If your templates use any of these, you'll need to fix them manually:

| Jinja2 | Tera equivalent |
|---|---|
| `var.lower()` | `var \| lower` |
| `var.upper()` | `var \| upper` |
| `var.strip()` | `var \| trim` |
| `var.title()` | `var \| title` |
| `var.startswith('x')` | `var is starting_with("x")` |
| `var.endswith('x')` | `var is ending_with("x")` |
| `var.split('x')` | No direct equivalent |
| `~` (string concatenation) | `~` (Tera supports this too) |
| `{% set x = ... %}` | `{% set x = ... %}` (same) |
| `loop.index0` | `loop.index0` (same) |

Most simple templates (`{{ var }}`, `{% if var %}`, `{% for x in list %}`) work identically. The problems show up with string manipulation methods and less common Jinja2 features.

When in doubt, check the [Tera documentation](https://keats.github.io/tera/docs/).

### Partially handled

**`_copy_without_render`** -- Translated to diecut's `[files] copy_without_render`, but cookiecutter and diecut use slightly different glob implementations. Review your patterns after migration. A `--dry-run` will show you what gets matched.

**Single-underscore private keys** -- Keys like `_internal_value` in `cookiecutter.json` are silently dropped. These are cookiecutter-internal variables not meant for templates. If you were actually using them in templates, you'll need to re-add them as regular variables in `diecut.toml`.

### Behavioral differences

**File rendering** -- Cookiecutter renders every file through Jinja2. diecut only renders files ending in `.tera`. When diecut runs a cookiecutter template in compatibility mode, it renders all files to match cookiecutter's behavior. But after `diecut migrate`, files are no longer auto-rendered. You need to add the `.tera` extension to any file that contains template syntax. The migration command does this for you for files it detects template syntax in, but double-check the results.

**Variable namespace** -- In cookiecutter templates, variables are referenced as `cookiecutter.project_name`. When using a cookiecutter template directly (`diecut new`), diecut preserves this namespace so existing templates work. After `diecut migrate`, references are rewritten to just `project_name`. If you have template syntax in places the migration doesn't scan (like generated scripts or deeply nested files), you may need to fix leftover `cookiecutter.` references manually.
