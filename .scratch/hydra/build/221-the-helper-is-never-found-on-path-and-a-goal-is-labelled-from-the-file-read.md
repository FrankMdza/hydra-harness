# 221 - The helper is never found on `PATH`, and a goal is labelled from the file that was read

Type: build
Status: claimed
Blocked by:

## What

A read-only review by `cursor-agent` (grok-4.7-high) on 2026-10-01 of build 196 (`9d9d836`,
`79b43b6`), the cutover of Master of Puppets to Rust. It confirmed that the grader writes nothing
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
