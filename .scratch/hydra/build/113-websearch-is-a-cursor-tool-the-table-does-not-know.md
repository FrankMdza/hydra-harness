# 113 - `webSearchToolCall`: a Cursor tool the table does not know

Type: build
Status: resolved
Blocked by:

## What

The corpus walk `no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent` in
`tests/normalize.rs` went red on 2026-09-24 on master `89485ff`. The record it names is a
`cursor-agent` run on `composer-2.5` under `standard`, launched at 08:37 UTC that day on
cursor-agent `2026.09.23-86fc751` (`hydra doctor` says the last measured build was
`2026.09.18-9a7762b`). The panic:

```
[GateLeak { call_id: "tool_22b4e38e-...", tool: "webSearchToolCall",
  reason: "hydra/unknown-tool: webSearchToolCall is not in Hydra's capability table for this
  provider. The table is stale or the provider renamed a tool." }]
```

The stream, in order, three frames with the same `toolCallId`:

1. `{"type":"interaction_query","subtype":"request","query_type":"webSearchRequestQuery",
   "query":{"id":0,"webSearchRequestQuery":{"args":{"searchTerm":"...","toolCallId":"tool_..."}}}}`
2. `{"type":"tool_call","subtype":"started","call_id":"tool_...","tool_call":{"webSearchToolCall":
   {"args":{"searchTerm":"...","toolCallId":"tool_..."}},"hookAdditionalContexts":[],
   "toolCallId":"tool_...","startedAtMs":"..."}}`
3. `{"type":"tool_call","subtype":"completed","call_id":"tool_...","tool_call":{"webSearchToolCall":
   {"args":{...},"result":{"success":{"references":[{"title":"Web search results","url":"",
   "chunk":"Links:\n1. [...](https://...)\n..."}]}}}}}`

No hook frame carries that call id. Every other gated call in the same record has a gate answer
in the stream (186 `hydra/worktrees` refusals, four `hydra/no-path`, one floor), so this is
map ticket 51's class: a tool Cursor runs without asking `preToolUse`, like `ReadLints`,
`getMcpToolsToolCall` and `awaitToolCall` in `src/policy/table.rs`. The table has no row, the
normalizer has no name mapping, so the replay files it as a leak and the walk fails.

This is provider drift doing what map ticket 43 said it would: the table is the alarm.

## Do

1. `src/normalize/cursor.rs`, the raw-name match near line 119: map `webSearchToolCall` the way
   `webFetchToolCall` maps to `Fetch`. Name the hook-side tool `WebSearch`, which is what
   `claude`'s table calls the same capability (`tool("WebSearch", Capability::NetworkRead)`), and
   carry the stream's own args, as the other unhooked rows do.
2. `src/policy/table.rs`, the `cursor-agent` table: `unhooked("WebSearch")` beside the three map
   51 rows, with a comment that says when and on which build it was measured (above). The
   compiler must keep skipping it, so the compiled gate config never names it.
3. If `src/provider/cursor.rs` holds the version `hydra doctor` prints as "measured", move it to
   `2026.09.23-86fc751` and say why in its comment. If it holds no such constant, leave it.
4. `IMPLEMENTATION.md`: `grep -n "getMcpToolsToolCall"` and add one sentence to the paragraph on
   the unhooked Cursor rows naming the fourth and the day it arrived. Nothing else in that file.

## Done when

- A test in `tests/normalize.rs` feeds a `cursor-agent` stream with the three frames above (a
  `started` and a `completed` `webSearchToolCall`, no hook frame) and reads back no
  `Finding::GateLeak`, with the call's `derived_code` being `unhooked`. It fails before the change
  with the `hydra/unknown-tool` reason. Check that it does and say so.
- `the_cursor_gate_config_carries_no_unhooked_tool` in `tests/gate.rs` and
  `unhooked_cursor_rows_are_known_and_carry_no_path` in `src/policy/table.rs` cover four names.
- The `as_hook_call` unit test in `src/normalize/cursor.rs` covers the new mapping.
- `cargo test --test normalize --test gate` and `cargo test --lib` green, `cargo fmt --check`
  and `cargo clippy --all-targets -- -D warnings` clean.

## Outcome

Built by `cursor-agent` on `composer-2.5` under `standard`, run 01a0d3e2-1098, 1.34M tokens, 14
minutes, applied to master by hand from the worktree diff (`~/.claude/handoffs/2026-09-24-panel4/diff-113.patch`).
Five files: the `webSearchToolCall` to `WebSearch` mapping and its unit test in
`src/normalize/cursor.rs`; `unhooked("WebSearch")` and a fourth name in the table's own test in
`src/policy/table.rs`; `a_cursor_websearch_call_is_the_providers_decision` in
`tests/normalize.rs`, which failed before the change with `left: "webSearchToolCall", right:
"WebSearch"`; a fourth name in `the_cursor_gate_config_carries_no_unhooked_tool` in
`tests/gate.rs`; one sentence in `IMPLEMENTATION.md`'s unhooked-rows paragraph.
`src/provider/cursor.rs` holds no measured-build constant, so nothing moved there. On master:
lib 209, gate 97, normalize 88 of 89 pass, fmt and clippy clean.

The agent reported three `run::tests` failures as pre-existing. On master they pass; inside the
run's worktree `git init` leaves `git_common_dir` at `None`, which is the worktree, not the code.

The one test still red is the corpus walk itself, and no longer on this record: with the Cursor
row in place it walks on to run 01a0d3e4-4245, a live `claude` record, and files two leaks the
gate did not commit. Build tickets 116 and 117 hold those, and map ticket 78 the scope question
under them.
