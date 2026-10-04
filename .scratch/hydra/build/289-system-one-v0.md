# 289 - System 1 v0

Type: build
Status: resolved
Blocked by: 288

## What

Map 109, second step (`.scratch/hydra/research/109-a-system-one-of-our-own.md`): embeddings of the diff and the task from the Ollama the apprentice
already uses, a logistic head with a temperature over `crates/grader/src/baseline.rs`, measured
with Brier and AUC against the base rate per provider and against Jev's 25 answers of build 134.
Answers task class, grade, outcome and, once labelled, `needs_correction`. Nothing leaves the
machine.


**From 288 (2026-10-04):** `hydra-grader originals` finds only 7 of 70 verdicts with their
original diff; the other branches were deleted. A head trained on diffs has 7 examples until
more runs are reaped with their branches kept, so v0 should start from the task text and the
record's features, which all 70 have, and add the diff when there are enough.

## Outcome

Built 2026-10-04 by a Grok 4.6 run (`01a1070d-abba`), amended and measured by the session.

`hydra-grader system-one` (`crates/grader/src/system_one.rs`, `system_one_cli.rs`): embeds each
labelled run's `task_text` through a loopback Ollama `/api/embed` (`HYDRA_EMBED_MODEL`, default
`nomic-embed-text`), appends the baseline's record features, fits a logistic head on the earlier
half of the slice before `--test-from`, a temperature on the later half, and prints Brier, log
loss and AUC for `outcome` against the constant and per-provider base rates. It writes nothing.
v0 uses no diff, and the head's minority floor is 3 rather than the baseline's 10.

**Found by the session on the real data:** 27 of 855 rows have no task text, Ollama answers an
empty input with no vector, and the command aborted on the first one. Those rows now get zeros
and no embed call; `a_run_with_no_task_text_is_not_sent_and_does_not_stop_the_fit` fails
without that with the same error.

**Measured on this machine's dataset** (`nomic-embed-text`, 768 dimensions, 2 min 20 s), on the
test slice from 2026-09-23 (559 runs, 41 not `completed`):

| | Brier | AUC | Log loss |
|---|---|---|---|
| Constant base rate | 0.0744 | 0.500 | 0.292 |
| Base rate per provider | 0.0778 | 0.579 | 0.296 |
| `baseline train`, record features only | 0.0770 | 0.746 | 0.272 |
| `system-one`, plus the task embedding | 0.0705 | 0.787 | 0.252 |

The embedding adds 0.04 of AUC and about 8% of Brier and log loss over the record features, on
41 positives. Temperature 1.02, so the head was already close to calibrated. Not measured: task
class, grade and `needs_correction`, and the comparison with Jev's 25 answers of build 134.

**A gate gap, seen again from an honest agent:** the run started `hydra-grader system-one` with no
home set, and the binary resolved and read the real grader dataset under `~/hydra`; it stopped
because Ollama was down. The shell check reads only the paths a command names.
