# 89 - A human stop, and the outcome it leaves

Type: task
Status: claimed
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

(pending: panel of three and a judge, under the human's delegation; the human can revert)
