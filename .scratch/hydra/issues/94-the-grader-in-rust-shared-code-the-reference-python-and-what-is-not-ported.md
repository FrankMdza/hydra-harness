# 94 - The grader in Rust: shared code, the reference Python, and what is not ported

Type: task
Status: resolved
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

Resolved 2026-10-01 by a panel of three under the human's delegation: agy gemini-3.1-pro-high,
cursor-agent grok-4.7-high and claude opus 5.5 (`codex` was `rate_limited`). The brief folded
the question's sub-options into three decisions; each went 2 to 1. Answers in the human's
decision log, `decisiones/2026-10-01/answer-94-*.md`. The plan is
`.scratch/hydra/research/94-the-grader-port-plan.md`.

- **Shared code: a new crate, `crates/pyjson`** (agy 0.85, cursor 0.57; opus chose a library
  dependency on `hydra-puppets` at 0.50 and named this second). The Python-exact JSON, string,
  sort and alias code stays one implementation, and both crates depend on it. Opus's objection
  was that the move cannot be checked now that `puppets.py` is gone; the session checked that
  `crates/puppets` already has a library target and that `json.rs`, `py.rs`, `pysort.rs`,
  `printable.rs` and `models.rs` use nothing else of the crate but one file reader, and the
  recorded fixtures are the check: the extraction must change no fixture comparison. Cursor's
  gap is closed the same way: the alias table moves too. `records.py` and `redact.py` move
  beside `apprentice.py` and `record_to_otlp.py` at the cutover.
- **The numbers match CPython 3.14.7 byte for byte** (cursor 0.62, opus 0.60; agy chose 3.9.6 at
  0.60 so that launchd's configuration would not change, which it will at the cutover in any
  case). The compensated `sum` is ported, and `exp`, `log` and `log1p` are probed first: opus's
  condition is one million sampled inputs with not one bit of difference between Rust and
  CPython 3.14.7 on this machine, or this decision falls to the tolerance. The launchd job moves
  to that Python and the models are retrained once with it before the cutover's gate; that step
  is the human's, since it changes his machine.
- **Not ported as it is** (cursor 0.70, opus 0.55; agy chose to leave `sheet.py` in Python at
  0.90): `jev_probe ask` is dropped and its results stay recorded, so no TLS dependency; `judge`
  gets a hand-written client for plain HTTP to localhost; `triage sample` gives a different but
  reproducible sample for a seed (the session checked that the one open sample is stored as a
  file, `triage-sample-163.txt`, which was opus's condition); `sheet.py`'s `build`, `budget` and
  `show` are ported into `hydra-grader` as a ticket of their own.

What reverses each: the extraction changing one fixture comparison, or taking more than two
review rounds (then a library dependency on `hydra-puppets`); one differing bit in the `exp`,
`log`, `log1p` probe (then the tolerance); a stored triage sample that must be drawn again from
its seed (then CPython's Mersenne Twister is ported).

Carried by builds 210 to 220.

