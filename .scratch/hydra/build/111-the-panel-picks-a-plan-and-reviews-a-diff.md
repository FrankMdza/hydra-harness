# 111 - The panel picks a plan, and its diff findings land in Hunk

Type: build
Status: resolved
Blocked by: 108

## What

Amended 2026-09-24 by the human: the human's diff review is Hunk, not anything the mode builds.
`puppets.py review <goal> <task>` already opens `hunk diff` in the run's worktree or `hunk show
hydra/<run-id>` after a reap (build 108's review added it). What this ticket adds on the diff side
is only the judges' findings pushed into that Hunk session as comments, through `hunk session
comment apply --stdin`, which map ticket 45 measured (`filePath`, `summary`, `hunkNumber`).

The two judged steps of Master of Puppets that ticket 108 stubs: `choose` records a verdict with
an empty judge list, and there is no review of a task's diff. Both become `judge`-policy runs, one
per seat, each answering one JSON object in the shape `parse_judge_answer` in
`src/rungs/review.rs` already reads, so a refusal or prose is never read as a pass. The plan-picking
prompt takes the three proposals in the prompt and asks for `{"chosen": "<provider>|composed",
"findings": [...]}` plus, when composed, the composed plan's task block. The diff checklist takes
the task text, the base tree, the diff and the run's projection, and asks for the rung-two shape
with a general checklist of at most twelve items written for task diffs rather than for Hydra's
own source. `puppets.py choose --judge` and `puppets.py review <task>` launch the seats, wait,
parse, and record `panel_verdict` and `review`.

Two facts the seats must carry: a `codex` judge under the `judge` policy is gated on its first
read, because `codex` reads only through the shell (map ticket 63), so `codex` sits on this panel
under `standard` until 63 is built, and the log records the policy each seat ran under. And the
panel never substitutes or shrinks: a missing or unparseable seat blocks the verdict, as
`docs/rungs-spec.md` 4.6 has it, and the human is told which seat is missing.

## Done when

`integrations/puppets/tests/test_puppets.py` gains cases with stub judge output: three parsed
verdicts produce a `panel_verdict`; one refusal blocks it and names the seat; a `composed` answer
imports its task block. The prompts live in `integrations/puppets/prompts/`. One real `choose
--judge` on this machine over ticket 75's four proposals, seats `claude` opus, `codex` (standard)
and `cursor-agent` composer-2.5, with the run ids in the outcome.

## Outcome

Built 2026-09-24 by composer-2.5 on run 01a0d41e-5b15 (2.63M tokens), after a first attempt on
`codex` (01a0d41b-f171) died at launch on its usage limit until 12:41 PM. `choose --judge` and a
judged `review`: three seats (`claude` opus under `judge`, `codex` under `standard` because of
map ticket 63, composer-2.5 under `judge`), parsers with a severity ceiling per checklist item so
a judge cannot escalate an advisory item, a plan verdict that needs every seat parsed and
unanimous, a `composed` answer imported as tasks, and major or critical diff findings pushed
into the Hunk session as comments before Hunk opens. Prompts in `prompts/choose.md` and
`prompts/diff.md`. 73 tests green on master; the session replaced one em dash in a reason
string. Not done from the ticket: the one real `choose --judge` over ticket 75's proposals,
because `codex` is out of credits until 12:41 and the panel never shrinks; the human runs it
from the session after that. The fourth panel's ticket 118 names what this file still gets
wrong (Hunk and `git diff` run in a run-writable checkout as the human, the copy out of a
worktree, the `plan` approval checked nowhere, no lock on the log) and is the next ticket on it.
