# 220 - grader in Rust: the cutover

Type: build
Status: open
Blocked by: 213, 214, 215, 216, 217, 218, 219

## What

Map ticket 94, "plan 217" of `.scratch/hydra/research/94-the-grader-port-plan.md`. The last of the grader's port: launchd runs
`hydra-grader once`, and the Python is deleted.

## Do

0. Close what the reviews of builds 212 to 219 found, each with a test first.
1. The gate, run by the session on a read-only copy of the real grader home (a run cannot read
   it): both implementations over the real `dataset.jsonl`, models and runs, identical output.
   The human moves the launchd job to Python 3.14 and retrains the models once before this, so
   the Python side of the gate is the reference the fixtures were recorded with.
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
