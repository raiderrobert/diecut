---
title: Building a personal project template
description: Write your stack conventions down once. Use them everywhere.
---

You start a new Python package roughly the same way every time: same folder layout, same `pyproject.toml` structure, same linting config, same CI workflow. You either copy an old project and scrub it clean, or prompt an LLM and get subtly different results each time. This tutorial shows you how to write that pattern down once as a diecut template, test it locally, push it to GitHub, and use it from anywhere.

## Set up the template directory

Create a directory for your template. If you plan to store multiple templates in one repo, put this in a subdirectory.

```text
templates/
  python-pkg/
    diecut.toml
    template/
```

Everything under `template/` becomes your generated project. Files ending in `.tera` are rendered through the Tera template engine and have the suffix stripped. Everything else is copied as-is.

## Write the config

Create `templates/python-pkg/diecut.toml`:

```toml
[template]
name = "python-pkg"

[variables.project_name]
type = "string"
prompt = "Project name"
default = "my-package"
validation = '^[a-z][a-z0-9-]*$'
validation_message = "Must start with a letter. Only lowercase letters, numbers, hyphens."

[variables.project_slug]
type = "string"
computed = "{{ project_name | replace(from='-', to='_') }}"

[variables.author]
type = "string"
prompt = "Author name"

[variables.description]
type = "string"
prompt = "Short description"
default = ""

[variables.license]
type = "select"
prompt = "License"
choices = ["MIT", "Apache-2.0", "GPL-3.0"]
default = "MIT"
```

`project_slug` is computed — it's derived from `project_name` with hyphens replaced by underscores. Python package names use underscores; project directory names conventionally use hyphens. The user only types one; diecut figures out the other. Computed variables are never shown as a prompt.

## Add the template files

### pyproject.toml

Create `templates/python-pkg/template/pyproject.toml.tera`:

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "{{ project_name }}"
version = "0.1.0"
description = "{{ description }}"
authors = [{ name = "{{ author }}" }]
license = { text = "{{ license }}" }
requires-python = ">=3.11"
dependencies = []

[project.optional-dependencies]
dev = ["pytest", "ruff"]

[tool.ruff]
line-length = 100
```

### Source package

Create `templates/python-pkg/template/src/{{ project_slug }}/__init__.py.tera`:

```python
"""{{ description }}"""

__version__ = "0.1.0"
```

The directory name `{{ project_slug }}` is also rendered by diecut — path components can contain Tera expressions.

### README

Create `templates/python-pkg/template/README.md.tera`:

````markdown
# {{ project_name }}

{{ description }}

## Installation

```bash
pip install {{ project_name }}
```

## License

{{ license }}
````

### Tests

Create `templates/python-pkg/template/tests/test_package.py.tera`:

```python
from {{ project_slug }} import __version__


def test_version():
    assert __version__ == "0.1.0"
```

Your template directory now looks like this:

```text
templates/python-pkg/
  diecut.toml
  template/
    pyproject.toml.tera
    README.md.tera
    src/
      {{ project_slug }}/
        __init__.py.tera
    tests/
      test_package.py.tera
```

## Test it locally

Before pushing anything, preview the output with `--dry-run --verbose`:

```bash
diecut new ./templates/python-pkg -o my-lib --dry-run --verbose
```

diecut prompts you normally, then prints each file it would write without touching the filesystem:

```text
Project name [my-package]: my-lib
Author name: Jane Doe
Short description: A small utility library.
License [MIT]:
  1. MIT
  2. Apache-2.0
  3. GPL-3.0

[dry-run] would write: my-lib/pyproject.toml
[dry-run] would write: my-lib/README.md
[dry-run] would write: my-lib/src/my_lib/__init__.py
[dry-run] would write: my-lib/tests/test_package.py
```

Check that filenames look right — especially the `my_lib` directory name, which is the computed slug. If anything looks off, fix the template and re-run. No cleanup needed.

Once you're satisfied, generate for real:

```bash
diecut new ./templates/python-pkg -o my-lib
```

The output directory:

```text
my-lib/
  pyproject.toml
  README.md
  src/
    my_lib/
      __init__.py
  tests/
    test_package.py
  .diecut-answers.toml
```

`.diecut-answers.toml` records the variable values used. You can regenerate or inspect later.

## Push to GitHub and use from anywhere

Commit the template directory to a GitHub repo. The structure can be a dedicated templates repo or a subdirectory inside an existing one:

```bash
git add templates/python-pkg
git commit -m "add python-pkg template"
git push
```

Now use it from any machine:

```bash
diecut new gh:yourname/templates/python-pkg -o my-lib
```

diecut fetches the repo, reads the template from the `python-pkg` subdirectory, and prompts as usual. Skip prompts entirely with `--defaults`, or override specific values inline:

```bash
diecut new gh:yourname/templates/python-pkg -o my-lib \
  -d project_name=my-lib \
  -d author="Jane Doe" \
  --defaults
```

## The payoff

Your conventions are now written down. The folder layout, the `pyproject.toml` structure, the linting config, the test setup — it's all in one place, version-controlled, and reproducible. Share the repo link with a teammate and they get the same starting point. Come back six months later and you're not reverse-engineering what you did last time.

---

To learn more about what you can do in a template, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
