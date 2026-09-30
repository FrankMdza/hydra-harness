# 177 - The dragon counts this session's seats

Type: build
Status: resolved
Blocked by: 176

## What

The human's cockpit redesign asks for one dragon head per agent of the session he is watching.
`src/cockpit/mascot.rs` wakes one head per live run in the whole corpus (`pose_for(live)`), and
build 152's Outcome said why: `Card` had no session field. Since build 169, `live::RunView` carries
`parent_run_id` from line 0, so the cockpit can tell a session's seats from everything else.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. `Card` gains `parent_run_id`, filled from `RunView`.
2. With `--session`, the heads count live runs whose `parent_run_id` is the session's run id,
   plus the session itself; `+N` and the `N live seat(s)` label count the same set. Without
   `--session`, nothing changes.
3. When one of the session's seats ends not ok while the cockpit watches, the flash the dragon
   already makes (build 152: `gated` poses refused, other not-ok poses meme) is kept, and the
   header line says which seat it was for one refresh: `<short id> <outcome>`.
4. `IMPLEMENTATION.md`, the mascot paragraph.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f30e-4656 under `standard`. The session
applied it as written. `Card` carries `parent_run_id`; with `--session`, the dragon's heads, `+N`
and the seat label count the session and its direct seats, and a seat that ends not ok is named
`<short id> <outcome>` under the count for one refresh. Until the chat prints its run id the
cockpit counts every live run, as without `--session`. The fire pose still ignores the session.
Five tests, four failing first against stubs; fmt and clippy clean, `--test cockpit` 44, `--lib`
221 on master.

