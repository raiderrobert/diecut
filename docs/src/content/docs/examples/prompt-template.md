---
title: Prompt and skill templates
description: "Write a high-quality prompt once. Personalize it without breaking it."
---

You've spent days on a system prompt. The right instructions. The right constraints. The right persona. The fallback behavior that keeps the assistant on topic when customers go off script. You've tested it.

Now you need five versions — one per client, one per product tier, one per language. The temptation is to copy and edit. The risk is that someone rewords a sentence that was carefully tested, removes a constraint that matters, or introduces inconsistency across variants that you won't catch until a client notices.

diecut separates the craft from the personalization. The core prompt stays fixed in a template. The context-specific parts — company name, product, tone, escalation contact — are variables. Anyone can instantiate a variant without touching the instructions that took days to get right.

## Set up the template

Create a directory for the prompt template, either in your project or in a shared prompts repository:

```text
prompts/
  support-bot/
    diecut.toml
    template/
      system-prompt.md.tera
```

## Write the config

Create `prompts/support-bot/diecut.toml`:

```toml
[template]
name = "support-bot"

[variables.company_name]
type = "string"
prompt = "Company name"

[variables.product_name]
type = "string"
prompt = "Product name"

[variables.assistant_name]
type = "string"
prompt = "Assistant name"
default = "Support"

[variables.tone]
type = "select"
prompt = "Tone"
choices = ["friendly", "professional", "technical"]
default = "professional"

[variables.escalation_contact]
type = "string"
prompt = "Escalation contact (email or channel)"
```

A few things to note:

- `tone` is a `select`. The user can only pick `friendly`, `professional`, or `technical`. No freeform input.
- `assistant_name` defaults to `"Support"`. Clients that want a branded name can override it; clients that don't can press Enter.

## Write the template file

Create `prompts/support-bot/template/system-prompt.md.tera`:

```markdown
You are {{ assistant_name }}, a support assistant for {{ company_name }}.

Your role is to help customers with questions about {{ product_name }}.

## Tone

{% if tone == "friendly" %}Be warm and conversational. Use plain language. Acknowledge frustration with empathy before solving.
{% elif tone == "professional" %}Be concise and precise. Use formal language. Prioritize accuracy over warmth.
{% else %}Assume the customer is technical. Use correct terminology. Skip basic explanations unless asked.
{% endif %}
## Scope

Only answer questions directly related to {{ product_name }}. If a customer asks about something outside your scope, acknowledge their question and redirect them to {{ escalation_contact }}.

Do not speculate about features, pricing, or roadmap unless the information is in the context provided.

## Escalation

If a customer is frustrated or the issue cannot be resolved through conversation, offer to connect them with a human agent via {{ escalation_contact }}.

## Response format

- Keep responses concise (2-4 sentences for simple questions, up to 3 short paragraphs for complex ones).
- Do not use bullet points unless listing steps or options.
- Never start a response with "Certainly!", "Of course!", "Absolutely!", or similar filler phrases.
```

The phrasing in the Scope and Escalation sections — the acknowledgment step, the redirect, the offer to escalate — stays exactly as written across every variant. Variables fill in the blanks without touching the instructions.

## Generate a variant

```bash
diecut new ./prompts/support-bot -o prompts/acme-support.md
```

diecut reads the config and prompts for each variable:

```text
Company name: Acme Corp
Product name: Acme Widget
Assistant name [Support]: Widget Assistant
Tone [professional]:
  1. friendly
  2. professional
  3. technical
Escalation contact (email or channel): support@acme.com
```

The output at `prompts/acme-support.md`:

```markdown
You are Widget Assistant, a support assistant for Acme Corp.

Your role is to help customers with questions about Acme Widget.

## Tone

Be concise and precise. Use formal language. Prioritize accuracy over warmth.

## Scope

Only answer questions directly related to Acme Widget. If a customer asks about something outside your scope, acknowledge their question and redirect them to support@acme.com.

Do not speculate about features, pricing, or roadmap unless the information is in the context provided.

## Escalation

If a customer is frustrated or the issue cannot be resolved through conversation, offer to connect them with a human agent via support@acme.com.

## Response format

- Keep responses concise (2-4 sentences for simple questions, up to 3 short paragraphs for complex ones).
- Do not use bullet points unless listing steps or options.
- Never start a response with "Certainly!", "Of course!", "Absolutely!", or similar filler phrases.
```

The Tera conditional in the `## Tone` section resolves to one of three instruction sets based on the value the user selected. Everything else in the Response format and Escalation sections is unchanged.

## Non-interactive variant

For scripting or CI pipelines where prompts aren't practical, pass all values on the command line:

```bash
diecut new ./prompts/support-bot --defaults \
  -d company_name="Acme Corp" \
  -d product_name="Acme Widget" \
  -d assistant_name="Widget Assistant" \
  -d tone=friendly \
  -d escalation_contact="support@acme.com" \
  -o prompts/acme-support.md
```

`--defaults` suppresses all prompts. Each `-d key=value` flag sets one variable. The result is the same file as the interactive workflow produces.

This is also how you batch-generate prompts for multiple clients from a script — loop over a list of client configs and call diecut once per client.

## What the `select` type buys you

`tone` is a `select` because the prompt was designed and tested for exactly three values. The friendly instructions were written to go with the escalation phrasing. The professional instructions were tested against the scope constraints. The technical instructions assume a user population that doesn't need the basic explanations that the escalation section is written around.

Freeform input would let someone pass `tone=empathetic` or `tone=casual` and get a prompt that was never optimized for that value. The choices list is not documentation. It is the contract. Only values on the list are valid.

## The core insight

In most diecut templates, the template is scaffolding — the value is in the files it produces. Here, the value is in the template itself.

The expert work is the system prompt — the instructions, the constraints, the fallback behavior. That work lives in `system-prompt.md.tera`. It does not change when a new client is onboarded. The person instantiating a variant fills in company name, product name, and tone. They cannot reword the escalation logic. They cannot remove the response format constraints. They pick from the approved list of tones.

Push the `support-bot/` directory to a shared repo and every team member generates from the same base. A client-specific variant is a generated file, not a fork of the prompt.

## Versioning and updates

When the prompt improves — tighter scope instructions, better fallback behavior, a new constraint added after testing — update the template once. Any client that needs the improvement gets it by re-running diecut:

```bash
diecut new gh:yourteam/prompts/support-bot \
  -d company_name="Acme Corp" \
  -d product_name="Acme Widget" \
  -d assistant_name="Widget Assistant" \
  -d tone=friendly \
  -d escalation_contact="support@acme.com" \
  --defaults \
  -o prompts/acme-support.md
```

Clients that don't need the update stay on the version they have. Clients that do get a clean regeneration from the updated template — not a diff to apply by hand.

---

To learn more about variable types, computed variables, and conditional fields, see [Creating Templates](/creating-templates/). For all CLI flags, see the [Commands reference](/reference/commands/).
