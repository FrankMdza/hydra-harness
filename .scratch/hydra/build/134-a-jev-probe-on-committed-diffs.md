# 134 - A Jev probe on committed build diffs, scored against their Outcomes

Type: build
Status: resolved
Blocked by:

## What

Map ticket 81-b, resolved 2026-09-26. Read its `## Answer` and research/81 sections 1 and 2 first.

Jev (Typesafe AI) answers typed questions: `POST https://api.typesafe.ai/v1/systemone` with a
`state`, a `model` (`jev-latest`) and a `questions` map; a `noul` question returns a probability
of yes. 64k context, 32k for `state` plus the longest question. The probe asks it about diffs
already committed to master and scores it against what the session actually did, which every
build ticket's `## Outcome` records: the diff was applied as written, or the session changed it.

## Do

1. `integrations/puppets/jev_probe.py`, stdlib only. `build <out.jsonl>` walks
   `.scratch/hydra/build/*.md` from 112 on, reads each resolved ticket's `## Outcome`, labels it
   `changed` when the Outcome says the session changed, fixed or added something to the diff, and
   `as_written` when it says the diff was applied as written. Anything else, or no run named, is
   skipped with a reason. It finds the commit whose message starts `Build NNN:` and takes its diff
   with `git show`, limited to the files that commit touched under `src/`, `tests/` and
   `integrations/`. Print the label counts and every skip.
2. `ask <in.jsonl> <out.jsonl>` sends each diff as `state` with four `noul` questions: the diff
   needs a correction before it is applied; it states a number it could not have measured; it
   changes behaviour the task did not ask for; it can lose data that arrives in pieces. The key
   comes from `TYPESAFE_API_KEY` in the environment and is never written anywhere. A diff over
   the context limit is skipped and counted. Answers go to the output file with the model id.
3. `score <out.jsonl>` prints, for the first question against the label: Brier score, a
   reliability table in five bins, and accuracy at 0.5. Say in the output that thirty-odd points
   make every one of these numbers loose.
4. Nothing under `src/` changes, nothing reads `~/hydra/runs/`, and the script sends nothing but
   committed diffs. Put that sentence at the top of the file.

A run launched by Hydra cannot reach the API: the environment allowlist drops
`TYPESAFE_API_KEY`, which is the design working. So `ask` is the session's to run after the
human puts the key in its environment. Build `build` and `score` and test them; test `ask`
against a stub server.

## Done when

- A test with a scratch git repo and two fake tickets builds one `changed` and one `as_written`
  row with the right diffs, and skips a third ticket with no Outcome.
- `ask` against a local stub HTTP server sends the documented request shape and writes the
  answers; `score` on a fixed answers file prints the expected Brier score.
- The puppets suite passes.

## Outcome

Resolved 2026-09-26. The script was built by `composer-2.5` on run 01a0ded9-c9a6 under
`standard`, about 0.90M tokens. The session changed its input, added a ranking score, and ran the
probe with the human's key.

What the agent built: `integrations/puppets/jev_probe.py`, stdlib only, with `build`, `ask` and
`score`, and three tests (a scratch git repo for `build`, a stub HTTP server for `ask`, a fixed
file for `score`). `ask` sends `state`, `model: jev-latest` and four `noul` questions, using the
API's `instructions` field, and takes the key from `TYPESAFE_API_KEY`.

What the session changed. First, the ticket was wrong about the input: the `Build NNN:` commit
is the diff after the session corrected it, so "does this need a correction" has no true answer
there. With the human's approval (map 81-b follow-up), the probe used the agents' own diffs as they
came back, 25 kept in the session's handoff folders for builds 89 to 132. The session labelled each
by hand from its `## Outcome`: 21 corrected or flawed, 4 applied as written (113, 114, 116, 127).
126 counts as flawed, because it went in as written and lost 105 comment lines that build 127 put
back. A new `patches <labels.json> <out.jsonl>` command reads those. Second, `score` printed only
calibration, and the result made clear that ranking is the other half, so it now prints
`rank_auc`, and `base_rate_yes` with the Brier score of always answering the base rate. Both have
tests; the suite passes, 123 tests. The key lives in the human's `~/hydra/.env`, now ignored by git
and so never seeded into a worktree. The session's wrapper read it without printing it.

Result, `jev-1.13.0`, 25 diffs, about 130k input tokens:

| Measure | Value | For comparison |
|---|---|---|
| Brier, `needs_correction` | 0.332 | 0.134 for always answering the base rate |
| Accuracy at 0.5 | 0.24 | 0.84 for always answering "yes" |
| Rank AUC | 0.964 | 0.5 is chance |

Jev never went above 0.75 on `needs_correction`, and 17 of 25 answers fell between 0.2 and 0.4,
so as a probability it is badly calibrated here and a 0.5 threshold is useless. As a ranking it did
well: the four clean diffs scored 0.20, 0.22, 0.26 and 0.28, and only build 117 (0.22) among the
corrected ones scored as low. The narrow questions matched the defects the session found:
"can it lose data that arrives in pieces" scored highest on 131 (0.49), the refusal lost across two
polls, and "does it state a number it could not have measured" scored highest on 132 (0.45), the
invented windows. With four negatives, the AUC is loose, and this is one run on one day.

What it supports, for the human to decide: Jev as an ordering signal (which diffs to read first,
or when to add an adversary seat), never as a threshold and never as a vote, and a temperature
fitted on these labels before any probability is read as one. The answers are in
`~/.claude/handoffs/2026-09-25-jev/answers.jsonl`.
