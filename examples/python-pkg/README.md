# python-pkg

A diecut template for generating a Python package with pyproject.toml (hatchling).

## Usage

```bash
diecut new ./examples/python-pkg -o my-project
```

## Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `project_name` | string | `my-package` | Project name |
| `description` | string | `A Python package` | Short description |
| `author` | string | | Author name |
| `python_version` | select | `3.12` | Minimum Python version |
| `use_cli` | bool | `false` | Include CLI entry point |
