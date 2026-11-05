# 221 - The helper is never found on `PATH`, and a goal is labelled from the file that was read

Type: build
Status: resolved
Blocked by:

## What

A read-only review by `cursor-agent` (grok-4.7-high) on 2026-10-01 of build 196 (`f4019a0`,
`16229a3`), the cutover of Master of Puppets to Rust. It confirmed that the grader writes nothing
when `hydra-puppets show` fails, that `labels.py` never searches `PATH`, and that no recorded
fixture is a failure of the recorder. Three defects are left, the first a way for a gated run's
environment to choose the binary a human's approval runs.

## Do

Tests first, each shown failing before the change.

1. **A symlinked `hydra` falls through to `PATH`** (`src/cockpit/mod.rs` near 98 to 102, spawned
   from `src/cockpit/snapshot.rs` near 461; `integrations/herdr/bin/hydra-board` near 22 to 33).
   `current_exe` returns the symlink's path on macOS, `helper_beside` looks only in that
   directory, finds nothing and returns the bare name `hydra-puppets`, and `Command::new` of a
   bare name searches `PATH`. `pending`, and the approve argv it prints, then run whatever is
   first on `PATH`, and `PATH` is on the allowlist a gated run is given (`src/env.rs` near 21).
   `hydra-board` does the same with `Path(HYDRA_BIN).parent` and `shutil.which`, and takes a
   relative `HYDRA_PUPPETS_BIN`. Canonicalize the executable first and look beside the real
   binary. If the helper is not there, do not run anything: the cockpit shows `no hydra-puppets
   beside <path>; run cargo build --release --workspace` where the steps would be, and the board
   prints the same and exits 1. `HYDRA_PUPPETS_BIN` must be absolute in both, as it is in
   `integrations/grader/labels.py`. No code path of the cockpit or the board may pass a bare
   program name for the helper: add a test that plants a `hydra-puppets` on `PATH` and a symlinked
   `hydra`, and checks the planted one is never run.
2. **A goal labelled from a file other than the one read** (`integrations/grader/labels.py` near
   193 to 196). The goal id is `Path.stem`, and for a file named `.jsonl` (or, on Python 3.14,
   any name of only dots plus `jsonl`) the stem plus `.jsonl` is not the file's name. `show` then
   opens a different file, finds none, prints the empty state with exit 0, the shape check
   passes, and the raw-event reader reads the original file: review and panel labels are written
   with no task and no goal text, and the pass goes on. Take the goal id by removing a literal
   `.jsonl` suffix; skip, with a line on stderr, a file whose id is empty or starts with a dot;
   and raise `LabelsError` when `show`'s `goal_id` is not the id asked for.
3. **A missing `vendors.json` means every provider is up** (`crates/puppets/src/log.rs` near 53
   to 80, `crates/puppets/src/fold.rs` near 523 to 528). A `hydra-puppets` copied out of its
   checkout with neither `HYDRA_PUPPETS_DATA` nor `HYDRA_HOME` set takes `<its directory>/
   integrations/puppets` as its data directory, finds no `vendors.json`, and `provider_available`
   answers true, so `review` sits a provider the checkout marks down, with no degraded approval.
   The recorded Python did treat a missing file as available, and the fixtures hold that, so
   keep that reading for a file missing from a data directory that exists. Change what happens
   when the data directory itself does not exist: every command that needs a data file fails
   with one line naming the directory and the two variables, exit 1. Say which fixtures, if any,
   run with no data directory, and do not change a fixture.

`IMPLEMENTATION.md` and `integrations/puppets/README.md`: the passages on where the helper and its
data are found.

## Done when

- Each test failed before the change; fmt, clippy over the workspace; `cargo test --lib`, `cargo
  test --test cockpit`, `cargo test -p hydra-puppets` with no fixture changed; the grader and
  Herdr Python suites pass (`test_launchd` aside).

## Outcome

Resolved 2026-10-01. Run `01a0f6ca` (claude opus 5.5, `standard`, 3.7M tokens), diff read by the
session and applied to master with one sentence added.

Built: `helper_beside` canonicalizes the executable, looks beside the real binary and returns the
path or the line to show; `helper_command` refuses a program that is not an absolute path, and
`pending`, `approve` and `review` all spawn through it, a step's own argv included. With no
helper the cockpit asks nothing and shows `no hydra-puppets beside <path>; run cargo build
--release --workspace` under the waiting lane. `hydra-board` no longer calls `shutil.which`:
`HYDRA_PUPPETS_BIN` and `HYDRA_BIN` must be absolute, `HYDRA_BIN` is resolved, and with no helper
it prints the line and exits 1. `labels.py` takes the goal id by removing a literal `.jsonl`,
skips an id that is empty or starts with a dot with a line on stderr, and raises `LabelsError`
when `show` answers for another goal. `data_dir()` fails when the directory does not exist;
commands that read no data file, `show` among them, still run.

Measured on master: fmt and clippy clean over the workspace; `cargo test --test cockpit` 112 and
one ignored (the planted-helper test re-runs the test binary as a child so `PATH` changes only
there); `cargo test -p hydra-puppets` 45, 5 and 18 with no fixture changed; `cargo test --lib`
224; the Herdr suite 44; the grader suite 135 with `test_launchd` its one error. The session
reverted `hydra-board` and saw three failures and an error, the planted helper running; reverted
`labels.py` and saw two failures; restored both. The run copied the release helper outside the
checkout: `pending` printed `[]` with exit 0 before, the one-line error with exit 1 after.

The session's sentence: `integrations/herdr/README.md` still said the board falls back to `PATH`.

What the ticket assumed that is false: the cockpit reads no `HYDRA_PUPPETS_BIN`; only the board
does, and the run added none, since a new variable that picks the binary an approval runs is a
decision. The `goal_id` check does not by itself catch a file named `.jsonl`: `show` echoes the
id it was asked for; the suffix rule and the skip are what fix that case.

Left open: `helper_path()` in `crates/puppets/src/main.rs` still falls back to the bare name in
the `argv` that `pending` prints; the cockpit now refuses such an argv, so the step cannot be
approved from the cockpit until the helper is found by path. No one ran `hydra cockpit` through
a real symlink. The board with no `HYDRA_BIN` now exits 1; `hy.zsh` always passes an absolute one.

