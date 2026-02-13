# Docs Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand diecut docs from 3 pages to 8, with example-first content, explicit audience split, and the author's warm, direct writing voice.

**Architecture:** Astro Starlight site at `docs/`. All content lives in `docs/src/content/docs/`. Sidebar config in `docs/astro.config.mjs`. Pages are `.mdx` (Astro components) or `.md` (plain markdown). Starlight provides search, dark mode, sidebar, mobile support out of the box.

**Tech Stack:** Astro Starlight, MDX, pnpm

**Design doc:** `docs/plans/2026-02-12-docs-redesign-design.md`

**Writing style:** Short declarative sentences. Concrete before abstract. No hedging. No emoji. Example-first — show config, command, output for every concept. See design doc "Writing Style Guide" section for full details.

---

### Task 1: Scaffold structure and update sidebar

**Files:**
- Modify: `docs/astro.config.mjs`
- Create: `docs/src/content/docs/using-templates/index.mdx`
- Create: `docs/src/content/docs/creating-templates/index.mdx`
- Create: `docs/src/content/docs/migrating-from-cookiecutter.md`
- Create: `docs/src/content/docs/reference/diecut-toml.md`
- Create: `docs/src/content/docs/reference/hooks.md`

**Step 1: Create stub pages**

Create each new page with just frontmatter so the build doesn't break. Each stub should have:

```md
---
title: [Page Title]
description: [One-line description]
---

(Content coming soon)
```

Stubs to create:

- `docs/src/content/docs/using-templates/index.mdx`:
  - title: "Using Templates"
  - description: "Generate projects from existing diecut and cookiecutter templates."

- `docs/src/content/docs/creating-templates/index.mdx`:
  - title: "Creating Templates"
  - description: "Build your own project templates with diecut."

- `docs/src/content/docs/migrating-from-cookiecutter.md`:
  - title: "Migrating from Cookiecutter"
  - description: "Convert cookiecutter templates to native diecut format."

- `docs/src/content/docs/reference/diecut-toml.md`:
  - title: "diecut.toml"
  - description: "Complete reference for the diecut template configuration file."

- `docs/src/content/docs/reference/hooks.md`:
  - title: "Hooks"
  - description: "Reference for Rhai hooks in diecut templates."

**Step 2: Update sidebar config**

Replace the sidebar array in `docs/astro.config.mjs` with:

```js
sidebar: [
    {
        label: 'Getting Started',
        items: [
            { label: 'Installation & Quick Start', slug: 'getting-started' },
        ],
    },
    {
        label: 'Guides',
        items: [
            { label: 'Using Templates', slug: 'using-templates' },
            { label: 'Creating Templates', slug: 'creating-templates' },
            { label: 'Migrating from Cookiecutter', slug: 'migrating-from-cookiecutter' },
        ],
    },
    {
        label: 'Reference',
        items: [
            { label: 'Commands', slug: 'reference/commands' },
            { label: 'diecut.toml', slug: 'reference/diecut-toml' },
            { label: 'Hooks', slug: 'reference/hooks' },
        ],
    },
],
```

**Step 3: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds with no errors. All 8 pages exist.

**Step 4: Commit**

```bash
git add docs/astro.config.mjs docs/src/content/docs/
git commit -m "docs: scaffold new page structure with stubs"
```

---

### Task 2: Rewrite landing page

**Files:**
- Modify: `docs/src/content/docs/index.mdx`

**Step 1: Rewrite the landing page**

Keep the Starlight splash template. Rewrite copy per design doc:

- **Hero tagline:** Direct, punchy. Not a feature description — a value statement. Something like: "Make templates. Start projects. Skip the setup." (Final wording is author's call — match the blog voice.)
- **Hero description:** One sentence: single binary, no runtime deps, cookiecutter compatible.
- **CTA buttons:** Keep "Get Started" → `/getting-started/` and "View on GitHub" → repo link.

Rewrite the 4 feature cards:

1. **"Single binary"** (icon: rocket) — No Python, no Node, no runtime dependencies. Download one file and go.
2. **"Easy to make"** (icon: pencil) — A `diecut.toml` and a folder. That's a template. You just made one.
3. **"Cookiecutter compatible"** (icon: puzzle) — Use existing cookiecutter templates directly. diecut auto-detects and translates them.
4. **"Updates built in"** (icon: refresh) — Upstream template changed? Update your project with a three-way merge. No re-generation needed.

Below the CardGrid, add an origin story section. 3-4 sentences, first-person, the author's voice. This is the emotional hook. Something like:

> I built diecut because I wanted my son to skip the setup and get to the fun part of programming. Every language has its own project scaffolding ritual — cargo init, npm init, poetry new — but none of them give you a real starting point. Templates do. And making templates shouldn't require a PhD in Jinja2.

(Author should rewrite in their own words — this is a placeholder for voice/length.)

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds. Landing page renders with new content.

**Step 3: Commit**

```bash
git add docs/src/content/docs/index.mdx
git commit -m "docs: rewrite landing page with origin story and value props"
```

---

### Task 3: Expand getting-started page

**Files:**
- Modify: `docs/src/content/docs/getting-started/index.mdx`

**Step 1: Rewrite the getting-started page**

Goal: install and generate your first project in under 60 seconds of reading.

**Section 1: Install**

Keep the three install methods (curl, cargo, releases). Tighten the copy. No explanation needed for what curl does — people who use curl know what it does.

```bash
curl -fsSL https://raw.githubusercontent.com/raiderrobert/diecut/main/install.sh | sh
```

```bash
cargo install --path crates/diecut-cli
```

Or grab a binary from [GitHub Releases](https://github.com/raiderrobert/diecut/releases).

**Section 2: Your first project**

Use the `examples/rust-cli` template as the real example. Show the command:

```bash
diecut new ./examples/rust-cli -o my-cli
```

Then show what the prompts look like (project_name, description, author, license, rust_edition). Then show the resulting directory tree:

```
my-cli/
  Cargo.toml
  src/
    main.rs
  .gitignore
  .diecut-answers.toml
```

Then show a snippet of the generated `Cargo.toml` so they can see their answers were applied.

**Section 3: What just happened?**

2-3 sentences: diecut read the `diecut.toml`, prompted you for variables, rendered `.tera` files through the Tera template engine, and wrote the result. Files without `.tera` were copied as-is. The `.diecut-answers.toml` saves your choices for later updates.

**Section 4: Next steps**

Explicit audience fork using Starlight `LinkCard` or bold links:

- **Want to use existing templates?** → [Using Templates](/using-templates/)
- **Want to make your own?** → [Creating Templates](/creating-templates/)

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/getting-started/index.mdx
git commit -m "docs: expand getting-started with real example and audience fork"
```

---

### Task 4: Write using-templates guide

**Files:**
- Modify: `docs/src/content/docs/using-templates/index.mdx`

**Step 1: Write the using-templates page**

This is the template *consumer* guide. Every section shows the command and the output.

**Section 1: Template sources**

The table from the README:

| Source | Example |
|--------|---------|
| Local path | `diecut new ./my-template` |
| GitHub | `diecut new gh:user/repo` |
| GitLab | `diecut new gl:user/repo` |
| Bitbucket | `diecut new bb:user/repo` |
| Sourcehut | `diecut new sr:user/repo` |
| Any Git URL | `diecut new https://git.example.com/repo.git` |

One sentence explaining that Git templates are cached at `~/.cache/diecut/templates/` (overridable via `DIECUT_CACHE_DIR`).

**Section 2: Overriding variables**

Show `-d key=value`:

```bash
diecut new gh:user/template -d project_name=foo -d license=MIT -o my-project
```

Explain: variables passed with `-d` skip the interactive prompt. You can mix — pass some via `-d` and let diecut prompt for the rest.

**Section 3: Non-interactive mode**

For CI pipelines or scripting:

```bash
diecut new gh:user/template --defaults -o my-project
```

Explain: `--defaults` uses every variable's default value without prompting. Combine with `-d` to override specific ones:

```bash
diecut new gh:user/template --defaults -d project_name=ci-test -o output
```

**Section 4: Updating a project**

This is a differentiator — give it real space.

Explain the concept: you generated a project 3 months ago. The upstream template added a new CI config. Instead of re-generating from scratch, update:

```bash
diecut update ./my-project
```

Show what happens: reads `.diecut-answers.toml`, compares the old template version against the new one, three-way merges against your actual files. Reports what changed.

Show `--ref` for pinning to a specific version:

```bash
diecut update ./my-project --ref v2.0.0
```

Mention: conflicts are saved as `.rej` files, like `patch` does. You resolve them manually.

**Section 5: Cookiecutter compatibility**

```bash
diecut new gh:audreyfeldroy/cookiecutter-pypackage -o my-package
```

Explain: diecut auto-detects `cookiecutter.json` and translates on the fly. No migration needed. If you want a permanent conversion, see [Migrating from Cookiecutter](/migrating-from-cookiecutter/).

**Section 6: Managing the cache**

```bash
diecut list
```

Shows cached templates. Templates live at `~/.cache/diecut/templates/`. Set `DIECUT_CACHE_DIR` to change this. Delete the cache directory to clear it.

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/using-templates/index.mdx
git commit -m "docs: add using-templates guide"
```

---

### Task 5: Write creating-templates guide (flagship)

**Files:**
- Modify: `docs/src/content/docs/creating-templates/index.mdx`

**Step 1: Write the creating-templates page**

This is the flagship page. Progressive disclosure — start with the absolute minimum, layer complexity.

**Section 1: Your first template**

The simplest possible template. Show the complete directory structure:

```
hello-template/
  diecut.toml
  template/
    hello.txt.tera
```

Show the `diecut.toml`:

```toml
[template]
name = "hello"

[variables.name]
type = "string"
prompt = "Your name"
default = "world"
```

Show the `hello.txt.tera`:

```
Hello, {{ name }}!
```

Show running it:

```bash
diecut new ./hello-template -o output
```

Show the prompt and the output file. End with: "That's a template. You just made one."

**Section 2: Variable types**

Show each type with a minimal `diecut.toml` snippet and what the prompt looks like:

- `string` — text input (already shown above)
- `bool` — yes/no toggle
- `int` — integer input
- `float` — decimal input
- `select` — pick one from choices
- `multiselect` — pick multiple from choices

For each, show the TOML and the rendered result. Use the `examples/rust-cli` and `examples/python-pkg` templates as real references where possible.

For `select`:

```toml
[variables.license]
type = "select"
prompt = "License"
choices = ["MIT", "Apache-2.0", "GPL-3.0"]
default = "MIT"
```

For `multiselect`:

```toml
[variables.features]
type = "multiselect"
prompt = "Features to include"
choices = ["logging", "docker", "ci"]
default = ["logging"]
```

**Section 3: Conditional prompts**

Use `when` to only ask a question when a previous answer makes it relevant:

```toml
[variables.use_ci]
type = "bool"
prompt = "Set up CI?"
default = true

[variables.ci_provider]
type = "select"
prompt = "CI provider"
choices = ["github-actions", "gitlab-ci"]
when = "{{ use_ci }}"
```

Explain: `ci_provider` is only prompted when `use_ci` is true. The `when` field takes a Tera expression.

**Section 4: Conditional files**

Include or exclude files based on variable values:

```toml
[files]
conditional = [
    { pattern = ".github/**", when = "use_ci and ci_provider == 'github-actions'" },
    { pattern = ".gitlab-ci.yml", when = "use_ci and ci_provider == 'gitlab-ci'" },
]
```

Explain: files matching the pattern are only included when the `when` expression is true. Use the `examples/python-pkg` template's conditional as a real example — it excludes `src/cli.py` when `use_cli` is false.

**Section 5: Computed variables**

Variables that are derived, never prompted:

```toml
[variables.project_slug]
type = "string"
computed = "{{ project_name | slugify }}"
```

Explain: `computed` variables are available in templates but never shown to the user. Useful for derived values like slugified names, lowercase versions, etc.

**Section 6: File handling**

Two features:

Exclude files from output (gitignore-style patterns):

```toml
[files]
exclude = ["*.pyc", ".DS_Store", "__pycache__/**"]
```

Copy files without rendering (for binaries, images, etc.):

```toml
[files]
copy_without_render = ["assets/**/*.png", "fonts/**"]
```

Explain: files matching `copy_without_render` are copied verbatim — Tera syntax inside them is left alone.

**Section 7: Tera template basics**

Don't assume Jinja2/Tera knowledge. Teach the 5 things they'll actually use:

1. **Variables:** `{{ project_name }}` — inserts the value
2. **Conditionals:** `{% if use_ci %}...{% endif %}` — include content conditionally
3. **Loops:** `{% for feature in features %}...{% endfor %}` — iterate over multiselect values
4. **Filters:** `{{ project_name | slugify }}`, `{{ name | upper }}` — transform values
5. **Comments:** `{# This won't appear in output #}`

Show a real `.tera` file using several of these together — use the `Cargo.toml.tera` from `examples/rust-cli` as the reference:

```
[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "{{ rust_edition }}"
description = "{{ description }}"
{% if author %}authors = ["{{ author }}"]
{% endif %}license = "{{ license }}"
```

Link to full Tera docs for advanced usage: https://keats.github.io/tera/docs/

**Section 8: Validation**

Show `diecut check` for validating a template:

```bash
diecut check ./my-template
```

Explain what it checks: format detection, variable definitions, config consistency, warnings. Show example output.

Show `diecut ready` for distribution readiness:

```bash
diecut ready ./my-template
```

Explain: `ready` is stricter than `check` — it verifies the template is suitable for sharing (e.g., has a description, variables have prompts).

Also show regex validation on variables:

```toml
[variables.project_name]
type = "string"
prompt = "Project name"
validation = '^[a-z][a-z0-9_-]*$'
validation_message = "Must start with a letter, only lowercase letters, numbers, hyphens, underscores"
```

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/creating-templates/index.mdx
git commit -m "docs: add creating-templates guide (flagship page)"
```

---

### Task 6: Write migrating-from-cookiecutter page

**Files:**
- Modify: `docs/src/content/docs/migrating-from-cookiecutter.md`

**Step 1: Write the migration page**

Short and practical. Not a comparison — a how-to.

**Section 1: Auto-detection (you might not need to migrate)**

Explain that `diecut new` auto-detects cookiecutter templates. If you just want to *use* a cookiecutter template, you don't need to migrate at all:

```bash
diecut new gh:audreyfeldroy/cookiecutter-pypackage -o my-package
```

Migration is for template *authors* who want to convert to native diecut format.

**Section 2: Dry-run first**

Always preview before migrating:

```bash
diecut migrate ./my-cookiecutter-template --dry-run
```

Show example output of what changes would be made.

**Section 3: Migrate**

Two options — in-place or to a new directory:

```bash
# To a new directory (safer)
diecut migrate ./my-cookiecutter-template --output ./my-diecut-template

# In-place
diecut migrate ./my-cookiecutter-template
```

**Section 4: What gets translated**

Side-by-side showing the mapping:

| Cookiecutter | diecut |
|---|---|
| `cookiecutter.json` | `diecut.toml` |
| `{{cookiecutter.var}}` in filenames/content | `{{var}}` |
| `hooks/pre_gen_project.py` | Not auto-converted (see below) |
| `hooks/post_gen_project.py` | Not auto-converted (see below) |

Show a concrete example — a `cookiecutter.json`:

```json
{
    "project_name": "my-project",
    "license": ["MIT", "Apache-2.0", "GPL-3.0"],
    "use_docker": "n"
}
```

And the resulting `diecut.toml`:

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

**Section 5: What doesn't translate**

Python hooks can't be auto-converted to Rhai. You'll need to rewrite them manually. Link to [Hooks reference](/reference/hooks/) for Rhai documentation.

Jinja2 extensions (like `cookiecutter.extensions` in `cookiecutter.json`) have no equivalent. Most common extensions (slugify, random string) are available as Tera filters.

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/migrating-from-cookiecutter.md
git commit -m "docs: add cookiecutter migration guide"
```

---

### Task 7: Rewrite commands reference

**Files:**
- Modify: `docs/src/content/docs/reference/commands.md`

**Step 1: Rewrite the commands reference**

Use a consistent template for every command (Starship-style). Source of truth: `crates/diecut-cli/src/cli.rs`.

For each command, use this format:

```
## diecut <command>

<One-sentence description.>

### Synopsis

\`\`\`bash
diecut <command> [ARGS] [OPTIONS]
\`\`\`

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| ... | ... | ... | ... |

### Examples

\`\`\`bash
# Description of example
diecut <command> <args>
\`\`\`

### Notes

<Edge cases, gotchas, related commands.>
```

Commands to document (from `cli.rs`):

1. **`diecut new <TEMPLATE>`** — Generate a new project from a template
   - Options: `-o/--output`, `-d/--data KEY=VALUE`, `--defaults`, `--overwrite`, `--no-hooks`
   - Template argument supports: local path, `gh:`, `gl:`, `bb:`, `sr:`, git URL

2. **`diecut list`** — List cached templates (no arguments, no options)

3. **`diecut update <PATH>`** — Update a previously generated project
   - Options: `--ref <TAG>` (default path: `.`)
   - Note: requires `.diecut-answers.toml` in the project

4. **`diecut check [PATH]`** — Validate a template directory (default: `.`)

5. **`diecut ready [PATH]`** — Check if a template is ready for distribution (default: `.`)

6. **`diecut migrate <PATH>`** — Migrate a cookiecutter template to native diecut format
   - Options: `-o/--output`, `--dry-run` (default path: `.`)

Each command gets 2-3 examples showing real usage with real flags.

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/reference/commands.md
git commit -m "docs: rewrite commands reference with consistent format"
```

---

### Task 8: Write diecut.toml reference

**Files:**
- Modify: `docs/src/content/docs/reference/diecut-toml.md`

**Step 1: Write the diecut.toml reference**

Two-tier pattern: summary table at top, detailed sections below. Source of truth: `crates/diecut-core/src/config/schema.rs` and `crates/diecut-core/src/config/variable.rs`.

**Summary table:**

Group by section, not alphabetically:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| **[template]** | | | |
| `name` | string | required | Template name |
| `version` | string | — | Template version |
| `description` | string | — | Short description |
| `min_diecut_version` | string | — | Minimum diecut version required |
| `templates_suffix` | string | `".tera"` | File suffix for template rendering |
| **[variables.NAME]** | | | |
| `type` | enum | required | `string`, `bool`, `int`, `float`, `select`, `multiselect` |
| `prompt` | string | — | Text shown to user (required for prompted vars) |
| `default` | varies | — | Default value |
| `choices` | string[] | — | Available options (required for select/multiselect) |
| `validation` | string | — | Regex pattern for input validation |
| `validation_message` | string | — | Message shown when validation fails |
| `when` | string | — | Tera expression — if false, variable is skipped |
| `computed` | string | — | Tera expression — value is derived, never prompted |
| `secret` | bool | `false` | If true, value is not saved to answers file |
| **[files]** | | | |
| `exclude` | string[] | `[]` | Glob patterns to exclude from output |
| `copy_without_render` | string[] | `[]` | Glob patterns to copy without Tera rendering |
| `conditional` | array | `[]` | Conditional file inclusion rules |
| **[files.conditional]** | | | |
| `pattern` | string | required | Glob pattern matching files |
| `when` | string | required | Tera expression — if false, files are excluded |
| **[hooks]** | | | |
| `pre_generate` | string[] | `[]` | Rhai scripts to run before generation |
| `post_generate` | string[] | `[]` | Rhai scripts to run after generation |
| **[answers]** | | | |
| `file` | string | `".diecut-answers.toml"` | Filename for answers file in generated project |

**Detailed sections:**

Below the table, one section per config group (`[template]`, `[variables]`, `[files]`, `[hooks]`, `[answers]`). Each section has:

- Brief explanation of the group's purpose
- A complete TOML example using real values (pull from `examples/rust-cli/diecut.toml` and `examples/python-pkg/diecut.toml`)
- Per-key details only where the table description isn't sufficient (e.g., explain `when` expressions, explain `computed` vs prompted, explain the validation regex behavior)

Keep it DRY — don't repeat what the creating-templates guide explains in narrative form. This page is for quick reference lookup.

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/reference/diecut-toml.md
git commit -m "docs: add diecut.toml config reference"
```

---

### Task 9: Write hooks reference

**Files:**
- Modify: `docs/src/content/docs/reference/hooks.md`

**Step 1: Write the hooks reference**

Guide + reference in one page. Source of truth: `crates/diecut-core/src/hooks/mod.rs` and `crates/diecut-core/src/hooks/rhai_runtime.rs`.

**Section 1: What are hooks?**

Hooks are scripts that run during project generation. They're written in [Rhai](https://rhai.rs/), a sandboxed scripting language compiled into the diecut binary. No shell, no external dependencies, identical behavior on every platform.

Two hook points:
- `pre_generate` — runs before files are rendered (e.g., validate inputs, abort on bad state)
- `post_generate` — runs after files are written (e.g., print messages, clean up files)

**Section 2: Setup**

Show how to configure hooks in `diecut.toml`:

```toml
[hooks]
pre_generate = ["hooks/pre_generate.rhai"]
post_generate = ["hooks/post_generate.rhai"]
```

And the corresponding file structure:

```
my-template/
  diecut.toml
  hooks/
    pre_generate.rhai
    post_generate.rhai
  template/
    ...
```

**Section 3: Available context**

In hooks, all template variables are available as Rhai variables:

```rhai
// If your diecut.toml has [variables.project_name]
print(`Project name: ${project_name}`);
```

Post-generate hooks also get `output_dir` — the path where files were written:

```rhai
print(`Files written to: ${output_dir}`);
```

Variable types map to Rhai types:
| diecut type | Rhai type |
|-------------|-----------|
| string | String |
| bool | bool |
| int | i64 |
| float | f64 |
| select | String |
| multiselect | String (serialized) |

**Section 4: Examples**

Real-world hook examples:

Print a success message:
```rhai
// hooks/post_generate.rhai
print(`Created ${project_name} at ${output_dir}`);
print("Run `cargo build` to get started.");
```

Validate input in pre-generate:
```rhai
// hooks/pre_generate.rhai
if project_name.len() < 2 {
    throw "Project name must be at least 2 characters";
}
```

Note: `throw` in a pre-generate hook aborts generation with an error message.

**Section 5: Sandboxing and limits**

Rhai is sandboxed. Hooks cannot:
- Access the filesystem (except through `output_dir` path as a string)
- Make network requests
- Execute shell commands
- Access environment variables

Safety limits (from `rhai_runtime.rs`):
- Max call depth: 32
- Max operations: 100,000
- Max string size: 10MB

These limits prevent runaway scripts. If a hook exceeds them, it fails with an error.

**Section 6: Rhai language reference**

Don't duplicate Rhai docs. Provide a brief cheat sheet of the most useful Rhai features for hook authors:

- Variables: `let x = 42;`
- Strings: template strings with `${var}`, concatenation with `+`
- Conditionals: `if`/`else if`/`else`
- Loops: `for item in list { ... }`
- Functions: `fn greet(name) { print(`Hello ${name}`); }`
- Error handling: `throw "message"` to abort

Link to full Rhai docs: https://rhai.rs/book/

**Step 2: Build to verify**

Run: `cd docs && pnpm build`
Expected: Build succeeds.

**Step 3: Commit**

```bash
git add docs/src/content/docs/reference/hooks.md
git commit -m "docs: add hooks reference"
```

---

### Task 10: Final build, review, and cleanup

**Step 1: Full build**

Run: `cd docs && pnpm build`
Expected: Clean build with no warnings or errors.

**Step 2: Visual review**

Run: `cd docs && pnpm dev`
Open in browser. Check every page:
- Landing page: hero, cards, origin story render correctly
- Getting started: install + example look clean
- Using templates: all sections present, examples formatted
- Creating templates: progressive disclosure reads well, code blocks render
- Migrating: side-by-side tables display correctly
- Commands: consistent format across all 6 commands
- diecut.toml: summary table and detail sections both render
- Hooks: code examples, tables, links all work

Check sidebar navigation matches the design. Check that internal links between pages work.

**Step 3: Update README**

Trim the README to point to the docs site instead of duplicating content. Keep:
- One-liner description
- Install section (curl + cargo)
- 3-4 line quick start
- Link to docs site for everything else

Remove from README: Template Format section, diecut.toml section, Hooks section, Development section (these now live in docs).

**Step 4: Commit**

```bash
git add docs/ README.md
git commit -m "docs: final review pass and README cleanup"
```
