# 134 - A Jev probe on committed build diffs, scored against their Outcomes

Type: build
Status: claimed
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
