# 128 - An offline audit of past delegations against a written rubric, scored by Kev and checked against the recorded verdicts

Type: research
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 117's answer, question 1. Research: the measurement
that decides whether a Jev tier earns an advisory line. It needs build 294 (Kev beside System 1,
the other session's ticket) in master before the scoring half can run; the rubric and the corpus
can be written before.

## Question

Map 117 decided that no classifier refuses a launch, and that a delegation score is declared
unvalidated until an offline audit shows it tracks review rejections. This is that audit.

1. The rubric, written first and by hand: for a task text as `worker_task_text` renders it plus
   the goal text, what counts as required behaviour and acceptance (good), and what counts as
   implementation prescribed when it need not be (the thing the middle tier was meant to catch).
   Two or three examples of each from the goal logs on disk.
2. The corpus: every task text handed to a builder in the goal logs under `~/hydra/puppets/`,
   joined to the task's verdicts (first-pass acceptance, `fixed`, rejections) by build 301's
   walk. Count it.
3. The score: Kev (build 294) reads each task text against the rubric and emits a number; the
   session also scores a blind sample by hand. Report the agreement between Kev and the hand
   scores, and whether either tracks the rejections. The 117 readings predicted the opposite of
   the tier's premise for small builders (more detail handed down did better), so report the
   direction of the correlation, not only its strength.

The answer feeds map 117's advisory line and nothing else; no refusal is built from it.

## Findings
