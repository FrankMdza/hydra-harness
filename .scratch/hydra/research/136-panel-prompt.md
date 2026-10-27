# Decision panel for map ticket 136

You are one seat on a four-seat decision panel. Read only: do not edit, create or delete a file in the repository, and do not commit. Scratch experiments under `/tmp` are fine. Your seat name is in your task line.

The human is away and said the panel decides. Your answer counts as a vote, so commit to one option.

## Read

1. `.scratch/hydra/issues/136-does-a-runs-process-group-die-with-the-run.md`.
2. `src/watch.rs`, the end of a run and `PTY_HELD_NOTE`; `src/stop.rs`, the group signalling and `GRACE`; `src/pty.rs`, how the child leads its group.
3. `src/sandbox.rs`, `wrap_bwrap_command` and `compose_bwrap`, for `--die-with-parent`.
4. `IMPLEMENTATION.md` around lines 36 to 46 and 488 to 502.
5. `src/reap.rs` and `src/worktree.rs` `sweep_private`, for what a surviving descendant can race.

## Answer

Pick A, B, C or D. Then:

- Say what your choice does to a descendant that ignores SIGTERM and SIGHUP, and to one that called `setsid`. If you can measure on this Mac with a small script under `/tmp`, do, and say so.
- Name every legitimate process that dies under your choice, with the provider that starts it, and whether a run's result depends on it surviving.
- Say where in the code the change goes, with file and line, and how the record shows it.

Then list the build tickets your answer needs, one line each, smallest first.

At most 700 words. End with `## Pending from me`, `## Changed` and `## Found`.
