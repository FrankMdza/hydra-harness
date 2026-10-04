# 220 - grader in Rust: the cutover

Type: build
Status: resolved
Blocked by: 213, 214, 215, 216, 217, 218, 219

## What

Map ticket 94, "plan 217" of `.scratch/hydra/research/94-the-grader-port-plan.md`. The last of the grader's port: launchd runs
`hydra-grader once`, and the Python is deleted.

## Do

0. Close what the reviews of builds 212 to 219 found, each with a test first. Known on
   2026-10-01, from the `codex` review of 214, 215 and 216 (the human's
   `builds/review-codex-214-215-216.md`):
   - **Where `aliases.json` is read from** (`crates/grader/src/collect_cli.rs` near 37). The Python
     reads the file beside its own checkout and ignores the environment. The Rust reads
     `HYDRA_PUPPETS_DATA`, else `HYDRA_HOME`, else above the binary, so a `HYDRA_PUPPETS_DATA`
     that names a directory with no `aliases.json` aborts a pass the Python completes, and
     another alias table silently changes the `model` written to the dataset. The recorder hides
     it by handing the Python a table. One rule for the binary, the same `hydra-puppets` uses
     since build 221, written down in the README; a missing file is an error that names the
     directory; and the plist names the directory explicitly.
   - **The helper's default path** (`crates/grader/src/labels.rs` near 238). With neither
     `HYDRA_PUPPETS_BIN` nor an absolute `HYDRA_BIN`, a binary installed outside the checkout finds
     no helper, where the Python found its checkout's. The plist sets `HYDRA_BIN` to an absolute
     path; say so in the README and make the error name the two variables.
   - **`TMPDIR` that does not exist** (`collect.rs` near 124). Python's `TemporaryDirectory` falls
     back to a usable directory; the Rust takes the first non-empty variable and fails. Only the
     library's path for a runs directory not named `runs` reaches it.
   - **Overflow in the monitor's scaling** (`monitor.rs` near 842). Forty training records with a
     usage count of `10**200` make Python raise `OverflowError` in the scaling; the Rust's `pow`
     gives infinity, goes on, fails later with `ValueError` when it serializes the model, and can
     have created the model's parent directory by then. Raise at the scaling, before any write.
   - **Two passes at once** share one `.tmp` name per file, in both implementations: one fails
     with `FileNotFoundError`. launchd starts a pass every fifteen minutes and a slow one can
     overlap the next. Take an `flock` on the grader home for the length of a pass, and have the
     second pass say it found one running and exit 0.
   - With `--jobs` above 1 the two fill `show-cache.json` in different order. Nothing reads the
     order and the dataset and scores do not change: leave it, and say so in `IMPLEMENTATION.md`.
   - `baseline_cli.rs`, `monitor_cli.rs` and `collect_cli.rs` each carry a copy of the `argparse`
     reader and of the alias lookup: one copy.
   - `record_baseline.py` writes the checkout's path into tracebacks; make it write `<repo>`, as
     `record_monitor.py` and `record_collect.py` do, so recording again changes no other
     ticket's fixture.
1. The gate, run by the session on the real grader inputs (a run cannot read them): both
   implementations over the real runs, goal logs, models, class files and ticket verdicts, each
   writing to a scratch home, identical output. Measured once already on 2026-10-01, after build
   214: against `grader.py once` under CPython 3.14.7 everything is byte-identical but the two
   wall-clock stamps (692 dataset rows, 1,384 scores). Against `/usr/bin/python3`, which is 3.9.6
   and is what the launchd job runs today, `cursor.json`, `show-cache.json` and `dataset.jsonl`
   are identical and `scores.jsonl` differs in the last digit of `p` on 136 of 1,390 lines
   (`0.7363960157201249` against `0.736396015720125`): 3.9's `sum()` is not compensated. So the
   cutover changes the last digit of about one new score in ten and nothing else; `scores.jsonl`
   is append-only and `backfill` scores only pairs that are missing, so no old score is rewritten.
   Map 94 asked for the launchd job to move to Python 3.14 and the models to be retrained first.
   The measurement says neither is needed for the port to be exact: the models are files both
   implementations read alike, and training is byte-identical with 3.14.7. Whether to retrain
   anyway is the human's call, recorded in his pending list. Run the gate once more before
   deleting, on `once`, `rebuild`, `backfill`, `baseline train`, `monitor train`, `triage run`.
2. The plist template runs `hydra-grader once`; `test_launchd.py` and
   `test_labels_recorded.py` become Rust tests.
3. `records.py` and `redact.py` move beside their remaining callers, `apprentice.py` and
   `record_to_otlp.py` (find them with `grep -rn 'records.py\|redact.py' integrations`); every
   other file under `integrations/grader/` and its tests is deleted. The data the binary reads
   stays where it finds it.
4. CI, the READMEs and `IMPLEMENTATION.md` follow. `hydra-puppets` finds `scores.jsonl` as before.

## Done when

- fmt and clippy over the workspace; `cargo test --workspace`; the remaining Python suites pass;
  outside `.scratch/`, `docs/` and the fixtures, `grader.py` is named only where a sentence says
  it was deleted.

## Built and held, 2026-10-01

Run `01a0f74f-70c5` (claude opus 5.5, `standard`, 22.0M tokens) did the cutover in its worktree.
The diff is in the human's `builds/diff-220.patch` (65 files, 2,525 lines added, 5,996 removed)
and the worktree is kept. **It is not on master**: the launchd job installed on this machine
runs `/usr/bin/python3 integrations/grader/grader.py once` from this checkout every fifteen
minutes, and the diff deletes that file. Master takes it after the human has replaced the job
with the two commands in `integrations/grader/README.md`, section "Schedule" (in the diff), or
has told the session to. The ticket stays `claimed` until then.

What the diff does: item 0 whole (one data-directory rule shared by both binaries, in
`hydra-pyjson`; one `argparse` reader; the helper error names its two variables; the scratch
directory falls back as `tempfile` does; an `flock` on the grader home for the length of a pass,
and a second pass says `grader: another pass is running in <home>; nothing done` and exits 0; the
monitor's scaling raises `OverflowError` before any write; recorders write `<repo>`). The plist
template runs `target/release/hydra-grader once`. `grader.py`, `baseline.py`, `monitor.py`,
`triage.py`, `judge.py`, `labels.py`, `jev_probe.py` and their tests are deleted; `records.py`,
`redact.py` and `models.py` move to `integrations/apprentice/` with their tests.
`test_launchd.py` is `crates/grader/tests/launchd.rs`, and `test_labels_recorded.py` is
`crates/puppets/tests/grader_labels.rs` (147 logs equal, 291 labels, 10 verdicts, 17 refused, the
numbers the Python test gave). `libc` is added to the grader for `flock`.

The run's results in its worktree: fmt and clippy clean; `hydra-grader` 41 unit, 6 cutover, 17
differential, 8 launchd; `hydra-puppets` and `hydra-pyjson` pass; otel 6, Herdr 46, apprentice 60
under 3.9.6.

**The gate, run by the session with the diff's binary** against master's Python under 3.14.7,
reading the real runs, goal logs, models, class files and ticket verdicts, each side writing to a
scratch home: `once`, `rebuild`, `backfill` and `triage run` give the same exit code, stdout and
stderr; `cursor.json` and `show-cache.json` are byte-identical; `dataset.jsonl` (697 rows),
`scores.jsonl` (1,394 lines) and `triage.jsonl` (78 rows) are identical but for their
wall-clock stamps; `baseline train` and `monitor train --at 0.5` for `outcome` and `verdict`
write byte-identical models and reports.

Not yet done: the suite on master with the diff applied, a review of the diff, and
`hydra-grader once` started by launchd.

Found by the run, for after: the lock covers `once`, `rebuild`, `backfill` and `watch` only;
`label-import` rewrites `dataset.jsonl` without it. The lock creates the grader home when it is
missing. `src/cockpit/window.rs` and `tests/cockpit.rs` still name `sheet.py` and `models.py` in
comments. The apprentice's and otel's own tests are not in CI. `record_sheet.py` and `record.py`
no longer run from this tree.

## Outcome

Applied 2026-10-03 by the session, at the human's word ("Sí, aplícalo tú"), from run
`01a0f74f-70c5`'s diff (`builds/diff-220.patch`), with one conflict in `README.md` resolved to
name both the grader's port and builds 252 and 262.

Measured on master with the diff applied: `cargo test --workspace` 1,233 passed and none failed;
`pty_tee` repeated alone after its first run hung under load, 4 of 4 in 422 s; clippy over the
workspace and fmt clean. The launchd job was swapped the same evening with the two commands in
`integrations/grader/README.md` "Schedule": the plist runs `target/release/hydra-grader once`,
and its first pass started at load, so a pass typed by hand a moment later answered `grader:
another pass is running in /Users/USER/hydra/grader; nothing done`, the new lock working.

Not done: a line-by-line review of all 2,525 added lines; the session read item 0's code paths
named in the ticket and trusted the run's own differential and recorded tests for the rest, as
the gate of 2026-10-01 had already compared every command's output byte for byte. Builds 224 to
238 are unblocked.
