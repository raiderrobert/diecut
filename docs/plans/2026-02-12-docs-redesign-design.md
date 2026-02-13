# Docs Redesign Design

## Motivation

diecut exists because templates should be easy to make, not just easy to use. The origin story: a dad wanted his son to skip the setup and get straight to the fun part of programming. That same impulse — simplicity, speed, less overhead — should define the documentation.

The current docs site has 3 pages (landing, getting-started, commands reference). The README is more comprehensive than the docs. This redesign expands to 8 focused pages that serve both template users and template authors equally.

## Design Principles

Derived from competitive research (cookiecutter, copier) and inspiration sites (just, ghostty, starship), filtered through the project's values:

1. **Example-first.** Every concept shows the config, the command, and the output. Borrowed from `just`, which does this better than anyone.
2. **Progressive disclosure.** Start simple, layer complexity. The creating-templates guide goes from "a folder and a TOML file" to conditional files and computed variables.
3. **Direct and personal tone.** Short declarative sentences. Concrete before abstract. Stories before principles. Skeptical of unnecessary complexity. Matches the author's blog voice.
4. **Low maintenance.** 8 pages total. No FAQ (answers go inline), no template gallery (curation burden), no blog, no changelog page, no comparison page. Content changes only when features change.
5. **Explicit audience split.** "Using Templates" for consumers, "Creating Templates" for authors. Not implicit — separate pages with separate navigation.
6. **Consistent reference format.** Every command and config key follows an identical template (Starship-style): synopsis/key, options table, examples, notes.
7. **Philosophy before reference.** The landing page leads with *why*, not *what*. Borrowed from Ghostty's approach.

## Framework

Keep Astro Starlight. Already set up, good defaults (search, dark mode, sidebar, mobile), zero maintenance. No framework migration.

## Site Structure

```
docs/src/content/docs/
  index.mdx                              # Landing page
  getting-started/
    index.mdx                            # Install + first project
  using-templates/
    index.mdx                            # Template user guide
  creating-templates/
    index.mdx                            # Template author guide (flagship)
  migrating-from-cookiecutter.md         # Cookiecutter migration how-to
  reference/
    commands.md                          # CLI reference
    diecut-toml.md                       # Config file reference
    hooks.md                             # Rhai hooks reference
```

Starlight sidebar config:

```
1. Getting Started        (single page)
2. Guides
   - Using Templates
   - Creating Templates
   - Migrating from Cookiecutter
3. Reference
   - Commands
   - diecut.toml
   - Hooks
```

## Page-by-Page Content Plan

### 1. Landing Page (index.mdx)

Rework the existing splash page.

**Hero tagline:** Something direct — not "language-agnostic project template generator" but something like "Make templates. Start projects. Skip the setup."

**4 feature cards (rewritten):**
- Single binary — no Python, no Node, no runtime. `curl | sh` and go.
- Templates are easy to make — a `diecut.toml` and a folder. That's it.
- Cookiecutter compatible — your existing templates just work.
- Updates built in — upstream template changes? Three-way merge.

**Origin story:** 3-4 sentences below the cards. The "I built this for my son" story. Personal, direct, brief. The emotional hook that no competitor has.

### 2. Getting Started (getting-started/index.mdx)

Goal: install and generate your first project in under 60 seconds of reading.

- **Install** — curl one-liner, cargo, manual download. Keep it short.
- **Your first project** — use a real example template from the repo (examples/rust-cli or examples/python-pkg). Show the command. Show the output directory tree.
- **What just happened?** — 2-3 sentences demystifying the flow: read template, prompted you, rendered files, done.
- **Next steps** — explicit audience fork: "Want to use templates?" / "Want to make templates?" with links.

### 3. Using Templates (using-templates/index.mdx)

The template consumer guide. Everything a user needs who isn't authoring templates.

- Template sources (local, gh:, gl:, bb:, sr:, git URLs) with the table
- Overriding variables from the CLI (-d key=value)
- Non-interactive mode (--defaults) for CI
- Updating a project when upstream changes (diecut update) — real space for this, it's a differentiator
- Cache behavior (location, clearing)
- Using cookiecutter templates directly (auto-detection)

Each concept: show the command, show the output.

### 4. Creating Templates (creating-templates/index.mdx)

The flagship page. This is what makes diecut different.

Progressive structure:
1. **Minimal template** — a folder, a diecut.toml with one variable, one .tera file. Show input, command, output. "That's a template. You just made one."
2. **Adding variables** — string, bool, select, multiselect. One example each.
3. **Conditional files** — when expressions, conditional file inclusion. Real use case (CI config only when use_ci is true).
4. **Computed variables** — derived values like slugified project names.
5. **File handling** — exclude patterns, copy-without-render for binaries.
6. **Tera basics** — teach the 5 things they'll actually use: {{ var }}, {% if %}, {% for %}, filters (slugify, upper), comments. Link to full Tera docs for the rest. Don't assume Jinja2 knowledge.
7. **Validating your template** — diecut check and diecut ready.

Use counterexamples (from just's pattern): show common mistakes and what happens when you make them.

### 5. Migrating from Cookiecutter (migrating-from-cookiecutter.md)

Short, practical. Not a comparison — a how-to.

- What diecut migrate does (format differences, what gets translated)
- Dry-run first, migrate second
- What doesn't translate (Python hooks to Rhai hooks, Jinja2 extensions)
- Side-by-side: cookiecutter.json vs diecut.toml for the same template

### 6. Commands Reference (reference/commands.md)

Expand the existing page. Consistent template per command:

- **Synopsis** — command signature
- **Options table** — flag, type, default, description
- **Examples** — 2-3 real examples with output
- **Notes** — edge cases, gotchas

Commands: new, check, ready, update, migrate, list.

### 7. diecut.toml Reference (reference/diecut-toml.md)

Exhaustive config reference. Two-tier pattern (from just):

- **Summary table** at top — every key, type, default, one-line description. Scannable.
- **Detailed sections** below — one per config key, with full explanation, example TOML, and rendered output.

Grouped by function: template metadata, variables, files, hooks. Not alphabetical.

### 8. Hooks Reference (reference/hooks.md)

Rhai hooks guide + reference in one page.

- What hooks are and when they run (pre_generate, post_generate)
- Minimal example
- Available API (variable() function, filesystem access, etc.)
- Real-world examples (conditional file cleanup, git init, printing messages)
- Link to Rhai docs for language reference

## Writing Style Guide

Based on the author's blog voice:

- **Short declarative sentences.** "Templates are just folders." Not "Templates are implemented as directory structures that contain..."
- **Concrete before abstract.** Show the example first, explain after.
- **First person for stories, second person for teaching.** "I built this because..." / "You'll need a diecut.toml..."
- **Skeptical of complexity.** If something is simple, say so. If something is complex, acknowledge it honestly (Ghostty pattern).
- **No hedging.** Don't say "you might want to consider." Say "do this."
- **No emoji.** Warm tone comes from word choice, not decoration.
- **UNIX sensibility.** diecut does one thing well. The docs should reflect that.

## What We're NOT Building

- No FAQ page (answers belong inline where questions arise)
- No comparison page (let the docs speak for themselves)
- No template gallery (maintenance burden, needs curation)
- No blog or changelog page (maintenance traps)
- No versioned docs (just "latest" — Starlight default)
- No internationalization (nice-to-have someday, not now)

## Research Sources

Analyzed 5 documentation sites to inform this design:

- **cookiecutter** (readthedocs) — good progressive hierarchy, two-tutorial approach, but stale and weak config reference
- **copier** (readthedocs) — excellent config reference page, strong update workflow docs, but dense and weak progressive disclosure
- **just** (just.systems) — extraordinary example density, two-tier reference pattern, counterexamples, honest about limitations
- **ghostty** (ghostty.org) — philosophy-first config docs, honest human tone, layered depth
- **starship** (starship.rs) — extreme template consistency, brand personality, tabbed selectors, preset gallery
