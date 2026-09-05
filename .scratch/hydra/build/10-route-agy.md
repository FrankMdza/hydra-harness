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

## What ticket 03 left you, added 2026-09-05

**The seam is `normalize::read_with(text, policies, layout)`, and it dispatches on
nothing today.** `src/normalize/mod.rs` holds the ten types and calls
`claude::normalize` directly, because one provider is routed. Your column needs an
`agy.rs` beside it and a branch on the envelope's `provider`, which every line carries
(contract 4).

**Two fields on the types exist for you.** `Message::block` is documented as the
`step_index` your concatenation runs over, and `Counts::cache_write_tokens` and
`Counts::thinking_tokens` are `Option` because your provider reports one and not the
other. `Counts::total_tokens()` deliberately does not match your own `total_tokens`, and
the reason is in its doc comment: yours excludes cache read, so copying it would make a
warm-cache run look cheap.

**`usage` is one event per run and you have per-step counts.** 9.2 says a run's usage is
the sum of its own `step_update` frames and that `result.usage` is recorded and not used
for it. The `claude` adapter emits its single `Usage` event when it reaches the `result`
frame; yours has to sum first and emit once, and the `seq` field on `Usage` is an
`Option<u64>` that will not name a single frame honestly. Say so in the outcome rather
than pointing it at the last step.

**`ResultStatus::Unclassified` is written and reached.** 7.6's warning about your
`TOOL_ERROR` is already the code's behaviour, and the `claude` adapter reaches the same
value from a different direction: a successful `Write` with no `is_error` field.
