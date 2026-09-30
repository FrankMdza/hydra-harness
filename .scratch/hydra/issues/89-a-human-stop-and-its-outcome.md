# 89 - A human stop, and the outcome it leaves

Type: task
Status: resolved
Blocked by:
Opened: 2026-09-30, by the session, from build ticket 169, at the human's request for a stop
command. The panel of 2026-09-30 chose `hydra stop <run>`, `--all` and `--session`, typed by a
human, never a watchdog (D10).

## Question

Build 169 checked the contract before any code and stopped. A stop can be written into
`run_end.error` (contract 6 already carries the watcher's `SIGNAL_NOTE` there), but no outcome of
contract 8's nine means "a human ended this run":

- `crashed` is what the code derives today: SIGTERM kills the child, the watcher writes
  `SIGNAL_NOTE`, `src/outcome.rs:229` reads it as `Crashed`. The panel ruled it out.
- `failed` is what a stop note not starting with `SIGNAL_NOTE` would derive today
  (`src/outcome.rs:235`). Contract 8 defines it as "the task ran and failed", and rung one learns
  from it (`src/rungs/corpus.rs:327`). Contract 8.6 already rejects that reading for an
  interrupted `hydra evolve` pass, as "a failure signal about nothing".
- `timed_out` means the child may still be running; `derive` never returns it.

Build 169's smallest proposal: a tenth outcome, `stopped`, derived from a fixed stop-note prefix
in `run_end.error` read before `SIGNAL_NOTE`, `outcome_source: derived`, no new field, `refusal`
untouched. The open part is rung one's column: does rung one learn from a stopped run?

Options:

- A. `stopped`, and rung one never learns from it (as 8.6 treats an interrupted pass).
- B. `stopped`, and rung one learns from it as a correction: a human judged the run was going
  wrong.
- C. No new outcome; a stop derives `failed`, and rung one's filter skips runs whose
  `run_end.error` starts with the stop note.
- D. No stop command; `hydra reap` and the provider's own Ctrl-C stay the only ways.

## Answer

Resolved 2026-09-30 by a panel of three under the human's delegation (he asked that day for the
panel on hard questions). Unanimous: **A**. codex 0.91, cursor-agent grok-4.7-high 0.71, agy
gemini-3.1-pro-high 0.85; all three named C second. Answers in the human's decision log,
`decisiones/2026-09-30/answer-89-*.md`.

A tenth outcome, `stopped`: a human ended the run with `hydra stop`. The watcher writes a fixed
stop note at the start of `run_end.error`, the derivation reads that prefix before
`SIGNAL_NOTE`, `outcome_source` stays `derived`, no new field, `refusal` untouched. Rung one never
learns from it: the record cannot tell a stop for a bad run from a stop for quota or a changed
mind, and 8.6 already keeps "a failure signal about nothing" out of the corpus. If the exclusion
proves wrong, the records still say `stopped` and can be admitted later; a false `failed` could not.

All three seats named one reversal: a reader that breaks on a tenth outcome. The session counted
them the same day. In `src/`, 19 uses of `Outcome::` outside `outcome.rs` sit in exhaustive
matches, so the compiler lists every one to update. In `integrations/`, every reader compares
strings, and none raises on an unknown one; `baseline.py`'s `outcome` label would count a stop as
not completed and its `task` label would leave it out, which build 169 should keep in mind.

Carried by build 169. Amends contract 8 (one row), harness spec 12's list, and `outcome.rs`.
