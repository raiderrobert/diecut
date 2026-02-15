---
title: Hooks
description: Reference for shell hooks in diecut templates.
---

## What are hooks?

Hooks are shell commands that run after project generation. Configure them in `diecut.toml`.

## Setup

```toml
[hooks]
post_create = "npm install"
```

The command runs in the generated project directory via `sh -c`.

## Examples

### Install dependencies

```toml
[hooks]
post_create = "pip install -e ."
```

### Initialize a git repo

```toml
[hooks]
post_create = "git init && git add -A && git commit -m 'Initial commit'"
```

### Run a setup script

```toml
[hooks]
post_create = "./setup.sh"
```

The script must be in the template's `template/` directory and will be copied to the output before the hook runs.

## Security

When generating from a remote template that contains hooks, diecut prints a warning:

```text
warning: This template contains hooks that will execute code on your machine
  source: https://github.com/user/repo.git
  use --no-hooks to skip hook execution
```

Use `--no-hooks` to skip hook execution for untrusted templates.
