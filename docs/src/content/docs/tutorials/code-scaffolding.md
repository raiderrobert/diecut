---
title: Scaffolding a multi-file feature
description: Enter a name once. Get all the files.
---

Every new entity in a TypeScript REST API means four files: a controller, a service, a repository, and a test. The entity name appears in class names, method names, import paths, and describe blocks across all of them. Copy-paste works until it doesn't — one unrenamed `ProductsController` hiding in what's supposed to be the orders module, and the error doesn't surface until runtime.

There's a better way. Write the pattern down as a template. Type the name once. All four files generate correctly, every time.

## The template structure

Create a template directory inside your repo:

```text
templates/
  endpoint/
    diecut.toml
    template/
      {{ entity_name }}.controller.ts.tera
      {{ entity_name }}.service.ts.tera
      {{ entity_name }}.repository.ts.tera
      {{ entity_name }}.controller.test.ts.tera
```

The filenames themselves contain `{{ entity_name }}`. diecut renders path components through Tera, so `{{ entity_name }}.controller.ts.tera` becomes `orders.controller.ts` in the output.

## diecut.toml

```toml
[template]
name = "endpoint"

[variables.entity_name]
type = "string"
prompt = "Entity name (kebab-case)"
default = "entity"
validation = '^[a-z][a-z0-9-]*$'
validation_message = "Must start with a letter. Only lowercase letters, numbers, and hyphens."

[variables.EntityName]
type = "string"
computed = "{{ entity_name | replace(from='-', to=' ') | title | replace(from=' ', to='') }}"
```

Two variables, one prompt.

`entity_name` is the only one shown to the user. `EntityName` is computed from it: hyphens replaced with spaces, title-cased, spaces removed — turning `orders` into `Orders` and `line-items` into `LineItems`. Use `entity_name` (with hyphens stripped) wherever a camelCase identifier is needed; for the common case of single-word entities like `orders`, `entity_name` and camelCase are identical.

Computed variables are never prompted. They're always derived, always consistent.

## Template files

### The controller

`template/{{ entity_name }}.controller.ts.tera`:

```typescript
import { Controller, Get, Post, Put, Delete, Param, Body } from '@nestjs/common';
import { {{ EntityName }}Service } from './{{ entity_name }}.service';
import { Create{{ EntityName }}Dto } from './dto/create-{{ entity_name }}.dto';
import { Update{{ EntityName }}Dto } from './dto/update-{{ entity_name }}.dto';

@Controller('{{ entity_name }}')
export class {{ EntityName }}Controller {
  constructor(private readonly {{ entity_name | replace(from='-', to='') }}Service: {{ EntityName }}Service) {}

  @Post()
  create(@Body() dto: Create{{ EntityName }}Dto) {
    return this.{{ entity_name | replace(from='-', to='') }}Service.create(dto);
  }

  @Get()
  findAll() {
    return this.{{ entity_name | replace(from='-', to='') }}Service.findAll();
  }

  @Get(':id')
  findOne(@Param('id') id: string) {
    return this.{{ entity_name | replace(from='-', to='') }}Service.findOne(id);
  }

  @Put(':id')
  update(@Param('id') id: string, @Body() dto: Update{{ EntityName }}Dto) {
    return this.{{ entity_name | replace(from='-', to='') }}Service.update(id, dto);
  }

  @Delete(':id')
  remove(@Param('id') id: string) {
    return this.{{ entity_name | replace(from='-', to='') }}Service.remove(id);
  }
}
```

`EntityName` appears in the class name and import paths. `entityName` appears in the injected service property. Both come from `entity_name` — the single value the user typed.

### The test

`template/{{ entity_name }}.controller.test.ts.tera`:

```typescript
import { Test, TestingModule } from '@nestjs/testing';
import { {{ EntityName }}Controller } from './{{ entity_name }}.controller';
import { {{ EntityName }}Service } from './{{ entity_name }}.service';

describe('{{ EntityName }}Controller', () => {
  let controller: {{ EntityName }}Controller;
  let {{ entity_name | replace(from='-', to='') }}Service: jest.Mocked<{{ EntityName }}Service>;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      controllers: [{{ EntityName }}Controller],
      providers: [
        {
          provide: {{ EntityName }}Service,
          useValue: {
            create: jest.fn(),
            findAll: jest.fn(),
            findOne: jest.fn(),
            update: jest.fn(),
            remove: jest.fn(),
          },
        },
      ],
    }).compile();

    controller = module.get<{{ EntityName }}Controller>({{ EntityName }}Controller);
    {{ entity_name | replace(from='-', to='') }}Service = module.get({{ EntityName }}Service);
  });

  it('should be defined', () => {
    expect(controller).toBeDefined();
  });

  it('findAll delegates to service', async () => {
    {{ entity_name | replace(from='-', to='') }}Service.findAll.mockResolvedValue([]);
    const result = await controller.findAll();
    expect({{ entity_name | replace(from='-', to='') }}Service.findAll).toHaveBeenCalled();
    expect(result).toEqual([]);
  });
});
```

The test file references the controller class, the service class, and the injected service variable — all using the same computed variables. No manual substitution, no risk of a stale name in the `describe` block.

## Run it

From the repo root:

```bash
diecut new ./templates/endpoint -o src/endpoints/orders
```

diecut prompts for one variable:

```text
Entity name (kebab-case) [entity]: orders
```

That's it.

Preview first with `--dry-run --verbose` if you want to see the output before writing:

```bash
diecut new ./templates/endpoint -o src/endpoints/orders --dry-run --verbose
```

To skip the prompt entirely:

```bash
diecut new ./templates/endpoint -o src/endpoints/orders -d entity_name=orders
```

## The result

```text
src/endpoints/orders/
  orders.controller.ts
  orders.service.ts
  orders.repository.ts
  orders.controller.test.ts
  .diecut-answers.toml
```

From the generated `orders.controller.ts`:

```typescript
@Controller('orders')
export class OrdersController {
  constructor(private readonly ordersService: OrdersService) {}
  ...
}
```

From the generated `orders.controller.test.ts`:

```typescript
describe('OrdersController', () => {
  let controller: OrdersController;
  let ordersService: jest.Mocked<OrdersService>;
  ...
});
```

`OrdersController`, `ordersService`, `OrdersService` — all consistent, all correct. The entity name was typed once.

## Next entity

Adding invoices:

```bash
diecut new ./templates/endpoint -o src/endpoints/invoices -d entity_name=invoices
```

Adding line items:

```bash
diecut new ./templates/endpoint -o src/endpoints/line-items -d entity_name=line-items
```

`line-items` becomes `LineItemsController`, `lineItemsService`, `LineItemsService` — the computed variables handle the casing transforms.

## The alignment guarantee

With copy-paste, a mismatch is always possible. You rename three occurrences and miss a fourth. The file compiles. The bug waits.

With a template, the entity name is a single source of truth. Every class name, every import path, every test describe block is derived from it at generation time. There's no fourth occurrence to miss. The files are either all right or all wrong — and you find out immediately, not at runtime.

---

To learn more about computed variables and validation, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
