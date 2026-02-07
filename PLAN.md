# Diecut: Architecture & Implementation Plan

## Context

**Problem**: Project template generators (cookiecutter, copier, yeoman) all require Python or Node.js runtimes. The community's #1 ask is a single-binary, language-agnostic tool with template updates and composability. No existing Rust tool fills this gap — cargo-generate is Rust-only, kickstart is abandoned, ffizer is niche.

**Goal**: Build `diecut`, a Rust CLI that combines cookiecutter's simplicity, copier's update mechanism, and a novel template composition system — all as a single binary with zero runtime dependencies.

**Confirmed decisions**: TOML config, Tera templating engine, basics-first milestone approach.

---

## Template Format

A diecut template repo looks like this:

```
my-template/
  diecut.toml              # Config (required)
  template/                # Template content (required)
    {{project_name}}/
      README.md.tera       # .tera files are rendered, suffix stripped
      src/
        main.rs.tera
      Cargo.toml.tera
      .gitignore            # No .tera = copied verbatim
  hooks/                    # Optional Rhai scripts
    pre_generate.rhai
    post_generate.rhai
```

Key design: template files live in `template/` subdirectory (not repo root). Files ending in `.tera` are rendered through Tera then have the suffix stripped. This solves cookiecutter's IDE-confusion problem.

### `diecut.toml` Example

```toml
[template]
name = "rust-cli"
version = "1.2.0"
description = "A production-ready Rust CLI template"
min_diecut_version = "0.1.0"
templates_suffix = ".tera"  # default

# --- Variables (prompted in declaration order) ---

[variables.project_name]
type = "string"
prompt = "Project name"
default = "my-project"
validation = '^[a-z][a-z0-9_-]*$'
validation_message = "Lowercase alphanumeric, hyphens, underscores only"

[variables.use_ci]
type = "bool"
prompt = "Set up GitHub Actions CI?"
default = true

[variables.license]
type = "select"
prompt = "License"
choices = ["MIT", "Apache-2.0", "GPL-3.0"]
default = "MIT"

[variables.features]
type = "multiselect"
prompt = "Features to include"
choices = ["logging", "cli-args", "docker", "nix"]
default = ["logging", "cli-args"]

# Conditional question — only asked when use_ci is true
[variables.ci_provider]
type = "select"
prompt = "CI provider"
choices = ["github-actions", "gitlab-ci"]
default = "github-actions"
when = "{{ use_ci }}"

# Computed variable — never prompted
[variables.project_slug]
type = "string"
computed = '{{ project_name | slugify }}'

# --- File Rules ---

[files]
exclude = ["*.pyc", ".DS_Store"]
copy_without_render = ["assets/**/*.png", "vendor/**"]

[[files.conditional]]
pattern = ".github/**"
when = "{{ use_ci and ci_provider == 'github-actions' }}"

[[files.conditional]]
pattern = "Dockerfile"
when = "{{ 'docker' in features }}"

# --- Hooks ---

[hooks]
pre_generate = ["hooks/pre_generate.rhai"]
post_generate = ["hooks/post_generate.rhai"]

# --- Answers (for future `diecut update`) ---

[answers]
file = ".diecut-answers.toml"
```

**Variable types**: `string`, `bool`, `int`, `float`, `select`, `multiselect`
**Special fields**: `when` (conditional), `computed` (derived), `secret` (not saved), `validation` (regex or Tera expr)

---

## Architecture

### Workspace Structure

```
diecut/
  Cargo.toml                  # Workspace root
  crates/
    diecut-cli/               # Binary crate
      src/
        main.rs
        cli.rs                # clap definitions
        commands/
          mod.rs
          new.rs              # `diecut new`
    diecut-core/              # Library crate
      src/
        lib.rs
        config/
          mod.rs              # Config loading
          schema.rs           # diecut.toml serde structs
          variable.rs         # Variable type definitions
        template/
          mod.rs
          source.rs           # Local/git/abbreviation resolution
          cache.rs            # Template caching
        prompt/
          mod.rs              # Interactive prompting via inquire
          engine.rs           # Orchestration with conditionals
        render/
          mod.rs
          context.rs          # Tera context construction
          walker.rs           # FS traversal + conditional inclusion
          file.rs             # Single file render/copy
        hooks/
          mod.rs
          rhai_runtime.rs     # Rhai scripting environment
        answers/
          mod.rs              # Answer persistence
        error.rs              # thiserror error types
```

### Dependencies

| Crate | Purpose |
|-------|---------|
| `tera` 2.x | Template engine (Jinja2-like, Rust-native) |
| `clap` 4 (derive) | CLI parsing |
| `inquire` 0.7 | Interactive prompts (text, confirm, select, multiselect) |
| `serde` + `toml` | Config deserialization |
| `gix` | Pure-Rust git (no C/libgit2 deps) |
| `globset` | Fast glob matching (by ripgrep author) |
| `rhai` | Cross-platform hook scripting (sandboxed, no shell needed) |
| `thiserror` + `miette` | Error types + rich colored diagnostics |
| `walkdir` | Directory traversal |
| `indicatif` + `console` | Progress bars + terminal styling |
| `dirs` | XDG-compliant platform directories |

### Data Flow

```
CLI: diecut new gh:alice/rust-cli my-project
  1. Resolve source -> expand abbreviation -> check cache -> git clone
  2. Load diecut.toml -> deserialize -> TemplateConfig
  3. Collect variables (in declaration order):
     - Skip computed vars (evaluate later)
     - Evaluate `when` against current answers
     - Check --data overrides -> previous answers -> prompt user
  4. Evaluate computed variables
  5. Evaluate [[files.conditional]] rules -> build exclusion set
  6. Run pre_generate hooks (Rhai)
  7. Walk template/ tree:
     - Skip excluded files/dirs
     - Render dir/file names through Tera
     - .tera files -> render content + strip suffix
     - Binary/copy_without_render -> copy verbatim
  8. Run post_generate hooks (Rhai)
  9. Write .diecut-answers.toml (excluding secrets)
  10. Print summary
```

---

## CLI Design

```
diecut new <template> [--output DIR] [--ref TAG] [--data K=V...] [--defaults] [--overwrite] [--no-hooks]
diecut check [PATH]        # Validate a template (for authors)
diecut update [PATH]       # Future: update project from template
diecut list                # Future: list cached templates
```

**Abbreviations** (built-in, extensible via `~/.config/diecut/config.toml`):

| Prefix | Expansion |
|--------|-----------|
| `gh:` | `https://github.com/{}.git` |
| `gl:` | `https://gitlab.com/{}.git` |
| `bb:` | `https://bitbucket.org/{}.git` |
| `sr:` | `https://git.sr.ht/~{}` |

---

## Key Design Decisions

### Why Tera over MiniJinja/Handlebars
- Jinja2-familiar syntax — cookiecutter/copier users feel at home
- Rich built-in filters (`slugify`, `date`, `urlencode`, etc.) perfect for project templating
- Native `{% extends %}` / `{% block %}` for future template composition (M5)
- Better error messages with source spans in v2 (pairs well with miette)
- MiniJinja is a viable migration target if needed — abstract behind a trait

### Why TOML over YAML
- Comments natively supported (critical for template config documentation)
- No "Norway problem" (YAML parses `NO` as `false`)
- Rust ecosystem standard (Cargo.toml, rustfmt.toml)
- No indentation sensitivity — less error-prone
- Trade-off: more verbose for deep nesting, but diecut config is mostly flat

### Why Rhai for hooks (not shell scripts)
- Cross-platform: identical behavior on Windows/macOS/Linux
- Sandboxed: hooks can't do arbitrary things unless explicitly allowed
- No runtime deps: compiled into the binary
- Proven: cargo-generate uses Rhai successfully

### Conditional files: declarative config (not hooks)
- Cookiecutter's approach (generate everything, delete in post-hook) wastes work
- Copier's approach (Jinja in filenames) is unreadable
- Diecut: `[[files.conditional]]` with glob patterns + Tera `when` expressions
- All logic visible in one place, easy to validate with `diecut check`

### .tera suffix approach
- Solves IDE confusion (linters don't try to parse `Cargo.toml.tera` as TOML)
- Explicit: you know exactly which files are templates
- Binary-safe: images/fonts never accidentally rendered
- Configurable: `templates_suffix = ""` renders everything (cookiecutter compat)

---

## Milestones

### M1: Core Generation (implement first)
Generate a project from a **local** template directory.

- [ ] Set up workspace (`diecut-cli` + `diecut-core`)
- [ ] `diecut.toml` deserialization (serde + toml)
- [ ] Variable prompting (inquire): string, bool, int, float, select, multiselect
- [ ] Tera rendering pipeline: context construction, file content, filenames, dir names
- [ ] .tera suffix detection + stripping
- [ ] Binary file detection (copy verbatim)
- [ ] File walker with `exclude` and `copy_without_render` patterns
- [ ] `diecut new ./local-path` command
- [ ] `--data`, `--defaults`, `--output`, `--overwrite` flags
- [ ] Error handling with miette
- [ ] Integration tests with fixture templates

### M2: Git Sources + Caching
- [ ] Git URL detection + abbreviation expansion
- [ ] Git clone via gix (branch/tag/commit checkout)
- [ ] Template caching (`~/.cache/diecut/templates/`)
- [ ] `--ref` flag
- [ ] `diecut list` command
- [ ] User config file (`~/.config/diecut/config.toml`)

### M3: Hooks, Conditionals, Validation
- [ ] Conditional questions (`when` field evaluation)
- [ ] Computed variables (`computed` field)
- [ ] Input validation (regex + Tera expressions)
- [ ] `[[files.conditional]]` evaluation
- [ ] Rhai hook system (sandboxed, with exposed FS/command functions)
- [ ] `diecut check` command (template validation)
- [ ] `.diecut-answers.toml` persistence

### M4: Template Updates (copier's killer feature, in Rust)
- [ ] Read answers file from existing project
- [ ] Three-way merge: old snapshot vs new snapshot vs actual project
- [ ] Conflict resolution (inline markers or .rej files)
- [ ] Migration hooks (version-specific)
- [ ] `diecut update` command

### M5: Template Composition (novel differentiator)
- [ ] `[template.extends]` — inherit from base template
- [ ] `[template.includes]` — pull in partial templates
- [ ] Variable/file overlay and merge strategy

### M6: Registry
- [ ] Template discovery and search
- [ ] `diecut search` / `diecut publish`

---

## Verification Plan

After M1 implementation:
1. Create a fixture template in `tests/fixtures/basic-template/` with diecut.toml + template/ directory
2. Run `cargo test` — integration tests generate projects and verify output
3. Run `cargo build --release` and manually test: `./target/release/diecut new tests/fixtures/basic-template/`
4. Verify: correct files generated, .tera suffixes stripped, variables substituted, binary files copied verbatim
5. Run `cargo clippy` and `cargo fmt --check`
