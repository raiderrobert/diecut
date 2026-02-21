---
title: Repeating a pattern within a project
description: Your codebase already has the right shape. Make it official.
---

You have a React/TypeScript app where every feature module looks the same: `index.ts`, `store.ts`, `api.ts`, `types.ts`. When you need a new feature, you copy `products/` and start renaming. You get `Order`, `OrderState`, `fetchOrders` — but the import in `api.ts` still reads `import type { Product } from './types'`. TypeScript catches that one. The `fetch('/api/products')` URL in the same file does not cause a type error. It silently hits the wrong endpoint until someone notices orders returning product data.

This tutorial shows you how to extract the pattern into a template that lives inside the project. Adding the next feature is a single command.

## The existing pattern

Your `src/features/` directory has two modules already:

```text
src/features/
  users/
    index.ts
    store.ts
    api.ts
    types.ts
  products/
    index.ts
    store.ts
    api.ts
    types.ts
```

Each module follows the same conventions: types are exported from `types.ts`, the API layer imports from there, and `index.ts` re-exports everything. You want `orders/` to look exactly the same.

## What copy-paste produces

Here is what `orders/api.ts` looks like after a hurried copy from `products/`:

```typescript
// orders/api.ts — after copying from products/
import type { Product } from './types';         // wrong — should be Order

export async function fetchOrders(): Promise<Product[]> {
  const response = await fetch('/api/products'); // wrong URL — silent bug
  return response.json();
}
```

This compiles. TypeScript sees no error because `Product[]` is a valid return type. The fetch URL is wrong and nothing will tell you until an order view renders product data.

## Create the template directory

Add a `_template/` directory alongside the existing modules. The underscore signals to your team that this is not a real feature.

```text
src/features/
  _template/
    diecut.toml
    template/
  users/
    ...
  products/
    ...
```

Everything under `_template/template/` becomes the generated module. Files ending in `.tera` are rendered through the Tera template engine and have the suffix stripped. Everything else is copied as-is.

## Write the config

Create `src/features/_template/diecut.toml`:

```toml
[template]
name = "feature-module"

[variables.feature_name]
type = "string"
prompt = "Feature name"
default = "my-feature"
validation = '^[a-z][a-z0-9-]*$'
validation_message = "Must start with a letter. Only lowercase letters, numbers, hyphens."

[variables.FeatureName]
type = "string"
computed = "{{ feature_name | replace(from='-', to=' ') | title | replace(from=' ', to='') }}"
```

`feature_name` is the kebab-case slug the user types — `orders`, `shopping-cart`, `line-items`. The validation rejects anything that would produce a broken import path.

`FeatureName` is computed from it. Tera's `title` filter capitalises each word, then the two `replace` calls strip hyphens and spaces, giving you `Orders`, `ShoppingCart`, `LineItems`. This is what you use wherever TypeScript expects a type name or interface prefix. Computed variables are never prompted — diecut derives them automatically.

## Add the template files

### types.ts

Create `src/features/_template/template/types.ts.tera`:

```typescript
export interface {{ FeatureName }} {
  id: string;
}

export interface {{ FeatureName }}State {
  items: {{ FeatureName }}[];
  loading: boolean;
  error: string | null;
}
```

### api.ts

Create `src/features/_template/template/api.ts.tera`:

```typescript
import type { {{ FeatureName }} } from './types';

export async function fetch{{ FeatureName }}s(): Promise<{{ FeatureName }}[]> {
  const response = await fetch('/api/{{ feature_name }}');
  return response.json();
}

export async function fetch{{ FeatureName }}(id: string): Promise<{{ FeatureName }}> {
  const response = await fetch(`/api/{{ feature_name }}/${id}`);
  return response.json();
}
```

### index.ts

Create `src/features/_template/template/index.ts.tera`:

```typescript
export type { {{ FeatureName }}, {{ FeatureName }}State } from './types';
export { fetch{{ FeatureName }}s, fetch{{ FeatureName }} } from './api';
```

Your template directory now looks like this:

```text
src/features/_template/
  diecut.toml
  template/
    types.ts.tera
    api.ts.tera
    index.ts.tera
```

## Preview before writing

Run with `--dry-run --verbose` to see what diecut would generate without touching the filesystem:

```bash
diecut new ./src/features/_template -o src/features/orders --dry-run --verbose
```

```text
Feature name [my-feature]: orders

[dry-run] would write: src/features/orders/types.ts
[dry-run] would write: src/features/orders/api.ts
[dry-run] would write: src/features/orders/index.ts
```

Check the filenames. If they look right, generate for real:

```bash
diecut new ./src/features/_template -o src/features/orders
```

## The result

```text
src/features/orders/
  types.ts
  api.ts
  index.ts
  .diecut-answers.toml
```

The generated `types.ts`:

```typescript
export interface Order {
  id: string;
}

export interface OrderState {
  items: Order[];
  loading: boolean;
  error: string | null;
}
```

The generated `api.ts`:

```typescript
import type { Order } from './types';

export async function fetchOrders(): Promise<Order[]> {
  const response = await fetch('/api/orders');
  return response.json();
}

export async function fetchOrder(id: string): Promise<Order> {
  const response = await fetch(`/api/orders/${id}`);
  return response.json();
}
```

The `import type { Order }` on line 1 and `fetch('/api/orders')` on line 5 both derive from the single value `orders` typed at the prompt. There is no second place to update.

## The key insight

You did not design a template from scratch. The pattern was already in your codebase — in `users/` and `products/`. You extracted it, named the parts that change, and wrote it down. Now adding a new feature module is a single command instead of a copy-paste session with a find-and-replace at the end.

The template lives in the project it serves. Your teammates find it where they would look for it. It evolves alongside the codebase.

When the pattern changes — say, you add a `store.ts` to every module — without a template you add it to `users/`, `products/`, and `orders/`, but six months from now someone adds `notifications/` by copying an older module and it ships without a store, inconsistent with everything else. With the template, you add `store.ts.tera` once. Every module created after that gets it.

---

To learn more about computed variables, filters, and other template features, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
