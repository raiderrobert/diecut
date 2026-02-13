---
title: Hooks
description: Reference for Rhai hooks in diecut templates.
---

## What are hooks?

Hooks are scripts that run during project generation. They're written in [Rhai](https://rhai.rs/), a sandboxed scripting language compiled into the diecut binary. No shell, no external dependencies, identical behavior on every platform.

Two hook points:

- **`pre_generate`** — runs before files are rendered
- **`post_generate`** — runs after files are written

## Setup

Configure hooks in `diecut.toml`:

```toml
[hooks]
pre_generate = ["hooks/pre_generate.rhai"]
post_generate = ["hooks/post_generate.rhai"]
```

Paths are relative to the template root. A typical file structure looks like this:

```
my-template/
  diecut.toml
  hooks/
    pre_generate.rhai
    post_generate.rhai
  template/
    ...
```

## Available context

All template variables are injected into the Rhai scope as variables:

```rhai
// If your diecut.toml has [variables.project_name]
print(`Project name: ${project_name}`);
```

Post-generate hooks also get `output_dir`:

```rhai
print(`Files written to: ${output_dir}`);
```

### Type mapping

| diecut type   | Rhai type              |
|---------------|------------------------|
| `string`      | `String`               |
| `bool`        | `bool`                 |
| `int`         | `i64`                  |
| `float`       | `f64`                  |
| `select`      | `String`               |
| `multiselect` | `String` (serialized)  |

## Examples

### Print a welcome message

```rhai
// hooks/post_generate.rhai
print(`Created ${project_name} at ${output_dir}`);
print("Next: cd into the directory and run `cargo build`");
```

### Validate input

```rhai
// hooks/pre_generate.rhai
if project_name.len() < 2 {
    throw "Project name must be at least 2 characters";
}
```

`throw` in a pre-generate hook aborts generation with an error message.

### Conditional logic

```rhai
// hooks/post_generate.rhai
if use_ci {
    print(`CI configured for ${ci_provider}`);
} else {
    print("No CI configured. You can add it later.");
}
```

## Sandboxing and limits

Rhai is sandboxed for security. Hooks **cannot**:

- Access the filesystem directly
- Make network requests
- Execute shell commands
- Access environment variables

Safety limits:

| Limit            | Value      |
|------------------|------------|
| Max call depth   | 32 levels  |
| Max operations   | 100,000    |
| Max string size  | 10 MB      |

If a hook exceeds these limits, it fails with an error. These limits exist to prevent runaway scripts from untrusted templates.

## Rhai quick reference

A brief cheat sheet for hook authors. For the full language reference, see the [Rhai book](https://rhai.rs/book/).

### Variables

```rhai
let name = "diecut";
let count = 42;
let flag = true;
```

### Template strings

```rhai
print(`Hello, ${name}!`);
```

### Conditionals

```rhai
if count > 10 {
    print("Big number");
} else {
    print("Small number");
}
```

### Loops

```rhai
for i in range(0, 5) {
    print(i);
}
```

### Functions

```rhai
fn greet(name) {
    print(`Hello, ${name}!`);
}
greet("world");
```

### Abort with error

```rhai
throw "Something went wrong";
```
