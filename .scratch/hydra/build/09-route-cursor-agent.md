# 09 - Route `cursor-agent`

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-06 on `cursor-agent` 2026.09.02-c22c1a3, `claude` 2.1.263, `agy` 1.1.26,
all three re-read at the top of the session and unchanged since the map closed.
`IMPLEMENTATION.md` has a section for this provider with every number below and the
reasoning; this is the short version.

**Built.** `src/provider/cursor.rs`: 6.7's argv, the hook config in Cursor's own format
with `preToolUse` and `beforeReadFile` both `failClosed: true`, `.cursor/mcp.json` with
`url` alone on a remote server, `CURSOR_DATA_DIR` set per run to
`~/hydra/spawn/<run-id>/cursor-data/` and carried to the detached half by value, the
free pre-flight of 6.1 item 7 as `mcp enable` per injected server, `mcp list`, `mcp
disable` for everything else under the redirect, `mcp list` again, refusing the run with
`mcp_unavailable` when an injected server is not `ready`. `src/policy/table.rs`: a
second capability table, five names measured off the hook and thirteen read off the
bundle, `MCP:` as the MCP prefix. `src/policy/gate.py`: a `cursor` dialect that answers
in Cursor's own shape, maps a `beforeReadFile` payload onto `Read`, and refuses to be
asked in a dialect other than the one it was compiled for. `src/normalize/cursor.rs`:
the second column of contract 7.2, dispatched on the envelope's provider. `outcome.rs`:
no terminator is `failed` on this provider, two stderr sentences derive `rate_limited`
and `auth_failed`, and a signal death now reaches `run_end.error` on both providers.
Thirty new tests across `gate.rs`, `spawn.rs`, `mcp.rs`, `normalize.rs` and
`outcome.rs`, and twenty-six unit tests under `src/`, none of which launches the
provider.

**Measured.** Two paid probes outside Hydra to learn the hook's spellings and what a
`beforeReadFile` deny looks like on the stream, then two recorded `hydra run`s on
`--model composer-2.5`: `01a07891-a339-7173-96d9-70beb88e5efb` under `standard`, three
tools allowed, `completed`, `usage_source: provider`, 11.9 s; and
`01a07892-459d-7420-904e-91d6a6986159` under `read-only`, an out-of-scope read refused
with `hydra/out-of-scope`, an in-scope read allowed, a write refused with
`hydra/capability`, `completed` with two `hydra_policy` denies and no finding. Both pass
the contract checker and the corpus walks. The pre-flight listing was measured by hand
against a working and a broken server under a scratch data directory, and `hydra reap`
took the first run's spawn directory, socket file and all.

**Done when, item by item.** A recorded run writes a record that passes the checker,
met twice. Reads are gated, met, and the mechanism is measured on both events. The
pre-flight fails the run when the listing does not report an injected server ready:
built, driven by `tests/mcp.rs` from the measured listing shape, and measured by hand
with the free commands, but not through `hydra run`, because `config/mcp.md` admits no
server and is a protected config. A run that ends with no counts is `unavailable`,
never `estimated`: met by construction and by `tests/normalize.rs`, not observed on a
real run, because producing one means spending a failure.

**What the ticket assumed that turned out false.**

- "`preToolUse` alone leaves the file-read path open on this provider." On this build
  `preToolUse` fires for `Read` with `tool_name` and `file_path` before the read runs,
  so the shipped gate refuses an out-of-scope read there and `beforeReadFile` never
  sees it. Both are registered anyway; the second was measured refusing on its own with
  a hook that allowed at the first.
- 6.6's "the script answers in two shapes rather than three." `beforeReadFile` takes
  only Cursor's own `permission` shape, so the `cursor` dialect answers in that shape
  on both events. Three shapes, one per provider.
- Ticket 08's "your MCP answer is the paid one." It is not: the pre-flight listing is
  free and is what `mcp.resolved` carries. The skill listing is paid and stays null.
- "A shared-tree run is a fact about the launch rather than the policy, so it will want
  an argument." It got one: `mcp::plan` takes `Checkout`, and `--shared-tree` is still
  not built, so `Checkout::None` is a run outside a repo today, which is refused before
  a run id exists.
- 6.5's account of what `CURSOR_DATA_DIR` moves is incomplete: the redirect also moves
  the per-project transcript, `mcp-auth.json`, `repo.json` and a worker socket, all of
  which die with the spawn directory. `chats/` stayed put, as 6.5 says.
- Two spellings nobody wrote down: the listing tool arrives at the hook as `Grep`, and a
  `Write` fires a `Read` hook for the same id first.

**No decision found.** Two things are worth the parent's eye without being the edge of
the design. The `decider` question the ticket flagged resolved without a guess: the
decider is derived from the policy as on `claude`, and the reason text is read only for
the `hydra/<code>:` prefix the gate writes, which 7.7 allows. And map ticket 39's
possible `Skill` split lands on `Read` here rather than on a row of this table, since
Cursor has no `Skill` tool and a linked skill is read through the file tool from a path
under `~/hydra/skills-snapshots/`, which is the same out-of-worktree link ticket 39 is
already about.

## Added by map ticket 46, 2026-09-06

On the next paid Cursor run, check whether a `beforeSubmitPrompt` hook in the user's config adds
context, and whether `hookAdditionalContexts` on a tool call ever carries anything. Ticket 46 found
`claude`'s `UserPromptSubmit` adding the user's standing reminder to every recorded run and decided
it is data and stays in `raw`. Nothing on this machine defines a Cursor hook today, so the claim in
harness spec 6.6 is stated as unmeasured.
