---
title: Structured content with consistent schemas
description: "Define the shape once. Every entry conforms — whether filled in by hand or by an LLM."
---

You maintain a conference listing site. Every event has the same fields: name, dates, location, ticket price, tags. Someone adds an event by hand and invents a tag that doesn't exist in your system. Another entry has `ticketCost` instead of `ticket_price`. A third skips `featured` entirely.

The frontmatter drifts. Queries break. You spend time cleaning up instead of publishing.

The fix is not a style guide in a README. It's a template that enforces the schema at the point of entry — and refuses to accept values it doesn't know about.

## Set up the template

Create a directory for the event template:

```text
templates/
  event/
    diecut.toml
    template/
      event.md.tera
```

## Write the config

Create `templates/event/diecut.toml`:

```toml
[template]
name = "event"

[variables.name]
type = "string"
prompt = "Event name"

[variables.start_date]
type = "string"
prompt = "Start date (ISO 8601, e.g. 2026-04-15T00:00:00.000Z)"

[variables.end_date]
type = "string"
prompt = "End date (ISO 8601, e.g. 2026-04-17T23:59:59.000Z)"

[variables.timezone]
type = "select"
prompt = "Timezone"
choices = [
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "America/Toronto",
  "Europe/London",
  "Europe/Berlin",
  "Asia/Tokyo",
  "UTC",
]
default = "America/New_York"

[variables.location]
type = "string"
prompt = "Location (city, state or country)"

[variables.online]
type = "bool"
prompt = "Online event?"
default = false

[variables.website]
type = "string"
prompt = "Event website URL"

[variables.free]
type = "bool"
prompt = "Free to attend?"
default = false

[variables.ticket_price]
type = "int"
prompt = "Ticket price (USD, whole dollars)"
when = "not free"

[variables.tags]
type = "multiselect"
prompt = "Tags"
choices = [
  "Web Development",
  "Tech",
  "Unconference",
  "Open Source",
  "Design",
  "DevOps",
  "AI/ML",
  "Security",
  "Data",
  "Leadership",
]

[variables.featured]
type = "bool"
prompt = "Feature this event?"
default = false

[variables.description]
type = "string"
prompt = "Short description (one or two sentences)"
```

A few things to note:

- `timezone` is a `select`. The user can only pick from the list. No invented timezone strings.
- `ticket_price` is conditional — only shown when `free` is false.
- `tags` is a `multiselect`. The user picks from a fixed list of choices. No freeform strings.

These constraints are not documentation. They are enforced at runtime.

## Write the template file

Create `templates/event/template/event.md.tera`:

```markdown
---
name: "{{ name }}"
startDate: {{ start_date }}
endDate: {{ end_date }}
timezone: "{{ timezone }}"
location: "{{ location }}"
online: {{ online }}
website: "{{ website }}"
{% if not free %}ticketPrice: {{ ticket_price }}
{% endif %}tags:
{% for tag in tags %}  - "{{ tag }}"
{% endfor %}featured: {{ featured }}
verified: false
---

{{ description }}
```

The `verified` field is always false on creation — a human reviewer sets it later. It doesn't need to be a variable.

## The human workflow

Someone needs to add Grok Conf. They run:

```bash
diecut new ./templates/event -o content/events/grok-2026.md
```

diecut reads the config and prompts for each variable:

```text
Event name: Grok Conf
Start date (ISO 8601, e.g. 2026-04-15T00:00:00.000Z): 2026-04-15T00:00:00.000Z
End date (ISO 8601, e.g. 2026-04-17T23:59:59.000Z): 2026-04-17T23:59:59.000Z
Timezone [America/New_York]:
  1. America/New_York
  2. America/Chicago
  3. America/Denver
  4. America/Los_Angeles
  5. America/Toronto
  6. Europe/London
  7. Europe/Berlin
  8. Asia/Tokyo
  9. UTC
Location (city, state or country): Greenville, SC
Online event? [no]: no
Event website URL: https://atlaslocal.com/grok-26
Free to attend? [no]: no
Ticket price (USD, whole dollars): 279
Tags:
  1. Web Development
  2. Tech
  3. Unconference
  4. Open Source
  5. Design
  6. DevOps
  7. AI/ML
  8. Security
  9. Data
  10. Leadership
Feature this event? [no]: no
Short description (one or two sentences): Grok Conf is an attendee-driven unconference for web folks.
```

The output at `content/events/grok-2026.md`:

```markdown
---
name: "Grok Conf"
startDate: 2026-04-15T00:00:00.000Z
endDate: 2026-04-17T23:59:59.000Z
timezone: "America/New_York"
location: "Greenville, SC"
online: false
website: "https://atlaslocal.com/grok-26"
ticketPrice: 279
tags:
  - "Web Development"
  - "Tech"
  - "Unconference"
featured: false
verified: false
---

Grok Conf is an attendee-driven unconference for web folks.
```

Only fields declared in `diecut.toml` appear. `ticketPrice` is present because the event is not free. The tags are from the approved list.

## The LLM workflow

An LLM can generate the same entry without interactive prompts. It reads `diecut.toml` to understand the schema, then calls diecut with `--defaults` and `-d` flags to pass each value:

```bash
diecut new ./templates/event --defaults \
  -d name="Grok Conf" \
  -d start_date="2026-04-15T00:00:00.000Z" \
  -d end_date="2026-04-17T23:59:59.000Z" \
  -d timezone="America/New_York" \
  -d location="Greenville, SC" \
  -d online=false \
  -d website="https://atlaslocal.com/grok-26" \
  -d free=false \
  -d ticket_price=279 \
  -d tags="Web Development,Tech,Unconference" \
  -d featured=false \
  -d description="Grok Conf is an attendee-driven unconference for web folks." \
  -o content/events/grok-2026.md
```

`--defaults` suppresses all prompts. Each `-d key=value` flag overrides one variable. The result is the same file as the human workflow produced.

The schema constrains what the LLM can do:

- It cannot pass a field that doesn't exist in the template. Unknown `-d` keys are ignored.
- It can only use tag values from the `choices` list. Passing `"JavaScript"` for `tags` would fail validation — that value is not in the list.
- It cannot set `ticket_price` when `free=true` — the conditional makes that field inactive.

The LLM reads the `diecut.toml` directly to know what fields exist and what values are valid. There is no separate documentation to maintain or drift from. The template is the spec.

## Preview before writing

Before committing the file, use `--dry-run --verbose` to see the rendered output without writing anything:

```bash
diecut new ./templates/event --defaults \
  -d name="Grok Conf" \
  -d free=false \
  -d ticket_price=279 \
  --dry-run --verbose \
  -o content/events/grok-2026.md
```

```text
[dry-run] would write: content/events/grok-2026.md
```

The rendered content is printed to the terminal. Useful for inspecting the output before it lands in your content directory.

## The core insight

`diecut.toml` is the contract. Whether a human is filling in the values through prompts or an LLM is passing them with `-d` flags, the output always conforms to the declared schema. Fields that don't exist in the config cannot appear in the output. Values for `select` and `multiselect` variables must come from the defined choices.

The schema travels with the template. Commit the `templates/event/` directory to your site repo and every contributor — human or automated — works from the same definition.

---

To learn more about variable types and conditional fields, see [Creating Templates](/creating-templates/). For all CLI flags, see the [Commands reference](/reference/commands/).
