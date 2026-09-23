# 81 - The concurrency cap reserves a slot rather than counting

Type: build
Status: resolved
Blocked by:

## What

Panel finding F30, stated as a limit in `IMPLEMENTATION.md`. `src/run.rs:446` calls
`live::guard(LIVE_RUN_CAP, &live::scan(&layout)?)`, which counts live runs and refuses at eight
(`src/live.rs:287`). Nothing is reserved between the count and the spawn, so nine launchers that
count at once all see seven and all launch.

Make the check a reservation. The linearization point is a file created with `create_new` (or an
`flock` held across count and create) under the layout's spawn directory before counting, keyed by
the new run id. A reservation whose launcher is dead is released by liveness, pid plus process
start time, the way `live.rs` already decides a run is dead; a reused pid is ruled out by the start
time. The reservation is released when the run's own record carries its pid, or when the launch
fails. Keep `TooManyLive` as the refusal type and keep "no run record on a cap refusal"
(`src/live.rs:316`).

## Done when

A test in `tests/status.rs` or a new file starts N threads that each try to reserve against a
scratch `Layout` with cap C < N, and exactly C succeed. A reservation left by a dead pid does not
count. Launches nothing. Tests take a scratch `Layout`, never `Layout::discover()`. Edits `src/`
and `tests/` only. Clippy and fmt clean.

## Outcome

`live::reserve` holds an `flock` on `<root>/reservations/.lock` across reading pending
reservations, scanning live runs and renaming `<run_id>.json` into place; a `Reservation` guard
removes the file once line 0 carries the child pid, or on any early return. Tests in
`tests/status.rs`: sixteen threads behind a barrier against a cap of three, exactly three reserve;
a dead or reused launcher pid releases; an unparseable reservation is stale and removed; a refused
duplicate leaves no temp file. The ticket assumed the reservations could live under `spawns/`, and
that was false: the Opus review found `reap::orphan_spawns` treats every directory there as a run
and would delete the lock. They moved to `<root>/reservations/`, on `denied_write_paths`, and
`tests/cleanup.rs::a_sweep_leaves_pending_reservations_alone` fails if they move back. Interactive
launches never reserve, so the cap does not count them; stated in `IMPLEMENTATION.md`.
Measured on master with `HYDRA_ROOT` empty: status 20, cleanup 29, spawn 29, delegation 14, lib 188
passed; clippy and fmt clean. Built by `codex` under `standard`, run 01a0cc2b-e29c, 1.3M tokens,
after a first run (01a0cc2a-f20b, 127k tokens) stopped on an unreadable skill, which is map ticket
71. The session's Opus reviewer found the reap bug and an Opus subagent fixed it in the worktree.

One more thing the ticket did not foresee, found by the session after 79 and 83 ran: on this
machine `~/hydra` is the checkout, and `reservations/` was not in `.gitignore`, so `seed_tree`
copied each launcher's own reservation into its worktree. Fixed in `e734038`.
