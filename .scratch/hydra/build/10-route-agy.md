# 10 - Route `agy`

Type: build
Status: open
Blocked by: 03, 08

## Goal

The third provider: `-p=` with the equals sign, `--add-dir` carrying three separate
things, the hook config that fails closed, and its rows in the normalizer.

## Where the design says it

- Harness spec 6.7's `agy` block, including why a bare `-p` silently drops the prompt
  and why `--dangerously-skip-permissions` is a launch decision rather than a run-time
  one.
- 6.4, 6.5 and 6.6 for the three things `--add-dir` carries, without which `agy` reports
  `hooks: []` rather than an error.
- 6.5 for the inherited-server residue this provider keeps, and why Hydra never calls
  `agy mcp disable`.
- 6.6 for `overwrite`, which is forbidden, and for the first-deny short circuit.
- Contract 7.3 for `text_delta`, which is a buffered flush wearing a streaming name, and
  9.2 for `result` usage being per conversation rather than per run.

## Done when

- A recorded `agy` run writes a record that passes the contract checker.
- The normalizer concatenates `text_delta` over `step_index` to produce one `message`,
  reproducibly from `raw`.
- `usage` comes off the step's `DONE` frames, not off `result`, on a resumed run.
- The three unverified facts in harness spec 19.3 are either measured or still carried
  as caveats, and the ticket says which.

## Watch for

**`agy` writes where it likes.** With `--dangerously-skip-permissions` a probe that wrote
`probe.txt` put it in `~/.gemini/antigravity-cli/scratch/`, not the launch directory.

**It has moved four times in four days.** Re-read the version and re-measure rather than
inherit. Ticket 26 probed the hook gate on 1.1.24 and ticket 30 re-measured on 1.1.25.

**A permission denial and a file-not-found get the same `state: ERROR` and the same
`error.type: TOOL_ERROR`, and both messages contain the word "permission".** The adapter
returns `unclassified` here rather than guessing. Guessing produces a curator that
rewrites a skill because a file was missing.
