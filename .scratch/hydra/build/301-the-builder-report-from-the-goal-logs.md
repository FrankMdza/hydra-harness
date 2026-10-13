# 301 - The builder report: acceptance, rejections and cost per task class and per model, from the goal logs

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session, from the map 117 panel (`117-synthesis.md`, question 4),
where all three seats said the numbers that judge any builder tier come from goal-log events
that exist today and no report prints them.

## What

`hydra-puppets` records, per task, its class and grade (`task`), the provider and model it was
assigned (`assignment`), each launch and its run id, each run's end with outcome and tokens
(`run_end`, written by `wait`), and the human's verdict (`applied`, `fixed`, `rejected`, in
`verdict`). `sheet.rs::rank` folds the verdicts of one goal into a first-pass bound. Nothing
prints the same facts across every goal on the machine, which is what the human needs to say
whether a cheaper builder pays: before changing who builds, see what each builder has cost and
how often its work stood.

## Do

1. `hydra-puppets report builders`: over every goal log under the puppets home, one row per
   task class and provider-and-model pair, with: tasks, first-pass acceptance (`applied` on the
   first attempt), eventual acceptance (`applied` or `fixed` on any attempt), rejections,
   attempts per task, total tokens and tokens per accepted task, and the number of tasks with no
   verdict yet. A `--class` and a `--model` filter; `--json` for the same rows as one object per
   line. Totals per provider-and-model at the bottom. A goal log whose tail is unterminated is
   read up to its last complete line and counted, with its id on stderr, the way `repair` treats
   it.
2. Follow the fold's definitions, not new ones: what `rank` counts as a first-pass acceptance is
   what this counts, and an `applied` on a resumed run is `fixed` here too. Read `sheet.rs` and
   `fold.rs` before writing, and reuse their functions rather than re-deriving the events.
3. One section in `integrations/puppets/README.md` under the commands, and one paragraph in
   `IMPLEMENTATION.md`.

## Done when

- A test in `crates/puppets/tests/` on a fixture of three goal logs (one with two attempts and a
  `fixed`, one rejected, one with no verdict and an unterminated tail) checks every column and
  the totals, text and `--json`; fmt, clippy, `cargo test -p hydra-puppets`. The report run on
  this machine's real goals goes in the Outcome, by the session.

## Outcome

