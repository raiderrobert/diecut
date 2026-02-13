# Astro CF Template Migration to Diecut

## Overview

Convert `astro-cf-template` from a Claude Code bootstrap-skill-driven template into a native diecut template. The new template lives in a separate directory (`astro-cf-diecut-template`) and achieves full parity with the existing `/bootstrap` skill.

## Directory Structure

```
astro-cf-diecut-template/
├── diecut.toml
├── README.md
└── template/
    └── {{ project_name }}/
        ├── .claude/skills/          (8 skills, each conditional)
        ├── .github/
        ├── adr/                     (conditional: include_adr)
        ├── business/                (conditional: include_business_docs)
        ├── public/
        ├── src/
        │   ├── components/Header.astro.tera
        │   ├── content/config.ts
        │   ├── layouts/Layout.astro.tera
        │   ├── layouts/MarkdownLayout.astro
        │   ├── pages/index.astro.tera
        │   ├── pages/sitemap.xml.js.tera
        │   ├── styles/global.css.tera
        │   ├── styles/markdown.css
        │   ├── types/metadata.ts
        │   └── utils/metadata.ts.tera
        ├── .env.example
        ├── .gitignore
        ├── astro.config.mjs.tera
        ├── biome.json
        ├── package.json.tera
        ├── pnpm-lock.yaml
        ├── pnpm-workspace.yaml
        ├── tsconfig.json
        ├── vitest.config.ts
        └── wrangler.toml.tera
```

## Variables (14 total)

### Core (3)

| Variable | Type | Default | Validation |
|----------|------|---------|------------|
| `project_name` | string | `my-project` | `^[a-z][a-z0-9-]*$` |
| `site_description` | string | `A fast, modern website built with Astro` | — |
| `site_url` | string | `https://example.com` | `^https://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}` |

### Colors (3)

| Variable | Type | Default | Validation |
|----------|------|---------|------------|
| `color_primary` | string | `#3b82f6` | `^#[0-9a-fA-F]{6}$` |
| `color_secondary` | string | `#8b5cf6` | `^#[0-9a-fA-F]{6}$` |
| `color_accent` | string | `#f59e0b` | `^#[0-9a-fA-F]{6}$` |

### Font (1)

| Variable | Type | Choices | Default |
|----------|------|---------|---------|
| `font` | select | System (default), Inter + Outfit, Roboto + Roboto Slab, Open Sans + Merriweather, Lato + Playfair Display | System (default) |

### Feature toggles (8)

| Variable | Type | Default | Controls |
|----------|------|---------|----------|
| `include_adr` | bool | true | `adr/` dir + `.claude/skills/create-adr/` |
| `include_business_docs` | bool | true | `business/` dir + `.claude/skills/business-context/` |
| `include_blog_skill` | bool | true | `.claude/skills/add-blog/` |
| `include_api_skill` | bool | true | `.claude/skills/add-api/` |
| `include_stripe_skill` | bool | true | `.claude/skills/add-stripe/` |
| `include_auth_skill` | bool | true | `.claude/skills/add-auth/` |
| `include_ci_skill` | bool | true | `.claude/skills/add-ci/` |
| `include_analytics_skill` | bool | true | `.claude/skills/add-analytics/` |

## Templated Files (7 files get `.tera` suffix)

Files with simple variable substitution:
- `package.json.tera` — `project_name`, `site_description`
- `wrangler.toml.tera` — `project_name`
- `astro.config.mjs.tera` — `site_url`
- `Header.astro.tera` — `project_name`
- `index.astro.tera` — `project_name`, `site_description`
- `sitemap.xml.js.tera` — `site_url`
- `metadata.ts.tera` — `site_url`

Files with conditional logic:
- `global.css.tera` — color hex values + `{% if font == "..." %}` blocks for body/heading font-family
- `Layout.astro.tera` — `{% if font != "System (default)" %}` block for Google Fonts `<link>` tags

## Font Handling

Font logic lives in `.tera` files using `{% if %}` / `{% elif %}` chains:
- `global.css.tera`: Sets `font-family` on `body` and optionally adds heading font rule
- `Layout.astro.tera`: Conditionally injects Google Fonts `<link>` tags in `<head>`

No computed variables needed — keeps `diecut.toml` clean.

## File Rules

- **Excluded:** `.DS_Store`, `node_modules/**`
- **Copy without render:** `pnpm-lock.yaml`, `public/**`
- **Conditional:** 10 patterns (adr, business, 6 skills × their `.claude/skills/` dirs, plus adr and business skill dirs)

## Curly Brace Handling

Astro uses `{expression}` (single braces). Tera uses `{{ var }}` (double braces). No syntactic conflict. Audit during implementation for any literal `{{` in source files; use `{% raw %}` blocks if needed.

## Exclusions from Template

- `node_modules/` — users run `pnpm install`
- `dist/` — build output
- `.git/` — not part of template
- `.claude/skills/bootstrap/` — replaced by diecut

## Approach

Flat template with Tera conditionals (Approach A). All files in a single `template/` directory. Conditional file inclusion via `[files.conditional]` in `diecut.toml`. Font/color logic embedded in `.tera` files.
