# 09 - Route `cursor-agent`

Type: build
Status: claimed
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

## What ticket 07 left you, added 2026-09-06

**Three provider facts are yours to fill in and they are already methods.**
`Provider::skill_root` is where Hydra links (`.cursor/skills` for you, and 6.4's table
says cwd or `--workspace` comes with it), `Provider::user_skill_roots` is which user-scope
roots the collision check compares against, and `Provider::resolved_skills` is how the
provider is asked what it resolved. `hydra::skills::index` already scans all seven roots
of 6.4, yours included, so only the filter is missing.

**Your resolved listing is the paid one.** Ticket 24 put it in
`~/.cursor/chats/<hash>/<session>/store.db`, largest blob, `content` field, after a real
run. Contract 5.3 makes the field optional on your provider for exactly that reason, and
`skills::record` already takes an `Option`, so returning nothing is a supported answer
rather than a gap. A run refused for a usage limit writes no `store.db` at all.

**Your listing has no name field**, only a path and a description, so the third collision
test needs a shape of its own. The first two tests compare names and work unchanged.

## What ticket 08 left you, added 2026-09-06

**The canonical document exists and your file is generated from it.** `config/mcp.md` is
protected config #5: Markdown prose around exactly one fenced `toml` block, one `[[server]]`
table per server, carrying `name`, `transport` (`stdio`, `http` or `sse`), `admit` as a list of
policy names, and then `command`/`args`/`env` or `url`/`headers`. `hydra::mcp::Document` parses
it and `hydra::mcp::Server` is the shape you generate from. Your half is two functions beside
`claude`'s: `claude::mcp_config` builds `{"mcpServers": {...}}` with `type` plus `url` on a
remote server, and yours writes `<worktree>/.cursor/mcp.json` with `url` alone. The stdio body
is identical, which is what one document buys.

**Your file does not go in the spawn directory, and `mcp::write_config` already knows that.**
It matches on the provider to pick a path, and `Layout::mcp_file` is the `claude` one. Yours
needs the worktree, so a `--shared-tree` run has nowhere to put one, which is the next note.

**Your `undeliverable` reason is the second one, and the shape is waiting for it.**
`hydra::mcp::plan` asks `claude::mcp_undeliverable`, which answers on the policy name. 6.5 gives
a `--shared-tree` Cursor run no injected config at all, so that is a launch that cannot deliver
whatever the document admits, and `Plan::shortfall` turns it into the same `mcp_unavailable`
refusal `judge` gets on `claude`. Today the function takes a policy name; a shared-tree run is a
fact about the launch rather than the policy, so it will want an argument.

**The fatal pre-flight is still yours and is not written.** 6.5 gives it to Cursor alone, so
nothing on the `claude` path checks that an injected server actually started. What `claude` does
instead is record the difference: an injected server that failed reaches stderr and shows up as
`injected` without `resolved`. Your listing is the one that lies, so yours refuses.

**`Provider::resolved_skills` is now `Provider::resolve`**, and it answers contract 5.3's two
`resolved` fields from one probe: `Resolved { skills, mcp, failed_mcp }`, each `Option` where the
provider did not say. It takes a `hydra::mcp::Probe`, which is either the run's own config file
or `Unanswerable` when the probe cannot reproduce the run's MCP shape. Your MCP answer is the
paid one, like your skill listing, so returning `None` for both is a supported answer.
