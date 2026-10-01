# 195 - puppets in Rust: the commands that launch

Type: build
Status: resolved
Blocked by: 194

## What

Third of four tickets for map 90's `puppets` port: the commands that start runs or other
programs: `plan`, `plans`, `launch`, `wait`, `review` (including `--no-open` and the Hunk comment
pusher), and the judge panel `choose` and `review` start.

## Do

Port them with the same arguments, output, exit codes and log lines. Launching goes through the
same `hydra run` argv the Python builds (compare the argv, never launch a real provider in a
test): the tests put a stub `hydra` and a stub `hunk` on `PATH`, record their argv and environment,
and compare what the Python and the Rust hand them, and the resulting logs. The Python stays.

## Notes from 194

- `crates/puppets/src/write.rs` holds the eight writers. `choose --judge` stops after its three
  refusals with nothing launched; the launch it leaves is `run_judge_panel` and `spawn_judge_run`
  in `cmd_choose`. Start there.
- `sheet.rs` has `rank`, `beta_lower` and `ranked_assignment` only. `build`, `budget` and
  `corpus_records` in `sheet.py` call `hydra show` per run; port them here if a launch command
  reaches them, and say so if none does.
- `models.rs` is ported and nothing calls it yet.
- The differential test's comparator masks appended `ts` values and a generated goal id. For
  launches it will also need run ids and whatever a stub `hydra` prints: use a stub binary named
  by `HYDRA_BIN`, never a real provider.
- Build 204 changes `replace_log` and stdout encoding in the same crate; rebase on it if it has
  landed.

## Done when

- fmt and clippy over the workspace; `cargo test -p hydra-puppets`; every command compared.

## Outcome

Resolved 2026-10-01. Run `01a0f638` (claude opus 5.5, `standard`, 12.9M tokens, 27 minutes). The
session read `launch` line by line against `cmd_launch` and took the rest on its tests; applied
to master as written.

Built: `crates/puppets/src/launch.rs` with `plan`, `plans`, `launch`, `wait` and what every
launch shares (the `hydra run` argv, its SHA-256, the run id read off stderr, the Herdr pane,
`hydra show`); `panel.rs` with `choose --judge` and `review`, the seat table, degraded
substitution, both prompts, answer parsing and both verdicts, and the Hunk comment pusher.
`launch` refuses on `launch_refusal`, appends `launch_intent` before it spawns and `run` after,
as the Python does. A `JSONDecodeError` now carries Python's message and position, because a
judge's unparseable answer lands in the `review` event's `reasons`.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-puppets` 60
unit tests and 14 differential tests; the launch test runs 87 commands and compares stdout,
stderr, the exit code, the goal log, the plan files, and each stub call's argv, environment,
working directory and stdin; with `--timeout 0` changed to `--timeout 1`, 47 of 87 fail; the
ignored real-log test passed; `cargo test --lib` 224 passed; the puppets Python suite 205 passed.

Departures from the Python, both deliberate: where the Python forks to push Hunk comments, the
Rust starts its own binary again as a hidden `push-hunk-comments <repo>` command with the payload
on stdin; inside Herdr with no finding worse than minor, the Python sleeps through thirty
one-second tries with nothing to push and the Rust prints at once.

What the ticket assumed that is false: no launch command reaches `sheet.py`'s `build`, `budget`
or `corpus_records`, so they are still not ported, and `models.rs` still has no caller. Build 196
has to say what happens to them when the Python goes.

Left open: no real `hydra run`, Hunk session or Herdr pane was ever started by the Rust; the
stub's `hydra show` shape was copied from `src/outcome.rs` and the fixtures, not compared with
the real command; three paths were not compared because each takes thirty seconds or never ends
(a `review` whose findings no Hunk session takes, a `review` outside Herdr with no `hunk`, a
panel run with no `run_end`); the Python loses output before `execvp` when stdout is a pipe, so
the test sets `PYTHONUNBUFFERED=1`.

