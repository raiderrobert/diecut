---
title: Scaffolding a multi-file feature
description: Enter a name once. Get all the files.
---

Every new entity in a TypeScript REST API means four files: a controller, a service, a repository, and a test. The entity name appears in class names, method names, import paths, and describe blocks across all of them.

You scaffold the orders module by copying `products/`. You rename five of the six occurrences of `ProductsController`. The file compiles and the tests pass — `describe('ProductsController', ...)` in the test file matches the class name you forgot to rename in the implementation. Two weeks later, an error log shows `ProductsController` handling an orders request. You grep for it and find the test that confirmed the wrong thing.

Write the pattern down as a template. Type the name once. All four files generate correctly, every time.

## What copy-paste produces

After copying `products.controller.ts` to `orders.controller.ts` and renaming most occurrences:

```typescript
// orders.controller.ts
import { OrdersService } from './orders.service';       // renamed

@Controller('orders')
export class ProductsController {                       // missed this one
  constructor(private readonly ordersService: OrdersService) {}
}
```

```typescript
// orders.controller.test.ts
describe('ProductsController', () => {                  // missed this too
  let controller: OrdersController;                     // renamed
  ...
```

The test imports `OrdersController` but the describe block still reads `ProductsController`. The test passes. A Jest describe label is just a string — TypeScript does not check it.

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

`entity_name` is the only one shown to the user. `EntityName` is computed from it: hyphens replaced with spaces, title-cased, spaces removed — turning `orders` into `Orders` and `line-items` into `LineItems`.

Computed variables are never prompted. They're always derived from the value the user typed.

Without computed variables, `OrdersController` in the class name and `OrdersService` in the import are typed separately — two strings, no enforced relationship. Here, both are rendered from `entity_name`. If you change `entity_name`, both change.

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
  constructor(private readonly service: {{ EntityName }}Service) {}

  @Post()
  create(@Body() dto: Create{{ EntityName }}Dto) {
    return this.service.create(dto);
  }

  @Get()
  findAll() {
    return this.service.findAll();
  }

  @Get(':id')
  findOne(@Param('id') id: string) {
    return this.service.findOne(id);
  }

  @Put(':id')
  update(@Param('id') id: string, @Body() dto: Update{{ EntityName }}Dto) {
    return this.service.update(id, dto);
  }

  @Delete(':id')
  remove(@Param('id') id: string) {
    return this.service.remove(id);
  }
}
```

`EntityName` appears in the class name, import paths, and DTO references. `entity_name` sets the route path. Both come from the single value the user typed.

### The test

`template/{{ entity_name }}.controller.test.ts.tera`:

```typescript
import { Test, TestingModule } from '@nestjs/testing';
import { {{ EntityName }}Controller } from './{{ entity_name }}.controller';
import { {{ EntityName }}Service } from './{{ entity_name }}.service';

describe('{{ EntityName }}Controller', () => {
  let controller: {{ EntityName }}Controller;
  let service: jest.Mocked<{{ EntityName }}Service>;

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
    service = module.get({{ EntityName }}Service);
  });

  it('should be defined', () => {
    expect(controller).toBeDefined();
  });

  it('findAll delegates to service', async () => {
    service.findAll.mockResolvedValue([]);
    const result = await controller.findAll();
    expect(service.findAll).toHaveBeenCalled();
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
  constructor(private readonly service: OrdersService) {}
  ...
}
```

From the generated `orders.controller.test.ts`:

```typescript
describe('OrdersController', () => {
  let controller: OrdersController;
  let service: jest.Mocked<OrdersService>;
  ...
});
```

`OrdersController`, `OrdersService` — class names and import paths rendered from `entity_name = 'orders'`. In a copy-paste workflow, each is typed separately. Any one can diverge. Here, there is one string and two renderings of it.

## Next entity

Adding invoices:

```bash
diecut new ./templates/endpoint -o src/endpoints/invoices -d entity_name=invoices
```

Adding line items:

```bash
diecut new ./templates/endpoint -o src/endpoints/line-items -d entity_name=line-items
```

`line-items` becomes `LineItemsController` and `LineItemsService` — the computed variable handles the casing transform.

## The difference

Copy-paste requires you to find every occurrence of a name and rename each one correctly. Miss one and nothing breaks immediately — the test still runs, the service still starts. The failure shows up later: a describe block that says `ProductsController` while you're debugging `OrdersService`, a class name in an error log that doesn't match the file you're reading.

The describe block example in this article is exactly that failure. The test imported `OrdersController` but the describe label still read `ProductsController`. It passed. TypeScript has no opinion on describe strings.

A template has one input — `entity_name` — and everything else derives from it at generation time. There's nothing to rename and nothing to miss.

---

To learn more about computed variables and validation, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
