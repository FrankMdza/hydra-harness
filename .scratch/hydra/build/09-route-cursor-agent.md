# 09 - Route `cursor-agent`

Type: build
Status: open
Blocked by: 03, 08

## Goal

The second provider: its command line, its hook dialect, `CURSOR_DATA_DIR`, the per
server `mcp enable`, the fatal pre-flight, and its rows in the normalizer.

## Where the design says it

- Harness spec 6.7's `cursor-agent` block, and why `--force` is safe under a gate.
- 6.5 for the data directory Hydra owns, the individual approvals, the two mechanics
  that decide how the launch is written, and why the pre-flight is fatal on this
  provider alone.
- 6.6 for `beforeReadFile` as a second event, `failClosed: true`, and why Hydra authors
  Cursor's own format rather than leaning on its conversion of Claude's.
- 6.2 for `CURSOR_AUTH_TOKEN` and `CURSOR_API_KEY`, which are write-through.
- Contract 7.5, 7.8 and 9.3: thinking text without a count, `usage` optional even on the
  success path, and `model` as a display name with the id in the chat store.

## Done when

- A recorded `cursor-agent` run writes a record that passes the contract checker.
- Reads are gated, because `preToolUse` alone leaves the file-read path open on this
  provider.
- The pre-flight fails the run when the listing does not report an injected server ready.
- A run that ends with no counts is `usage_source: unavailable`, never `estimated`.

## Watch for

**Cursor is the only provider whose MCP failure mode produces a wrong answer rather than
no answer.** An unapproved project entry does not shadow the human's same-named server,
it fails to replace it, so the listing says the name is not loaded while the human's
server starts with the human's token.

**`cursor-agent` refuses a symlinked hook config silently and runs the command anyway**,
while following a symlinked MCP config. That is a loader property and not a provider
property, so "real files, never links" is a hook rule that does not generalise.

**Every hook's message is concatenated into the reason text**, and one probe reached the
model quoting the hook that allowed. A `decider` parsed out of that is worse than no
field.

**Provider claims expire without a version moving.** Ticket 33 found this binary
reporting a different `model` because the account's model selection changed.

## What ticket 03 left you, added 2026-09-05

**The seam is `normalize::read_with(text, policies, layout)`, and it dispatches on
nothing today.** `src/normalize/mod.rs` holds the ten types and calls
`claude::normalize` directly, because one provider is routed. Your column needs a
`cursor.rs` beside it and a branch on the envelope's `provider`, which every line carries
(contract 4). The types themselves should not need to change: `ToolCall::call_id` is a
`String`, which is what lets you mint the pairing 7.6 says you have to mint, and
`ResultStatus::Unclassified` is already the answer for a result you cannot label.

**`usage` on your provider will exercise the branch `claude` never does.** `UsageSource`
has `Unavailable` wired and reached: a run with no `result` frame gets a `Usage` event
with `source: unavailable` and no counts rather than zeroes. 7.8 says your `catch` path
is exactly that shape, and the code for it is written.

**`permission_decision` is derived by `src/policy/replay.rs`, which is per provider in
one place.** It sends the gate `Dialect::Claude` and a `claude`-shaped `PreToolUse`
payload. Re-evaluating a `cursor-agent` call means passing your dialect and mapping your
frame onto the payload the gate reads, which is `tool_name`, `tool_input` and `cwd`.
Nothing else in the file is provider-specific.

**Do not try to attribute a refusal from your reason text.** The `claude` adapter tells
Hydra's gate from the user's guards by the `hydra/` prefix on the reason, and 7.7 already
records that your provider concatenates every hook's message, so that prefix can arrive
inside another hook's prose. On this provider the honest answer may be that `decider` is
underivable, which is a finding rather than a guess.
