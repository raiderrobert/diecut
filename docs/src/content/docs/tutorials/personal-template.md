---
title: Building a personal project template
description: Write your stack conventions down once. Use them everywhere.
---

You start a new Python package roughly the same way every time: same folder layout, same `pyproject.toml` structure, same linting config, same CI workflow. You copy `old-service` to `my-lib`, run a search-and-replace for the project name, and push. Three days later you notice `pyproject.toml` still says `name = 'old-service'` because the old name appeared in a comment you didn't touch. Or the README still references your old author email. You've shipped the wrong metadata again.

This tutorial shows you how to write that pattern down once as a diecut template, test it locally, push it to GitHub, and use it from anywhere.

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

`project_slug` is computed — it's derived from `project_name` with hyphens replaced by underscores. Python package names use underscores; project directory names conventionally use hyphens.

Without this, you'd have to ask for both separately and trust the user types them consistently. If they enter `project_name = my-lib` but `project_slug = my_lib_utils` by mistake, the import in `test_package.py` (`from my_lib_utils import ...`) won't match the directory diecut creates (`src/my_lib/`). The computed variable eliminates that class of mismatch: one value is entered, the other is always derived from it.

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

The thing to verify is that `my_lib` appears, not `my-lib`. If you'd written `computed = '{{ project_name }}'` without the replace filter, the dry-run would show `src/my-lib/__init__.py` — a directory name Python can't import from. Catching that here costs nothing; catching it after `pip install -e .` fails costs you a confused ten minutes.

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

`.diecut-answers.toml` records the variable values used. If a teammate asks which license you picked, or you want to scaffold a closely related second package with the same author and description, the answers are already there — no digging through `pyproject.toml` to reconstruct what you typed.

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

Six months from now, when you start `data-pipeline`, you run one command instead of opening `my-lib` and hunting for every `my-lib`, `my_lib`, and `Jane Doe` that needs changing. Your teammate onboards with `diecut new gh:yourname/templates/python-pkg -o their-tool` and gets `ruff` at line-length 100, `hatchling` as the build backend, and `pytest` in dev dependencies — exactly what you'd have set up for them, without a setup doc, without a call.

---

To learn more about what you can do in a template, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
