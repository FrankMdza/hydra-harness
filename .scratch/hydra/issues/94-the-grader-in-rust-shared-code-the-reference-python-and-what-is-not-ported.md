# 94 - The grader in Rust: shared code, the reference Python, and what is not ported

Type: task
Status: open
Blocked by:
Opened: 2026-10-01, by the session, from a read-only plan of the grader's port (claude opus 5.5,
run `01a0f6b6-a5ce`, kept in the human's `builds/plan-grader.md`). The plan is eight build
tickets and three questions it could not answer from the repository.

## Question

Map 90 puts `integrations/grader/` next after `puppets`: a crate `crates/grader`, a binary
`hydra-grader`, proved against the Python before the Python goes. The grader is 2,867 lines of
stdlib Python: a record scan and a redactor, a logistic regression trained by full-batch gradient
descent, a dataset collector run from launchd, a trace-prefix monitor, a triage table, a verdict
queue, a local judge over HTTP, and a probe that posts to an outside API.

**A. Where shared code lives.** The grader needs what `crates/puppets` already has: `json.rs`
(Python's `json` to the byte), `py.rs`, `pysort.rs`, `models.rs`. And two other Python
integrations, `apprentice.py` and `record_to_otlp.py`, load the grader's `records.py` and
`redact.py`, so those two files cannot be deleted with the rest.

- A1. `hydra-grader` depends on `hydra-puppets` as a library.
- A2. The shared files move to a new crate, `crates/pyjson`, that both depend on.
- A3. Copy them into `crates/grader`.

And for the two Python files: (i) keep them until `apprentice` and `otel` are ported, (ii) move
them beside their remaining callers, (iii) port those callers first.

**B. Which Python the numbers must match, and how exactly.** The same training data gives
different weights under CPython 3.9.6 and 3.14.7 (`scale` mean `6.059784353227739` against
`...741`), because `sum()` over floats is compensated since 3.12. The launchd template runs
`/usr/bin/python3`, which is 3.9.6 on this machine; the session and the differential fixtures use
3.14.7. A model's `version` is a hash of its weights, so one bit off renames the model and makes
`backfill` score every run again.

- B1. 3.14.7, byte for byte, porting the compensated `sum` and matching `exp`, `log` and `log1p`.
- B2. 3.9.6, byte for byte, since that is what launchd runs today.
- B3. A tolerance (1e-12 on weights and on `p`), with `version` recomputed by the Rust; the models
  on disk are retrained once at the cutover.

**C. What is not ported as it is.**

- `jev_probe.py ask` posts to `api.typesafe.ai` over HTTPS, which needs a TLS dependency.
  C1 add one (`ureq` with rustls); C2 drop `ask` as a finished experiment; C3 keep `jev_probe.py`
  as the one Python file left.
- `triage.py sample` draws with `random.seed(str)` and `random.sample`. Byte equality needs
  CPython's Mersenne Twister, its string seeding and its `sample`. C4 port them; C5 accept a
  different sample for the same seed.
- `sheet.py`'s `build`, `budget` and `show` (map 93 left them in Python until this port). C6 a
  ninth ticket ports them into `hydra-puppets`; C7 into `hydra-grader`; C8 they stay in Python.

## Answer
