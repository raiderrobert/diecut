---
title: New packages in a monorepo
description: Keep a template in your repo. Add packages in seconds.
---

You're adding the Nth package to a TypeScript monorepo. Your options: copy an existing package and rename everything by hand, or ask an LLM and hope it gets all the filenames right. Both are slow and error-prone.

There's a better way. Keep a template in the repo itself. Run one command. The package name propagates to every file automatically.

## The monorepo before

```text
acme/
  _templates/
    package/          ← template lives here
  packages/
    auth/
      package.json
      tsconfig.json
      src/
        index.ts
        auth.test.ts
    payments/
      package.json
      tsconfig.json
      src/
        index.ts
        payments.test.ts
  turbo.json
  package.json
```

`auth` and `payments` already exist. You're adding `analytics`.

## The template

```text
_templates/package/
  diecut.toml
  template/
    package.json.tera
    tsconfig.json
    src/
      index.ts.tera
      index.test.ts.tera
```

### diecut.toml

```toml
[template]
name = "package"

[variables.package_name]
type = "string"
prompt = "Package name"
validation = '^[a-z][a-z0-9-]*$'
validation_message = "Lowercase letters, numbers, and hyphens only."

[variables.package_scope]
type = "string"
computed = "@acme/{{ package_name }}"
```

Two variables. `package_name` is prompted once. `package_scope` is computed from it — never shown to the user, always consistent.

### template/package.json.tera

```json
{
  "name": "{{ package_scope }}",
  "version": "0.0.0",
  "private": true,
  "main": "./src/index.ts",
  "scripts": {
    "build": "tsc",
    "test": "vitest run"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^1.0.0"
  }
}
```

### template/tsconfig.json

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src"]
}
```

No variables here — this file is identical for every package. diecut copies it as-is.

### template/src/index.ts.tera

```typescript
/**
 * {{ package_scope }}
 */

export {};
```

### template/src/index.test.ts.tera

```typescript
import { describe, it } from "vitest";

describe("{{ package_name }}", () => {
  it("has tests", () => {
    // add your tests here
  });
});
```

`package_name` appears in the `describe` block. `package_scope` appears in `package.json` and the index comment. Both come from the same single answer.

## Run it

From the monorepo root:

```bash
diecut new ./_templates/package -o packages/analytics
```

diecut prompts for one variable:

```text
Package name: analytics
```

That's it.

## Preview before writing

Not sure what you'll get? Add `--dry-run --verbose` to see the rendered output without writing any files:

```bash
diecut new ./_templates/package -o packages/analytics --dry-run --verbose
```

## The result

```text
packages/analytics/
  package.json
  tsconfig.json
  src/
    index.ts
    index.test.ts
  .diecut-answers.toml
```

The generated `package.json`:

```json
{
  "name": "@acme/analytics",
  "version": "0.0.0",
  "private": true,
  "main": "./src/index.ts",
  "scripts": {
    "build": "tsc",
    "test": "vitest run"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^1.0.0"
  }
}
```

The generated `src/index.ts`:

```typescript
/**
 * @acme/analytics
 */

export {};
```

The generated `src/index.test.ts`:

```typescript
import { describe, it } from "vitest";

describe("analytics", () => {
  it("has tests", () => {
    // add your tests here
  });
});
```

`analytics` was typed once. It appears correctly in `package.json` (as `@acme/analytics`), in `index.ts` (as `@acme/analytics`), and in the test file (as `analytics`). No find-and-replace. No missed occurrences.

## What your team gets

The `_templates/` directory is committed to the repo. Everyone on the team runs the same command and gets the same result. When the team decides every package needs a new file — say, a `CHANGELOG.md` or a `vitest.config.ts` — you add it to the template once. The next package generated picks it up automatically. Existing packages are unaffected.

To add a second package later:

```bash
diecut new ./_templates/package -o packages/notifications
```

Same template, new name, done in seconds.

---

To learn more about computed variables and template features, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
