---
title: Structured content with consistent schemas
description: "Define the shape once. Every entry conforms — whether filled in by hand or by an LLM."
---

You maintain a conference listing site. A contributor submits this PR:

```markdown
---
name: "PyData NYC"
startDate: 2026-09-18
endDate: September 20, 2026
timezone: US/Eastern
city: New York
ticketCost: 350
tags:
  - Python
  - Data Science
---

Three days of talks and workshops.
```

Four things are wrong: `endDate` is in a different format than `startDate`. `timezone` is `US/Eastern`, not a valid IANA zone. `city` is an invented field — your schema uses `location`. `ticketCost` is `ticket_price` misspelled. Your query that filters by `ticket_price` silently excludes this event. The timezone string fails your date library. You find out when the site renders.

The fix is not a style guide in a README. It's a template that enforces the schema at the point of entry.

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

These constraints are not documentation. They are enforced at generation time. A contributor who types a timezone not in the list sees:

```text
Timezone [America/New_York]: US/Eastern
Error: "US/Eastern" is not a valid choice. Select a value from the list.
```

The file is not written. The invalid value never reaches the content directory.

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

An LLM can generate the same entry without interactive prompts.

Without a schema, an LLM asked to "add Grok Conf as a conference entry" might produce:

```markdown
---
title: "Grok Conf"
date: "April 15–17, 2026"
location: "Greenville, South Carolina"
price: "$279"
categories:
  - web
  - javascript
  - unconference
isFeatured: false
---
```

`title` instead of `name`. `date` instead of `startDate`/`endDate`. `price` as a string with a dollar sign instead of `ticket_price` as an integer. `categories` instead of `tags`, with freeform strings instead of approved choices. Your content layer ignores the entry silently.

With diecut, the LLM reads `diecut.toml` to understand the schema, then calls:

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

The LLM reads the `diecut.toml` directly to know what fields exist and what values are valid.

Compare this to the alternative: a `CONTRIBUTING.md` documenting the required fields and valid tag values. Three months later someone adds `Rust` to the approved tags — in `diecut.toml` and in the multiselect choices, but not in the README. The LLM reads the README and generates entries without `Rust`, or reads a cached version and generates `Rust` when the template no longer accepts it.

The `diecut.toml` the LLM reads is the same file that enforces constraints at generation time. They cannot drift from each other because they are the same artifact.

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

## The difference

The PyData example at the start of this article — wrong field names, invented timezone strings, mismatched date formats — came from a contributor who didn't know the schema. A style guide in a README can go stale; `diecut.toml` is the schema, so it can't drift from itself.

For LLM workflows this matters directly: the LLM reads `diecut.toml` to understand what fields exist and what values are valid. That is the same file diecut uses to validate at generation time. There is no separate source of truth — no README, no wiki page — that can fall out of sync with what the tool accepts.

Validation happens at generation time, before anything lands in the content directory. `US/Eastern` for the `timezone` field is rejected at the prompt, not discovered later when the date library fails at render time.

---

To learn more about variable types and conditional fields, see [Creating Templates](/creating-templates/). For all CLI flags, see the [Commands reference](/reference/commands/).
