# 301 - The builder report: acceptance, rejections and cost per task class and per model, from the goal logs

Type: build
Status: resolved
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

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a13438-d161-7bb2-ac17-a6f4cd2e6398` under
`standard`, 1.92M tokens, applied to master on 2026-10-13 after the diff was read whole.
`hydra-puppets report builders` is `crates/puppets/src/report.rs`; the verdict rule moved out
of `rank` into `sheet::builder_verdict` and `first_builder_verdict` so the ranking and the
report count the same thing, and a numbered repeat attempt counts as a continuation. Three
fixture logs pin every column, the totals, the filters and the torn tail. fmt, clippy and
`cargo test -p hydra-puppets` green on master. One cost worth knowing: the report re-folds the
goal up to each launch event to find the assignment in force, quadratic in a goal's events,
which is nothing at today's sizes.

Run on this machine's real goals the same night: two goal logs, eleven tasks. composer-2.5 on
`batch_scripting`: 5 tasks, 3 first-pass, 5 eventually accepted, 4.55M tokens; claude opus on
one `security_review`: accepted on a second attempt, 1.14M; grok 4.6 on four tasks and gemini
on one: no verdict. That is the whole corpus `hydra-puppets` can see, because the builds of
the last month, tonight's eight included, were launched with `hydra run` straight from a
ticket and never through a goal log; their verdicts live in the grader's `ticket-verdicts.json`
(build 288). The number the map 117 and 118 panels asked for is therefore eleven, and the
report says so without anyone having to count by hand.
