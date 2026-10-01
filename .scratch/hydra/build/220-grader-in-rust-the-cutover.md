# 220 - grader in Rust: the cutover

Type: build
Status: claimed
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
