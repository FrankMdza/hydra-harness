# 33 - What the Cursor usage estimator keys on, when `model` says "Auto"

Ticket: [33 - What the Cursor usage estimator keys on, when `model` says "Auto"](../issues/33-cursor-estimator-model-family.md)
Type: research. This file reports measurements. It proposes mechanisms but decides nothing.

Measured 2026-09-04 on this machine, 15:50 to 16:05 local. Four `cursor-agent` runs, two of which
returned counts and two of which were refused by the subscription before any model ran.

## Versions under test

```
cursor-agent  2026.09.02-c22c1a3
```

Read at 15:45 and again after the last run. Same build ticket 25 measured, which matters: the
finding below that contradicts ticket 25 is **not** a version drift. It is a config change on this
machine, and that distinction is the whole point of the first section.

## Probe hygiene

Runs and scratch files live in
`/private/tmp/claude-501/-Users-USER/63edcca5-a9f3-4f9c-ba02-4bf0272e02c9/scratchpad/t33/`. No
user-scope config was written. The four runs created four chat directories under `~/.cursor/chats/`,
which is Cursor writing its own state, not Hydra writing outside a path it created.

A venv was created to install `tiktoken` for the tokenizer comparison. Pip cannot bootstrap in this
sandbox, so that measurement was done a different way, described in its own section.

## What the runs cost

Four runs, all on the agent-CLI plane, no billed key touched.

| run | model asked for | counts returned |
|---|---|---|
| P1 | default | 7,214 in + 8,896 cached, 33 out |
| P2 | `--model auto` | 16,606 in + 512 cached, 30 out |
| P3 | `--model claude-sonnet-5-thinking-high` | none, refused |
| P4 | `--model gpt-5.2` | none, refused |

## The short version

**Ticket 09's design works, and not from the stream.** The resolved model id is recoverable, but it
lives in the vendor's chat store rather than in any frame `cursor-agent` prints. So the estimator can
pick a tokenizer by family after all, at the cost of reading a second source at harvest time.

Four findings, in the order they change things:

1. `model` on `system/init` is the **display name of the selection**, not the model that served the
   request. Under Auto it says `"Auto"` and the served model never appears in the stream.
2. **The resolved id is in the chat store**, at `providerOptions.cursor.modelName`, per assistant
   message, in the directory named by the run's `session_id`. The Auto run resolved to
   `cursor-grok-4.5-high`.
3. Ticket 25 measured `model: "Auto"` as the default. **On the same build it now reports
   `"Composer 2.5"`**, because the account's selected model changed. The field tracks config, so any
   claim about what it says has a shelf life measured in whatever the user last clicked.
4. An estimator built on transcript text has **no ground truth to calibrate against**. The text the
   store keeps under-explains the provider's own input count by roughly half.

## `model` is the selection's display name

P1, default flags, no `--model`:

```json
{"type":"system","subtype":"init","apiKeySource":"login","cwd":"…/t33",
 "session_id":"774333cd-0bd9-4a61-b96c-f9493b9f2d6f","model":"Composer 2.5","permissionMode":"default"}
```

Ticket 25 recorded `"Auto"` here on this same build. Nothing about `cursor-agent` changed. What
changed is the account's model selection, which `--list-models` reports as
`composer-2.5 - Composer 2.5 (current)` while still listing `auto - Auto (default)`. So `model`
echoes the selection, and ticket 25's reading and this one are both correct about different config.

Pinning a model changes it, and the display names are their own problem:

| `--model` argument | `--list-models` label | what `model` reported |
|---|---|---|
| `auto` | `Auto (default)` | `Auto` |
| `claude-sonnet-5-thinking-high` | `Claude Sonnet 5 1M Thinking` | `Claude Sonnet 5 300K High` |
| `gpt-5.2` | `GPT-5.2` | `GPT-5.2 Medium` |

**The three names for one model do not agree.** The id, the roster label and the reported display
name are three different strings, and the reported name is the only one in the record. So mapping
what the stream says back to a model id needs a lookup table that Cursor does not publish and that
this table shows would be wrong in both directions. Deriving a family from the leading word
(`Claude`, `GPT`, `Composer`, `Gemini`, `Grok`) does work on everything seen here, and it is the one
thing the display name is good for. It does not survive `Auto`, which names no vendor at all.

`--list-models` is free, needs no run, and marks the current selection. It is a reasonable thing for
`hydra doctor` to call. It does not answer the per-run question.

## The resolved id is in the chat store

A run's `session_id` is the directory name under `~/.cursor/chats/<workspace-hash>/`, so the store
for a run is findable without a search. Inside, each assistant message carries the model that
produced it:

```json
…,"providerOptions":{"cursor":{"modelName":"composer-2.5"}}},{"type":"text","text":"OK"…
```

P2 is the one that matters. It ran `--model auto`, the stream said `"Auto"`, and the store said:

```json
"providerOptions":{"cursor":{"modelName":"cursor-grok-4.5-high"}}
```

**Auto routed to Grok 4.5 High and told the stream nothing.** Two further things fall out of that
one line. `cursor-grok-4.5-high` is not in `--list-models`, whose roster only goes up to
`cursor-grok-4.6-*`, so Auto can pick models the user cannot pick and whose names Hydra cannot
enumerate ahead of time. And because `modelName` sits on each assistant message rather than on the
session, a run where Auto switches models mid-flight records both, which no single per-run `model`
field could represent.

The field is not an artefact of these probes. It appeared 90 times across the 33 chat stores on this
machine during ticket 34's scan.

The cost of using it is real and belongs in the decision. Reading it means harvesting a second
source, in a format Hydra does not own, and ticket 34 found that source comes in two shapes: plain
JSON in some chats, protobuf with a `blobEncryptionKey` in others, with ordering carried by a root
blob's hash list. These four probe stores are the JSON shape.

## Rate-limited runs write no counts, and this is now reproducible

P3 and P4 were refused before a model ran:

```
ActionRequiredError: You've hit your usage limit You've saved $49 on API model usage this month
with Pro. Switch to a different model or set a Spend Limit to continue with Sonnet. Your usage
limits will reset when your monthly cycle ends on 9/29/2026.
```

Both exited 1. Both wrote `system/init` and the `user` frame and then stopped. **No `result` frame,
so no usage at all.** This is the `catch`-to-stderr path ticket 25 read in the emitter and never
managed to trigger, and it is now trivially reproducible on this account: pin any premium model
before 9/29 and it happens every time.

Two consequences worth carrying. `model` is present on these runs even though usage is not, so the
model and the counts fail independently and a record cannot assume that having one means having the
other. And the whole premium roster is unavailable on this subscription until the cycle resets,
which means in practice every Cursor run Hydra records between now and 9/29 is Composer or whatever
Auto picks. Ticket 20 already recorded the `agy` version of this: a limited subscription aborts the
way an expired login does.

## The estimator has nothing to calibrate against

The plan was to tokenize a run's input with a generic tokenizer and compare against the provider's
count. Pip cannot bootstrap in this sandbox, so `tiktoken` was unavailable. What follows is the
measurement that was possible without it, and it turns out to answer a more basic question first.

Taking every text, result and reasoning string out of the two runs' stores and comparing the
character count against the provider's own input total:

| run | resolved model | recovered text | provider input | chars per token |
|---|---|---|---|---|
| P1 | `composer-2.5` | 34,053 chars | 16,110 | 2.11 |
| P2 | `cursor-grok-4.5-high` | 33,047 chars | 17,118 | 1.93 |

English prose runs about 4 characters per token and structured text about 3. **Both runs sit near 2,
so the provider is counting roughly twice what the transcript can account for.** The store is not
hiding the text either: the non-JSON blobs in each store total under 2.7 KB against 35 KB of JSON,
so there is no bulk of unread content to explain the gap.

The likeliest explanation is tool schemas, which are thousands of tokens on every request and which
no chat transcript records. I did not confirm that and it does not change the consequence:

**Whatever Cursor counts, it is not the text the transcript keeps.** An estimator that tokenizes the
recoverable text would be low by around half, and no amount of picking the right tokenizer closes a
gap that size. Worse, this cuts off the obvious way to check an estimator's accuracy, because the
only ground truth available is a number computed over an input Hydra cannot see.

The 2.11 against 1.93 spread between the two models is not evidence that family matters. It is
smaller than the error in what I am not counting.

## What this suggests for the contract

Findings, not decisions. Section 7.8 and section 9 are ticket 16's to write.

- **`usage_source: estimated` looks unearned on this provider.** Ticket 09 rejected `unavailable`
  for Cursor on a premise that has since expired twice, and the third measurement does not rescue
  it. An estimate that is structurally low by half is the fake measurement 7.8's "never a bare null"
  rule exists to prevent. `unavailable` says the true thing.
- **If `estimated` survives anyway**, `usage_estimator` should record the resolved `modelName` from
  the store rather than anything from the stream, and record that the estimate covers recoverable
  text only. The honest version of the field names its own blind spot.
- **A `model_resolved` field is available and cheap** if the record wants it, sourced from the store
  at harvest and populated per assistant message rather than per run. It is the only way a record
  can say what actually served an Auto run. Whether that is worth a second source at harvest is a
  contract decision, not a research one.
- **`model` in the record should be labelled as the selection**, not the model. Under Auto it names
  no model at all, and ticket 25's and this ticket's readings differ only because someone clicked
  something.
- Ticket 11 kept `model` off the delegate face. Nothing here argues with that.

## Unmeasured

- **The actual tokenizer error**, for want of `tiktoken`. The character-ratio result makes it a
  second-order question: the input is wrong before the tokenizer choice matters.
- **Whether Auto switches model mid-run.** `modelName` is per assistant message, so the record shape
  allows it, and both probes were single-turn. A longer run would settle it.
- **The premium roster**, until 9/29. Every pinned-model claim above rests on two refusals, and
  what `model` reports on a premium run that actually completes is unknown.
- **Whether the protobuf stores carry `providerOptions` too.** All four probes wrote JSON stores.
  If the newer format drops the field, the recovery path has a hole in exactly the chats that are
  newest.
