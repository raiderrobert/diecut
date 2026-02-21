---
title: Scaffolding a FastAPI resource
description: "Add a new endpoint. Enter the name once. Get the router, schemas, and tests."
---

Every new resource in a FastAPI application means three files: a router, a Pydantic schema module, and a test file. The resource name appears in class names, route paths, function names, and import paths across all of them. Copy-paste works until it doesn't — one unrenamed `ProductSchema` hiding in what's supposed to be the orders module, and the error is silent until runtime.

There's a better way. Write the pattern down as a template. Type the name once. All three files generate correctly, every time.

## What copy-paste produces

You copy `products/schemas.py` to `orders/schemas.py` and rename `Product` to `Order` in four places:

```python
# orders/schemas.py — after copy-paste
from pydantic import BaseModel

class OrderBase(BaseModel):
    pass

class OrderCreate(OrderBase):
    pass

class ProductUpdate(OrderBase):   # missed this one
    pass

class OrderResponse(OrderBase):
    id: int
```

The router imports `OrderUpdate`. Python resolves it to `ProductUpdate` because that is what `schemas.py` exports under that name — wait, it doesn't export `OrderUpdate` at all. The import fails immediately on startup. If you catch it then, you're lucky. If you renamed it to `OrderUpdate` but left the class body wrong, the app starts, validates with the wrong schema, and you find out three sprints later when a field mismatch causes a 422.

## The template structure

Create a template directory inside your repo:

```text
templates/
  resource/
    diecut.toml
    template/
      router.py.tera
      schemas.py.tera
      test_{{ resource_plural }}.py.tera
```

The test filename contains `{{ resource_plural }}`. diecut renders path components through Tera, so `test_{{ resource_plural }}.py.tera` becomes `test_orders.py` in the output.

## diecut.toml

```toml
[template]
name = "resource"

[variables.resource_name]
type = "string"
prompt = "Resource name (singular, snake_case)"
default = "resource"
validation = '^[a-z][a-z0-9_]*$'
validation_message = "Must start with a letter. Only lowercase letters, numbers, and underscores."

[variables.ResourceName]
type = "string"
computed = "{{ resource_name | replace(from='_', to=' ') | title | replace(from=' ', to='') }}"

[variables.resource_plural]
type = "string"
computed = "{{ resource_name }}s"
```

Three variables, one prompt.

`resource_name` is the only one shown to the user. Enter `order` — singular, because Python class names are conventionally singular. `ResourceName` is computed from it: underscores replaced with spaces, title-cased, spaces removed — turning `order` into `Order` and `line_item` into `LineItem`. `resource_plural` appends an `s` for use in route paths, test function names, and the test filename.

Computed variables are never prompted. They're always derived from the value the user typed.

The `s` suffix works for regular nouns: `order` → `orders`, `invoice` → `invoices`, `line_item` → `line_items`. Irregular plurals like `person` or `category` need a manual rename in the generated files after running.

Without this, `line_item` → `LineItem` → `line_items` happens in three separate places that you type manually. Someone generates the router with `line_item`, the schema with `lineitem` (missing the underscore), and the test with `LineItems` (accidentally plural). The app wires up, the import fails at startup, and you spend ten minutes wondering why `LineItemCreate` isn't found. With computed variables, the derivation is in the template — there is no opportunity to type the name differently across files.

## Template files

### The router

`template/router.py.tera`:

```python
from fastapi import APIRouter, HTTPException
from .schemas import {{ ResourceName }}Create, {{ ResourceName }}Update, {{ ResourceName }}Response

router = APIRouter(prefix="/{{ resource_plural }}", tags=["{{ resource_plural }}"])

@router.get("/", response_model=list[{{ ResourceName }}Response])
async def list_{{ resource_plural }}():
    ...

@router.get("/{id}", response_model={{ ResourceName }}Response)
async def get_{{ resource_name }}(id: int):
    ...

@router.post("/", response_model={{ ResourceName }}Response, status_code=201)
async def create_{{ resource_name }}(data: {{ ResourceName }}Create):
    ...

@router.patch("/{id}", response_model={{ ResourceName }}Response)
async def update_{{ resource_name }}(id: int, data: {{ ResourceName }}Update):
    ...

@router.delete("/{id}", status_code=204)
async def delete_{{ resource_name }}(id: int):
    ...
```

`ResourceName` appears in the import line and every response model annotation. `resource_plural` sets the route prefix and the tag. `resource_name` names each handler function. All three come from the single value the user typed.

### The schemas

`template/schemas.py.tera`:

```python
from pydantic import BaseModel

class {{ ResourceName }}Base(BaseModel):
    pass

class {{ ResourceName }}Create({{ ResourceName }}Base):
    pass

class {{ ResourceName }}Update({{ ResourceName }}Base):
    pass

class {{ ResourceName }}Response({{ ResourceName }}Base):
    id: int

    model_config = {"from_attributes": True}
```

Every class name is built from `ResourceName`. The inheritance chain — `OrderBase`, `OrderCreate`, `OrderUpdate`, `OrderResponse` — stays consistent because the same variable drives all four.

### The tests

`template/test_{{ resource_plural }}.py.tera`:

```python
import pytest
from fastapi.testclient import TestClient
from app.main import app

client = TestClient(app)

def test_list_{{ resource_plural }}():
    response = client.get("/{{ resource_plural }}/")
    assert response.status_code == 200

def test_create_{{ resource_name }}():
    response = client.post("/{{ resource_plural }}/", json={})
    assert response.status_code == 201
```

The test function names, the route paths in the assertions, and the filename all derive from the same two computed variables. Nothing to rename manually.

## Run it

From the repo root:

```bash
diecut new ./templates/resource -o src/resources/orders
```

diecut prompts for one variable:

```text
Resource name (singular, snake_case) [resource]: order
```

That's it.

Preview first with `--dry-run --verbose` if you want to see the rendered output before writing:

```bash
diecut new ./templates/resource -o src/resources/orders --dry-run --verbose
```

To skip the prompt entirely:

```bash
diecut new ./templates/resource -o src/resources/orders -d resource_name=order
```

## The result

```text
src/resources/orders/
  router.py
  schemas.py
  test_orders.py
  .diecut-answers.toml
```

From the generated `router.py`:

```python
from .schemas import OrderCreate, OrderUpdate, OrderResponse

router = APIRouter(prefix="/orders", tags=["orders"])

@router.get("/", response_model=list[OrderResponse])
async def list_orders():
    ...

@router.post("/", response_model=OrderResponse, status_code=201)
async def create_order(data: OrderCreate):
    ...
```

From the generated `schemas.py`:

```python
class OrderBase(BaseModel):
    pass

class OrderCreate(OrderBase):
    pass

class OrderUpdate(OrderBase):
    pass

class OrderResponse(OrderBase):
    id: int
```

`Order`, `OrderCreate`, `OrderUpdate`, `OrderResponse` — consistent across the router's imports, the response model annotations, and the schema definitions. The name was typed once.

## Next resource

Adding invoices:

```bash
diecut new ./templates/resource -o src/resources/invoices -d resource_name=invoice
```

Adding line items:

```bash
diecut new ./templates/resource -o src/resources/line_items -d resource_name=line_item
```

`line_item` becomes `LineItem`, `LineItemCreate`, `LineItemResponse` — and `line_items` in the route prefix, the test filename, and the test function names.

## The alignment guarantee

With copy-paste, you find out when it fails. A `ProductUpdate` that survived the rename shows up in a 422 traceback pointing at Pydantic, not at the copy-paste. You diff two schema files to find it.

With a template, generation is atomic. Every class name, route prefix, and test function name is rendered from the same `resource_name` at the same moment. The only way to get a mismatch is to edit the generated files afterward — at which point the diff makes it obvious.

---

To learn more about computed variables and validation, see [Creating Templates](/creating-templates/). For all CLI options, see the [Commands reference](/reference/commands/).
