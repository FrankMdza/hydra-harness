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
