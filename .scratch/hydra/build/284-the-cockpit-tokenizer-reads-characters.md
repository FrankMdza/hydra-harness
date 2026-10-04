# 284 - The cockpit's tokenizer reads characters, and its scroll clamps

Type: build
Status: resolved
Blocked by:

## What

From the Rust review of 2026-10-03 (build 275, run `01a1050b-c1bf`), `.scratch/hydra/research/275-rust-review.md`. Fixes that change behaviour, findings 8 and 10: `src/cockpit/call_head.rs:445` and `:498` iterate bytes as `char`, so an accented path is shown as mojibake and `à` (C3 A0) splits a word; iterate `char_indices()`. And the scroll limits at `app.rs:1331` and `:1400` cast to `u16`, wrapping past 65,535 rows, with `+` that panics in debug; use `u16::try_from(..).unwrap_or(u16::MAX)` and `saturating_add`, and clamp the wheel at `:1063`. A test each, failing before.

## Done when

- fmt, clippy over the workspace and `cargo test --workspace` clean; every behaviour-preserving
  change leaves every existing test passing unchanged. `IMPLEMENTATION.md`: one sentence.

## Outcome

Resolved 2026-10-04 by run `01a10540-0ea6` (`codex` gpt-6.1-sol, `standard`, 1.16M tokens),
applied without changes.

Built: `command_has_list` and `shell_segments` in `src/cockpit/call_head.rs` iterate characters,
so an accented path shows as written and `à` no longer splits a word; the dead `||` check is
gone. The scroll limits in `app.rs` convert with `u16::try_from(..).unwrap_or(u16::MAX)`, add
with `saturating_add`, and the detail's key and wheel movement clamp to the limit (the wheel
handler was at line 1542, not 1063). Two tests in `tests/cockpit.rs`; the agent restored the old
code and saw both fail.

Measured by the session: the whole workspace, 1,252 passed and 4 failed, all four in
`tests/cockpit.rs` and all about `hydra-puppets pending` (`exit_two_says_the_goal_moved_on`,
`exit_three_says_already_approved_and_hides_the_step`,
`inside_a_seatbelt_a_on_a_step_runs_nothing`,
`pending_runs_on_the_shown_goal_at_most_every_ten_seconds`); `tests/cockpit.rs` alone right
after, 137 of 137. They time a helper process and fail under the load of three live runs and a
full suite. Build 287 makes them hold under load.
