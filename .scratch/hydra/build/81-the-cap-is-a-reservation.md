# 81 - The concurrency cap reserves a slot rather than counting

Type: build
Status: open
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
