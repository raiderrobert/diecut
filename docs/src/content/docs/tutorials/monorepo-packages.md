---
title: New packages in a monorepo
description: Keep a template in your repo. Add packages in seconds.
---

You're adding the Nth package to a TypeScript monorepo. You copy `packages/auth/` to `packages/analytics/`, rename the directory, update `package.json`, update `tsconfig.json`. You open `index.ts` and the JSDoc comment still says `@acme/auth`. You grep for `auth` and find it in the `describe()` block in `index.test.ts` too. Then a week later CI fails because someone missed `auth` in a file that wasn't open at the time.

With a template in the repo, you type the package name once.

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

Two variables. `package_name` is prompted once. `package_scope` is derived from it automatically.

In the copy-paste workflow, this is where mistakes happen: `package.json` gets updated to `@acme/analytics` but the JSDoc comment in `index.ts` still says `@acme/auth` because it was easy to miss. Here, both come from the same computed value — if one is right, they're all right.

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

`analytics` was typed once. Notice that `package.json` and `index.ts` use `@acme/analytics` (the scoped name) while `index.test.ts` uses `analytics` (the bare name). In a copy-paste workflow, getting these two forms right across all files is exactly the step that gets missed under time pressure. Here, both are computed from the same `package_name`.

## What your team gets

The `_templates/` directory is committed to the repo. Everyone on the team runs the same command and gets the same result.

When the team decides every package needs a `vitest.config.ts`: without a template, someone writes the config for `analytics`, then someone else adds a slightly different one to `payments`, and now you have two diverging configurations to reconcile. With the template, you add `vitest.config.ts` once to `_templates/package/template/`. The next `diecut new` picks it up. Packages that already exist keep their own files — you update them separately, on your own schedule, if at all.

To add a second package later:

```bash
diecut new ./_templates/package -o packages/notifications
```

Same template, new name, done in seconds.

---

To learn more about computed variables and template features, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
