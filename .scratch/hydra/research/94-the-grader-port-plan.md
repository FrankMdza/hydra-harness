# 94 - The grader's port to Rust, a plan

Written 2026-10-01 by a read-only run (claude opus 5.5, `01a0f6b6-a5ce`, 1.0M tokens) for map
ticket 94. The plan's own ticket numbers, 210 to 217, are not the build's: the panel's answer
added two tickets in front and one behind, so read "plan 210" as build 212, and so on up to
"plan 217" as build 220. The build tickets carry the mapping.

## Not confirmed

- I ran no cargo and no Rust, so every "this will differ" below comes from reading code.
- The real `~/hydra/grader` and `~/hydra/runs` are refused to me. File sizes, which Python trained the models on disk, and whether the launchd job is installed are unknown.
- I measured that Python 3.9.6 and 3.14.7 train different models. The cause, compensated `sum()` over floats since 3.12, is from memory.
- `monitor.py train`, `triage.py run`, `judge.py` and `jev_probe.py` were not run twice. Their determinism claims are from the source.
- 37 of 133 grader tests failed here only because no `hydra-puppets` is built under `target/`. I did not see them pass.
- I read only the function list of `sheet.py`, and only the imports of `apprentice.py` and `record_to_otlp.py`.
- Ticket numbers assume 209 is the last build ticket.

## What puppets taught

- Record the Python once into fixtures and replay them. No test starts `python3` (`crates/puppets/tests/differential.rs:1-6`).
- Run every recorded command from an empty environment, with frozen data files and a fixed commit time (`IMPLEMENTATION.md:3454-3457`).
- Break the Rust on purpose in each ticket and count the failures. 193 and 195 did.
- No `serde_json` in the crate: `preserve_order` would reach `hydra` in a workspace build (193 Outcome). `json.rs`, `py.rs`, `pysort.rs` and `models.rs` already do Python's JSON, sort and `canonical`.
- What went wrong:
  - After the cutover the fixtures cannot be recorded again (`IMPLEMENTATION.md:3469-3473`).
  - They were recorded on arm64 macOS with CPython 3.14.7 only.
  - `lgamma` was matched to 1e-12, not to the byte (194 Outcome).
  - Ticket assumptions about the Python were false in all four tickets.
  - Ten follow-up tickets, 197 to 209, came from reviews.

## The plan

Eight tickets, 210 to 217. Each adds `crates/grader/tests/record.py` cases and fixtures for its own part. The Python stays until 217.

**210. grader in Rust: the crate, the record scan and the redactor**
- Ports: `records.py` except `refresh_show_answer` and `iter_records`; all of `redact.py`.
- Depends on: question A below.
- Inputs: the 12 records under `tests/fixtures/records/`, and the strings in `test_redact.py` and `test_records.py`. Synthesize a torn tail, a garbled interior line, a line 0 that is not an object, invalid UTF-8, a non-ASCII character before an `sk-` key, and `\x1c` as the character after a secret.
- Identical: the fingerprint, `record_row` as `json.dumps` bytes, `write_json` and `write_jsonl` file bytes, each redacted string, and the stderr skip lines.
- Normalize: the record's directory in paths.
- Most likely to differ: `str.isspace()` at `redact.py:92` counts `\x1c` to `\x1f` as space and Rust's `char::is_whitespace` does not. Second is `datetime.fromisoformat` at `records.py:106`, which accepts more spellings on 3.11 and later than chrono does.

**211. grader in Rust: the logistic baseline trains, reports and scores**
- Ports: `baseline.py` lines 51 to 276 and 373 to 398, plus the `train` and `score` commands.
- Depends on: 210, question B.
- Inputs: a synthesized dataset of about 180 rows across both sides of `--test-from`, like the one I used. One dataset per label, one below the `MIN_MINORITY` floor, and rows with `model: null` and non-ASCII `task_text`. For `score`, the 12 fixture records.
- Identical: the model file bytes, including the `version` digest, the report on stdout, the refusal text and the exit code.
- Normalize: nothing. That is the point of the ticket.
- Most likely to differ: `sum()`. It appears at `baseline.py:109`, `:189`, `:190`, `:131`, `:137`. A plain Rust fold matches 3.9 and not 3.14. After that come `math.exp`, `log` and `log1p` against Rust's libm, then `float_repr`. Because `version` hashes the weights (`baseline.py:205`), one bit off changes the model's name. If bytes cannot be matched, map 90's fallback applies: ported tests plus a stated tolerance.

**212. grader in Rust: collect writes the dataset, the cursor and the scores**
- Ports: `labels.py` whole, all of `grader.py`, and `records.refresh_show_answer`.
- Depends on: 210, 211.
- Inputs: the homes `test_grader.py` builds with its stub `hydra` (`test_grader.py:93`), the goal logs in `crates/puppets/tests/fixtures/read/` with their recorded labels, and synthesized `task-classes-*.jsonl`, `ticket-verdicts.json` and two model files.
- Identical: `dataset.jsonl`, `cursor.json`, `show-cache.json`, `scores.jsonl`, stdout, and the stub's recorded argv and environment. Run `once`, `once` again, `rebuild` and `backfill`.
- Normalize: `collected_at` and `scored_at`, which are wall clock. Also the temp directory in `HYDRA_ROOT` when the runs directory is not named `runs` (`records.py:122`).
- Most likely to differ: key order. `show-cache.json` and `cursor.json` keep insertion order, and with `--jobs 4` the cache order follows thread completion (`records.py:143`). Record with `--jobs 1`. `label_digest` uses `sort_keys=True` on a model-written dict (`grader.py:85`).

**213. grader in Rust: the monitor reads a prefix of a record**
- Ports: `monitor.py` whole.
- Depends on: 211.
- Inputs: the 12 fixture records at the three fractions for `features`, and the frames `test_monitor.py` builds. For `train`, synthesize about 180 small records joined to the 211 dataset.
- Identical: `features` JSON, model file bytes, report, and `score` output.
- Normalize: nothing.
- Most likely to differ: `call_key` (`monitor.py:196`) hashes `json.dumps(args, sort_keys=True, default=str)`, so sorted keys and `ensure_ascii` escapes must match to the byte. `cursor_name` depends on dict iteration order (`monitor.py:133`).

**214. grader in Rust: triage names why a run did not complete**
- Ports: `triage.py` whole.
- Depends on: 213 for `Prefix`.
- Inputs: the cases in `test_triage.py`, and the fixture records with a hand-made dataset row for each.
- Identical: `triage.jsonl`, the `table` output, and the `sample` output for seeds `s` and `x`.
- Normalize: `triaged_at`. It keeps microseconds and `+00:00`, unlike every other timestamp (`triage.py:158`).
- Most likely to differ: `random.seed(str)` then `random.sample` (`triage.py:194-195`). Byte equality needs a Mersenne Twister, CPython's SHA-512 string seeding and its `sample` algorithm. See question C.

**215. grader in Rust: the verdict queue and the Jev probe's local half**
- Ports: `baseline.py` lines 279 to 370 with `label-queue` and `label-import`; `jev_probe.py` `build`, `patches` and `score`. `ask` waits on question C.
- Depends on: 210.
- Inputs: synthesized build tickets and CSVs from `test_baseline.py`, a throwaway git repository committed at a fixed time for `build`, and the answer files in `test_jev_probe.py`.
- Identical: the CSV bytes, `ticket-verdicts.json`, the rewritten dataset, the jsonl `build` writes, and the `score` printout.
- Normalize: commit ids, only if the fixed time does not pin them.
- Most likely to differ: the `csv` module writes `\r\n` and quotes minimally (`baseline.py:435`). `jev_probe` writes `ensure_ascii=False` (`jev_probe.py:178`), and `nan` prints through `:.6f` (`jev_probe.py:367`).

**216. grader in Rust: the local judge labels and agrees**
- Ports: `judge.py` whole.
- Depends on: 210, 212, question C for the HTTP client.
- Inputs: the stub server and stub `hydra` in `test_judge.py`; replies that are valid, unparseable, a 404 and a refused connection.
- Identical: `judge-labels.jsonl`, the request body the stub received, stdout, stderr and the exit code.
- Normalize: `judged_at` and the stub's port.
- Most likely to differ: `prompt_sha256` hashes the whole prompt (`judge.py:211`), and the prompt holds `cut()` by code points (`judge.py:79`) and a `sort_keys` dump of `diff_stat` (`judge.py:104`).

**217. grader in Rust: the cutover, and the Python is deleted**
- Changes: the plist runs `hydra-grader once`. `test_launchd.py` and `test_labels_recorded.py` become Rust tests. The README, CI line 45, and `IMPLEMENTATION.md` follow.
- Deletes `integrations/grader/*.py` and its tests, subject to question A's sibling: `apprentice.py:35-36` and `record_to_otlp.py:16` load `records.py` and `redact.py`.
- Depends on: all of the above.
- Gate before deleting: both implementations on a read-only copy of the real `dataset.jsonl` and models, run by the session, since a run cannot read them.
- Most likely to differ: launchd starts `/usr/bin/python3` (plist line 14), which is 3.9.6 here, and the fixtures will come from 3.14.7. The live `scores.jsonl` may hold `p` values neither the fixtures nor the Rust reproduce.

`sheet.py`'s `build`, `budget` and `show` were "left for the grader's own port" (`IMPLEMENTATION.md:3462-3464`). I left them out of these eight. They need a ninth ticket or a decision that they belong to `puppets`.

## The five answers

**1. Numeric parts, and determinism.**
- `fit` is full-batch gradient descent, 3,000 steps, rate 0.5, L2 1.0, no shuffling (`baseline.py:112-126`, constants at `:46-48`). `monitor.py:536` reuses it.
- The split is by launch time read from the UUIDv7, not random (`baseline.py:163-169`, `:51-54`).
- Scaling is at `baseline.py:187-191` and `monitor.py:506-513`. Metrics are at `baseline.py:129-160`.
- The only randomness is `random.seed(seed)` in `triage.py:194`.
- Same interpreter: identical. I trained all three labels on a synthetic 180-row dataset under `PYTHONHASHSEED=1` and `2` with `python3 integrations/grader/baseline.py train`. Model and report matched with `cmp`. `monitor.py features --at 0.5` matched on all 12 fixture records. `random.sample` repeated across processes.
- Across interpreters: not identical. `/usr/bin/python3` 3.9.6 gave `scale` mean `6.059784353227739` against `6.059784353227741` on 3.14.7, with different weights and a different report.
- Not repeatable by design: the timestamps, and `show-cache.json` key order with `--jobs` above 1.
- `python3 -m unittest discover -s integrations/grader/tests` ran 133 tests: 95 pass, 37 fail for the missing binary, and `test_launchd` errors on a broken `pyexpat`, as 196 already recorded.

**2. Disk.**
- Reads:
  - `$HYDRA_ROOT/runs/*.jsonl` (`grader.py:150`)
  - `$HYDRA_ROOT/config/preamble.md` (`records.py:43`)
  - goal logs under the puppets home (`labels.py:193`)
  - `task-classes-*.jsonl` (`labels.py:205`)
  - `integrations/puppets/aliases.json` through `models.py` (`records.py:17`)
  - build tickets (`baseline.py:416`)
  - patch files (`jev_probe.py:200`)
- Writes under the grader home:

| File | Written at |
|---|---|
| `dataset.jsonl`, `cursor.json` | `grader.py:213-214` |
| `show-cache.json` | `grader.py:189` |
| `scores.jsonl`, append | `grader.py:71` |
| `models/*.json` | `baseline.py:427` |
| `models/monitor/*.json` | `monitor.py:596` |
| `ticket-verdicts.json` | `baseline.py:364` |
| `triage.jsonl` | `triage.py:160` |
| `judge-labels.jsonl`, append | `judge.py:223` |
| `launchd.log` | plist lines 34 and 36 |

- Read by another program: `scores.jsonl`, by `hydra-puppets queue` and `board` (`crates/puppets/src/read.rs:442` and `:613`). `apprentice.py:66` keeps its own files under `grader/apprentice`. I found no other reader in `src/` or `integrations/`.

**3. Shell-outs and network.**
- `hydra show <id> --events` (`records.py:126`)
- `hydra show <id>` (`judge.py:92`)
- `hydra-puppets show -- <goal>` (`labels.py:80`)
- `git rev-parse`, `git log`, `git show` (`jev_probe.py:37`, `:96`, `:108`)
- HTTP POST to Ollama on localhost (`judge.py:125`)
- HTTPS POST to `api.typesafe.ai` (`jev_probe.py:226`)

**4. What stays Python.** Map 90 keeps GPU training on PyTorch, which does not exist. No file here is that. The grader is stdlib only, and the only third-party import under `integrations/` is numpy for the Herdr mascot (`integrations/requirements.txt`). `apprentice/README.md:8` says nothing trains on the pairs yet. Every grader file is in scope.

**5. Three questions for a human or a panel.**

- **A. Where does shared code live?** The grader needs `json.rs`, `py.rs` and `models.rs` from `crates/puppets`, and its redactor copies `src/export.rs:576`. Two Python integrations still load `records.py` and `redact.py`.
  - Options for the Rust: depend on `hydra-puppets` as a library; move the shared files to a new `crates/pyjson`; or copy them.
  - Options for the Python: keep the two files until apprentice and otel are ported, move them beside their callers, or port those callers first.
  - I would take `crates/pyjson` and move the two files. Copies of a JSON writer drift.
- **B. Which Python is the reference for the numbers, and how exact?** Options: 3.14.7 byte for byte, with compensated `sum` ported and libm matched; 3.9.6, since launchd scores with it; or a tolerance such as 1e-12 on weights and `p`, with `version` recomputed. A new `version` makes `backfill` score every run again (`grader.py:56`). I would take 3.14.7 byte for byte and fall back to the tolerance only if `exp` or `log1p` cannot be matched.
- **C. Do the two network commands and the seeded sample get ported as they are?**
  - `jev_probe ask` needs TLS, so it is a new dependency. Options: add `ureq` with rustls, drop `ask` as a finished experiment, or keep it as the one Python file left.
  - `judge` needs only plain HTTP to localhost, which a hand-written client over `TcpStream` covers.
  - `triage sample` either ports CPython's Mersenne Twister for byte equality or accepts a different sample per seed.
  - I would drop `ask`, hand-write the localhost client and accept a different sample.
