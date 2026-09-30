# 150 - `hydra cockpit`, the read side, with one write for approvals

Type: build
Status: claimed
Blocked by:

## What

Map ticket 86, cut one. Read its `## Answer` first, then research 85 section 1 (what Hydra
already shows and from where).

A new subcommand, `hydra cockpit [--goal <id>] [--interval <secs>]`, a full-screen Ratatui
application the human runs in a terminal of his own, beside the chat of an orchestrating session.
It reads files and prints; it launches no provider and edits no config. Its one write is the
approval of a seat request that waits, the same code path as `hydra approve <id>`.

## Do

Add `ratatui` and `crossterm` to `Cargo.toml`. Keep every new module under `src/cockpit/`, and
nothing under `src/` outside `main.rs`, `cli.rs` and that directory changes, except a `pub` you
need on `approve.rs` or `status.rs` to reuse them. If a function you need is private and moving it
would change behaviour, stop and say so.

1. **Layout.** Three columns and a status line. Left: the task table of one goal, read from the
   goal log under the puppets home the way `integrations/puppets/puppets.py board` folds it (read
   the JSONL yourself, the events are documented in `integrations/puppets/README.md`; do not shell
   out to Python). Columns: task id, class, grade, provider, model, status, and the run id. Middle:
   a kanban of runs in four lanes, `live`, `waiting approval`, `ended ok`, `ended not ok`, built
   from `hydra status`'s liveness (`src/live.rs`, `src/status.rs`) and each record's derived
   outcome (`src/outcome.rs`, `src/show.rs`). Right: the detail of the selected run: provider,
   model, policy, outcome, tokens, the `result_text` and the last twenty record lines, scrolled.
   The status line names the goal, the refresh interval, the count per lane, and the keys.
2. **Input.** Arrow keys and `Tab` move the selection across panels; `Enter` opens the detail;
   `Esc` closes it; `q` quits. Mouse: a click on a row selects it, a click on a card opens it,
   the wheel scrolls the panel under the pointer. Hit testing is yours: keep every drawn `Rect`
   in the app state and test `Rect::contains` on the click. Enable mouse capture with crossterm
   and disable it on exit, also on panic (a panic hook that restores the terminal).
3. **Refresh.** Re-read the goal log and the records every `--interval` seconds, default 2, on a
   timer in the event loop. Reading must not block input: read on a thread, hand the app a
   snapshot. Without `--goal`, use the goal log that changed last, and say so on the status line.
   With no goal log, the left column says so and the rest still works.
4. **Approve.** A lane `waiting approval` lists the requests `hydra approve` would list
   (`src/approve.rs`). Selecting one and pressing `a` opens a confirmation that shows the request
   as `hydra approve` prints it; `y` calls the approve function, `n` closes. The record and the
   store are exactly what `hydra approve <id>` would have written. Refuse to approve when the
   cockpit runs inside a seatbelt (the same check `approve.rs` makes), and say so on screen.
5. **Nothing else writes.** No policy, no config, no run launched. The cockpit is a viewer with
   one button.
6. **`IMPLEMENTATION.md`**: a section for the cockpit, what it reads, its one write, and what cut
   two adds. `README.md`: one paragraph and the command.

## Done when

- Tests in `tests/cockpit.rs`: the fold of a fixture goal log into table rows; the lane
  assignment from fixture records of each outcome plus a live pid; the hit test over a fixed
  layout (a click at a coordinate selects the row it should); the approval calling the same code
  as `hydra approve` on a scratch store. Drive the app state without a terminal: the render and
  the event handling must be testable functions over a `Buffer`, not only a running screen.
- `cargo test --test cockpit`, `cargo test --lib`, `cargo fmt --check` and `cargo clippy
  --all-targets -- -D warnings` clean. The three `run::tests` that fail inside a worktree are
  known; ignore them.
- The binary starts, draws, and quits with `q` on a real terminal if you can drive one; if not,
  say so. The report says what stays untested without a terminal.
- Nothing on the seatbelt or the gate changes. `hydra doctor` output is unchanged.
