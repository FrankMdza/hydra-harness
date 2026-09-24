# 117 - `Agent` is `Task` renamed, on Claude Code 2.1.281

Type: build
Status: resolved
Blocked by:

## What

The `claude` table in `src/policy/table.rs` was read off the `system`/`init` frame of 2.1.263 on
2026-09-06 and has `tool("Task", Capability::SubagentSpawn)` at line 101. Claude Code 2.1.281
renamed that tool. The corpus on 2026-09-24 shows `Agent` in `tool_use` blocks on every run from
06:45 UTC on (01a0d229-cd8f, 01a0d23c-ccc9 with four, 01a0d3e4-4245 with two), with hook frames
named `PreToolUse:Agent`, and no `Task` call on any 2.1.281 record. `hydra doctor` prints
`claude 2.1.281 (Claude Code) measured 2.1.268`.

The gate is fail-closed on a name it does not know: on run 01a0d3e4-4245 the `hook_response`
for the `Agent` call was `permissionDecision` `deny` with `hydra/unknown-tool`. That is the
right answer for an unknown name and the wrong one for this tool, because `standard` has a
position on `SubagentSpawn` already, and every Opus run since the rename has been refused its
subagents without anyone deciding that.

## Do

1. `src/policy/table.rs`: add `tool("Agent", Capability::SubagentSpawn)` beside `Task`, keep
   `Task` so records from 2.1.263 to 2.1.280 still read back, and say in the comment which build
   renamed it and where it was measured (above). If the table has a per-provider count in a doc
   comment or a test (the "twenty-nine tools" comment near line 78, the list near line 455), make
   them agree.
2. If `src/provider/claude.rs` holds the version `hydra doctor` prints as "measured", move it to
   `2.1.281` and say why in its comment. If it holds no such constant, leave it.
3. Wherever the gate compiler or the replay treats `Task` by name rather than by capability
   (`grep -rn '"Task"' src/`), treat `Agent` the same way. Read each hit before deciding.
4. `IMPLEMENTATION.md`: `grep -n "SubagentSpawn\|Task" IMPLEMENTATION.md`, and in the passage on
   the `claude` table, one sentence on the rename. Nothing else in that file.

## Done when

- A test in `tests/gate.rs` compiles a `standard` gate for `claude` and feeds it a
  `PreToolUse` payload for `Agent`, and reads back the same decision and code the same payload
  gets for `Task`. It fails before the change with `hydra/unknown-tool`; check that it does and
  say so.
- The table's own unit tests cover the new row.
- `cargo test --test gate --test normalize` and `cargo test --lib` green; `cargo fmt --check`
  and `cargo clippy --all-targets -- -D warnings` clean.

What `standard` does with a subagent is not this ticket's question; the policy set already
answers it for `Task`, and this ticket only makes the answer reach the new name.

## Outcome

Built by `cursor-agent` on `composer-2.5` under `standard`, run 01a0d3ec-aba2-7352, 1.67M
tokens, applied to master by hand (`~/.claude/handoffs/2026-09-24-panel4/diff-117.patch`).
`tool("Agent", Capability::SubagentSpawn)` beside `Task`, thirty rows, both kept so records on
either side of the rename read back. `agent_gets_the_same_gate_answer_as_task_under_standard`
in `tests/gate.rs` failed before the change with `Agent` answered `hydra/unknown-tool` where
`Task` gets `hydra/capability`; both are refusals under `standard`, which is the point: the
rename cost the right reason, not safety. The replay test and the missing-`tool_input` test loop
over both names. Three of the ticket's premises were false and the agent said so: the doctor's
measured version lives in `src/provider/mod.rs` (`MEASURED_VERSIONS`), not `provider/claude.rs`;
the compiler reads the table and hardcodes no name; the replay treats `Task` specially only in
a unit test. The measured version was left at 2.1.268, since this ticket measured one tool, not
the frame.

The session changed one thing on the way in: the agent had dropped the table's
`CLAUDE.len() == measured.len()` equality, which is the check that the table names nothing the
provider never offered. It is back as `measured.len() + 1`, with `Agent` named as the one row
beyond 2.1.263's frame. The next rename breaks it again, on purpose.
