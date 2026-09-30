# 187 - The cockpit passes the new guards, and keeps every note

Type: build
Status: claimed
Blocked by: 185

## What

After build 185, `puppets.py review` takes `--no-open`, and `pending`'s argv carries
`--expect-run` or `--expect-plan` beside `--expect-revision`. The cockpit (builds 182 and 183)
must pass and accept them. And a codex review of build 186 found, from the code: when two stops
report in the same `poll_stops`, each answer replaces the single `drafted` note
(`src/cockpit/app.rs` near 875), so only the last note reaches the chat.

## Do

Tests first in `tests/cockpit.rs` and `tests/stop.rs` with the stubs they already use, each shown
failing before the change.

1. The review spawn passes `--no-open`; once the review ended, the run's detail shows the
   `hunk show ...` line the helper printed, from its log.
2. `parse_steps` accepts an argv ending in `--expect-revision N` preceded optionally by
   `--expect-run <id>` or `--expect-plan <path>`, and still refuses one without
   `--expect-revision` of the item's own revision. Exit 3 from `approve` reads "already approved".
3. Notes queue: every stop that reports pastes its note, in order, each ending without a newline;
   a test with two stops reporting in one poll.
4. `IMPLEMENTATION.md`.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit`, `--test
  stop` pass.
