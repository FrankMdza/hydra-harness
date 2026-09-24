# 116 - A hook refusal on a live `claude` record reads as a leak

Type: build
Status: claimed
Blocked by:

## What

The corpus walk `no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent` went red
on 2026-09-24 on a `claude` record that was still being written, run 01a0d3e4-4245, Opus on
Claude Code 2.1.281 under `standard`. Two findings, both on calls the gate refused:

```
GateLeak { call_id: "toolu_01K2...", tool: "Read", reason: "hydra/out-of-scope: policy standard
  allows file_read inside worktree or skill_snapshot and /users/user/.claude/projects/.../tool-results/blt3rf93c.txt is outside it." }
GateLeak { call_id: "toolu_01Lc...", tool: "Agent", reason: "hydra/unknown-tool: Agent is not
  in Hydra's capability table for this provider. The table is stale or the provider renamed a tool." }
```

The record shows the gate answering both. For each call, in order: the `tool_use` block; a
`system`/`hook_started` frame with `hook_name` `PreToolUse:Read`; a `system`/`hook_response`
frame whose `output` is Hydra's own JSON, `permissionDecision` `deny` with the reason above; and
a `user` frame whose `tool_result` has `is_error: true` and this content:

```
PreToolUse:Read hook error: hydra/out-of-scope: policy standard allows file_read inside ...
```

That prefix, `PreToolUse:<Tool> hook error: `, is new. Records on Claude Code 2.1.268 and
earlier carry the reason bare, starting at `hydra/`. The corpus shows the prefixed form on every
refusal from 2.1.280 on (first seen 2026-09-23 18:25 UTC, run 01a0cf84-4a6d) and no bare one
after it.

Why it passed until today. `src/normalize/claude.rs`, the function near line 855 that decides
whether the record refused a call, tries four sources in order. The third reads the tool result:
`ResultStatus::Error` and `reason_code(output)` not `allow`. `reason_code` in
`src/policy/replay.rs:76` strips the prefix `hydra/` and returns `None` otherwise, so on the
prefixed text it returns `None` and this source no longer fires. The fourth source,
`denied_in_result`, reads `permission_denials` off the final `result` frame, which is what has
been attributing every refusal since 2.1.280, with the weakest attribution the function has
("8.5's list") and a comment that calls it the last resort. A live record has no `result` frame
yet, so on it all four sources miss, `refusal` is `None`, and the branch near line 679 files a
`GateLeak` for a call the gate refused and the record shows refused.

So two things drifted: the attribution silently fell from `HydraPolicy` to the result list on
every completed record for a day, and a live record now reads as a leak.

## Do

1. In `src/normalize/claude.rs`, where the tool result's text is read for a reason (the third
   source), strip a leading `PreToolUse:<Tool> hook error: ` before `reason_code`, only when
   `<Tool>` is the call's own tool name. Put the stripping in one named function with a doc
   comment that says which Claude Code build introduced the prefix and cites this ticket. Do not
   change `reason_code`; other providers' reasons still start at `hydra/`.
2. Check whether `provider_denied` (line 263) or any other reader of that text needs the same
   stripping, and say so in the report either way.
3. `IMPLEMENTATION.md`: `grep -n "is_error: true" IMPLEMENTATION.md` and, in the passage on how a
   `claude` refusal is attributed, one sentence on the prefix and the build it arrived on.
   Nothing else in that file.

## Done when

- A test in `tests/normalize.rs` builds a `claude` record with a `Read` outside the worktree, the
  two hook frames and a `tool_result` in the prefixed form, **without** a `result` frame (see
  `a_call_the_record_stops_after_is_unknown_rather_than_a_leak` for an unclosed record), and reads
  back a `PermissionDecision` with `decider` `HydraPolicy` and no `GateLeak`. It fails before the
  change with the `GateLeak`; check that it does and say so.
- A second test, or a second arm of the same one, closes the record with a `result` frame
  carrying `permission_denials` and checks the decider is still `HydraPolicy`, not `External`.
- The existing tests that feed the bare form keep passing: records from 2.1.268 and earlier are
  in the corpus and stay readable.
- `cargo test --test normalize` and `cargo test --lib` green; `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` clean.

The `Agent` half of the panic is build ticket 117, not this one.
