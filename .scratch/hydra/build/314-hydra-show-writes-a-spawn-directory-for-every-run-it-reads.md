# 314 - `hydra show` writes a `spawn/<run-id>/` directory for every run it reads

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session. Build 312's `report tickets` points `HYDRA_ROOT` at a
temporary layout whose `runs/` is a symlink to the real records and calls `hydra show <id>
--events` once per run named in a ticket. After 265 shows the temporary root held
`spawn/<run-id>/` for every one of them, each empty, and the report's `Drop` could not remove
the root because `remove_dir` refuses a non-empty directory. The root was left behind in the
repository checkout as `.puppets-report-<pid>-0/`.

## What

`hydra show` is a read. The record contract and `CLAUDE.md` describe `reap` and `prune` as the
two commands that change the layout, typed by a human. Something on the show path creates
`Layout::spawn(run_id)`: the likely place is the replay of the gate (`src/normalize/mod.rs`,
`ReplaySpawn::of`, or `src/policy/replay.rs`), which compiles the gate the run would have been
given and may write it, or `CheckoutGit::for_run`, which reads the saved `.git-context.json`
from the spawn directory and may create the parent on the way. Whichever it is, a read command
that creates a directory per record under the layout is a bug: on the real layout it hides
which spawn directories were made by launches, and `hydra reap`'s sweep of `spawns()`
(`src/reap.rs` line 742) then lists them.

## Do

1. Find it: `HYDRA_ROOT=<scratch> hydra show <id> --events` against a copy of one fixture
   record, then `find <scratch>`. Name the function that created the directory in the Outcome,
   with the line.
2. Make the read not write. If the gate replay needs a file on disk for the interpreter, it
   writes under a temporary directory it removes, or under the reader's own cache, never under
   `Layout::spawn(run_id)`. If `CheckoutGit::for_run` creates the parent, it stops.
3. `crates/puppets/src/report/tickets.rs::ShowRoot::drop`: `remove_dir_all`, so the root never
   outlives the command whatever a future `hydra` leaves in it, and the symlink to the real
   records is removed first so `remove_dir_all` cannot follow it.
4. A test in `tests/show.rs`: show one fixture record under a scratch `HYDRA_ROOT` and assert
   the scratch holds nothing it did not hold before. One in `crates/puppets/tests/tickets.rs`:
   after `report tickets --runs`, no `.puppets-report-*` directory remains in the cwd.
5. On this machine, say in the Outcome how many `spawn/<id>/` directories under `~/hydra/spawn`
   are empty and have no matching worktree or record activity, so the human can decide whether
   they are launches or shows. Do not delete any.

## Done means

A show under a scratch root leaves the root as it found it; the report cleans up after itself;
the two tests; `cargo nextest run --release --workspace` and `cargo test --release --workspace
--doc` green. Amend `IMPLEMENTATION.md` where `show` or the replay is described.

## Outcome
