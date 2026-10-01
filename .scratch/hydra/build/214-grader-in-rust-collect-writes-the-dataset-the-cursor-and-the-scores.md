# 214 - grader in Rust: collect writes the dataset, the cursor and the scores

Type: build
Status: resolved
Blocked by: 212, 213

## What

Map ticket 94, the grader's port. This is "plan 212" of `.scratch/hydra/research/94-the-grader-port-plan.md`: read that section whole,
and "What puppets taught" above it. It names the Python files and functions to port, the inputs
the differential test compares on, what must be identical, what is normalized, and the one thing
most likely to make Python and Rust differ.

## Do

1. Port what the plan's section names into `crates/grader` (package `hydra-grader`, binary
   `hydra-grader`), with the Python's arguments, output and exit codes. The crate depends on
   `hydra-pyjson` and not on `hydra-puppets` or `hydra`.
2. Record the Python once: `crates/grader/tests/record.py` runs the Python on every input of this
   ticket from an empty environment, with frozen data files, and writes
   `crates/grader/tests/fixtures/`. The Rust tests compare with those files and start no
   `python3`. Follow `crates/puppets/tests/record.py` and `differential.rs`. Record with the
   `python3` on `PATH`, which is 3.14.7, and write its version into the fixtures.
3. Break the Rust on purpose once (one constant, one word) and say how many comparisons fail.
4. The Python stays. `IMPLEMENTATION.md`: a passage on what this ticket ported and what it
   normalizes.

## Done when

- The report lists each command and input compared, what was normalized and why, and what the
  plan assumed that turned out false. fmt and clippy clean over the workspace; `cargo test --lib`
  and `cargo test -p hydra-grader` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f6f0` (claude opus 5.5, `standard`, 12.5M tokens). The session took
the port on its tests, its deliberate breaks and a full live pass of its own, merged it by hand
with builds 215, 216, 217, 222 and 223, which landed while it ran, and applied it otherwise as
written.

Built: `crates/grader/src/labels.rs` (`labels.py` whole, with the shell-out to `hydra-puppets
show -- <goal>`, the absolute-path rule and `LabelsError`), `collect.rs` (`collect`,
`score_rows`, `dataset_row`, `label_digest`, `refresh_show_answer`) and `collect_cli.rs`:
`hydra-grader once [--jobs N]`, `watch --interval S [--jobs N]`, `rebuild [--jobs N]` and
`backfill`. The command runs on a thread with a 1 GiB stack, as `hydra-puppets` does.
`hydra-pyjson` gained `loads_bytes_pairs_explained`, a parse that keeps a key written twice. No
dependency added.

The merge: five files conflicted, each because two runs added a block in the same place. Both
sides were kept, except in `main.rs`, where the dispatch of `jev`, `monitor` and `triage` stays
and `collect_cli` takes what is left, and in `differential.rs`, where the test for "no command is
ported yet" stays removed.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 25 unit
and 14 differential tests; this ticket's two tests make 552 comparisons of labels over 239 goals
and 2,201 over 356 commands in 114 scenarios; `hydra-pyjson` and `hydra-puppets` unchanged.
Reversing the key order in `label_digest` failed 22 comparisons; misspelling `plan_judge` 40.

The session ran one whole pass both ways, `grader.py once --jobs 1` under 3.14.7 and
`hydra-grader once --jobs 1`, reading the real `~/hydra/runs`, the real goal logs and copies of
the real models, class files and ticket verdicts, each writing to a scratch grader home:
`cursor.json` and `show-cache.json` byte-identical; `dataset.jsonl`, 692 rows, identical but for
`collected_at`; `scores.jsonl`, 1,384 lines under the two models on disk, identical but for
`scored_at`.

Normalized: `collected_at` and `scored_at` (each must fall inside its command's run, then is
rewritten to the recorder's second); the scratch directory; a traceback by its class; a wrong
command line by its last line; five environment variables the stubs see.

Left open: with more than one worker the Python fills `show-cache.json` in arrival order and the
Rust in record order, and no comparison covers it; launchd runs `once` with the default. `watch`
was recorded only for its first pass. The helper's default path, with neither `HYDRA_PUPPETS_BIN`
nor an absolute `HYDRA_BIN`, is the first directory above the binary that holds
`integrations/puppets`, the run's choice. `aliases.json`'s lookup and the `argparse` reader are
copied from `baseline_cli.rs`, a third time with `monitor_cli.rs`. The crate's fixtures are 7.2
MB. `records.iter_records` has no caller and was not ported.

