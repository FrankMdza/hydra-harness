# 59 - The three documents against the code, before anyone else reads them

Type: research
Status: resolved
Blocked by:

## Question

The human is deciding whether to publish and asked whether the three design documents still
describe the system as built. A read-only agent audited `docs/*.md` against `src/`, `policies/`,
`config/`, `tests/` and `IMPLEMENTATION.md` on 2026-09-12. Departures IMPLEMENTATION.md already
records are not findings. What follows is what it found, checked at the cited lines by the
session before this ticket was written.

## Answer

### docs/harness-spec.md

| Section | Claim | Code | Severity |
|---|---|---|---|
| 3, 4, 6.7, 19.6 | Interactive: no gate, no injection, any provider, no worktree | `--interactive` is `claude` only (`interactive.rs:172-199`); with `--policy` it seeds a worktree, links skills, compiles a gate (`interactive.rs:75-159`). Section 3 says so; 4, 6.7 and 19.6 still say the opposite | stale |
| 7.1 | `agy` needs both `write_to_file` and `write_file` rows | only `write_to_file` (`policy/table.rs:224`) | stale |
| 9 | usage line for `hydra run` | lacks `--interactive` (`cli.rs:81`) | stale |
| 15.1, 15.2 | code rows 1, 2, 4, 6, 7 "refused at admission" | `protected_path_hit` checks `policies/` and `config/*` only (`evolve.rs:447-456`). A rung-two patch to `review.rs`, the deny list, `gate.py`, `record/` or the admission gate reaches the panel | **wrong** |
| 17 | `evolve` row; `reap [<run-id>]`; `show <run-id>` | `evolve --task` (`cli.rs:164`); `show` and `reap` take a prefix; `status` and `logs` do not | stale |
| 18 | Hunk opens the branch against `base_tree` | nothing drives Hunk; the branch exists for it | vague |

### docs/run-record-contract.md

| Section | Claim | Code | Severity |
|---|---|---|---|
| 1 | a TUI eventually will, in a separate process | harness 18 rules a TUI out; the two documents contradict | stale |
| 5.1 | `command` is `run`, `evolve`, `resume` or `curate`; a curator run reads `command: curate` | `hydra curate` writes `command: "run"` (`curate.rs:127`). Only `run`, `resume`, `evolve` are written. "The asymmetry in 5.5 is visible in the data" is false | **wrong** |
| 5.3 | allowlist is `HOME USER PATH TERM` plus `SHELL TMPDIR` for `claude` | also six proxy names, `NODE_EXTRA_CA_CERTS` on claude and cursor, `CURSOR_DATA_DIR` on cursor (`env.rs:21-47`) | vague |
| 5.6 | interactive run: policy, worktree, skills, scopes null or absent | with `--policy` all filled (`interactive.rs:240-251`); no sentence admits the second shape | stale |
| 7.2.1, 7.7 | `permission_decision` exists only on a recorded run | harvest derives a `deny` by `hydra_policy` from every error result starting `hydra/`, with a limit that allows were not recorded (`normalize/claude.rs:1285-1321`) | stale |
| 8.5 | `rate_limit_event` carries `status` at the top of the frame | IMPLEMENTATION.md:2070 measured it nested under `rate_limit_info`; `rate_limit_abort` reads `raw.status` (`evolve.rs:697`) and never fires. Doc wrong, code copied it | **wrong** |
| 16.1 | two files from one traversal, `sft.jsonl` and `preference.jsonl` in TRL's shapes | `hydra export` writes one header line plus `{"events": [...]}` per run (`export.rs:304-318`). No SFT or preference shape exists | **wrong** |
| 18.4 | "Hydra has run zero times" | 140+ records | stale |

### docs/ouroboros-spec.md

| Section | Claim | Code | Severity |
|---|---|---|---|
| 1 | rung two "fully specified here and shipped later" | built since build 36; eight passes | stale, more conservative than the code |
| 1, 8 | "Hydra has run zero times" | see above | stale |
| 3.2 | retry loop derived from the `parent_run_id` tree | counted as `parent_run_id.is_some()` (`corpus.rs:193`): every child, judge and proposer is a "retry" | vague |
| 3.8 | the named class becomes the measurement afterwards | nothing measures a count after a merge | vague |
| 3.9 | rules 3, 6, 12 verbatim | no code; rules 1, 2, 5, 10 reach the brief as prose (`curate.rs:271-410`), 3, 6, 12 reach nothing | vague |
| 4.1 | the pass diagram | missing the 400-line cap before the path check, the rate-limited abort, and `--task` | stale |
| 4.4 | "a patch that removes a check in the quorum touches the review module, refused at admission; two of three judges approving it is not a state this design can reach" | admission refuses five config paths only. A patch to `review.rs` goes to the panel, and the `severity_ceiling_table` item is the only thing holding it | **wrong** |
| 4.5 | protected test files come from the baseline sha | `admission_gate` runs `cargo test` in the patched worktree and reads nothing from a baseline (`evolve.rs:472-480`). The `baseline_test_provenance` judge item polices a mechanism that does not exist. It was still the item `agy` blocked pass 7 on, correctly | **wrong** |
| 4.7 | before a review, read the last `claude` run's `rate_limit_event` | runs after the proposer, so the last run is the proposer; and reads the wrong field, so never aborts | **wrong** |
| 5.2 | `changelog_accuracy` | no changelog exists | vague |
| 5.3, 5.4 | "two dialects" | `gate.py` has three (`gate.py:69-83`); the judge prompt inherits the wrong number (`review.rs:325`) | stale |

### The five fixes that matter most for a first reader

1. Contract 5.1: `command` never carries `curate`. Build ticket 42 writes it.
2. Contract 16.1: say what `hydra export` writes today and that the TRL shapes are not built.
3. Harness 15.2 and ouroboros 4.4, 4.5: admission refuses `policies/` and `config/` only; the
   code rows are panel-guarded; baseline test provenance is not built. Build ticket 40 closes
   the first half; the second is a decision, since the spec's opening scenario rests on it.
4. Contract 5.6, 7.2.1, 7.7 and harness 4, 6.7, 19.6: the gated interactive shape exists and
   is `claude` only.
5. Ouroboros 4.7 and contract 8.5: the early abort is dead code. Build ticket 41.

### Also found

`propose()` at `curate.rs:254-264` runs `git branch -d` on its own proposal branch after
removing the checkout, on every pass; a real proposal is never merged, so the delete always
fails and lands in `left`. No document claims that behaviour. Build ticket 43.

IMPLEMENTATION.md:2676 still says the `correction` class has no producer while `corpus.rs:382`
counts corrections; the contract is right and IMPLEMENTATION.md is behind.
