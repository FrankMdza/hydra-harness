# 38 - A Cursor run may read its own `cursor-data/`, so it can await a background task

Type: build
Status: resolved
Blocked by: 35

## Goal

Run 01a08111-1e5e, Composer 2.5 under `standard` on 2026-09-08, started `sleep 8 && echo
slept` as a background task and called `awaitToolCall` on it. Cursor answers that call by
reading the task's output file, `<spawn>/cursor-data/projects/<slug>/terminals/<taskId>.txt`,
and fires the hook on that read; the gate refused it `hydra/out-of-scope`, because build
ticket 14 denied `~/hydra/spawn/` whole. The model then tried `Read` on the same file and was
refused again, and reported the task done by guessing. The terminal file is the run's own
output, written by the run's own provider under the run's own spawn directory, and under
`standard` a run may read what it wrote. Carve it out for reads.

## Where the design says it

- Map ticket 40's decision 5 and build ticket 14: the deny list is two kinds of entry, denied
  parents and own-path carve-outs, and the spawn directory is a denied parent because a
  `permissive` run could rewrite a sibling's gate. A read of the run's own provider data
  touches neither reason.
- Map ticket 51's answer and build ticket 35: `awaitToolCall` was placed among the `unhooked`
  rows on the evidence of one run; three later runs show the hook firing on the terminal file.
- `src/policy/mod.rs`, where the compiled config's denied parents and `own_write_paths` are
  built; `src/policy/gate.py`, where they are read; `src/provider/cursor.rs` for where
  `CURSOR_DATA_DIR` points.
- `IMPLEMENTATION.md`, the Cursor section, the paragraph on the three unhooked tools.

## The slices

**The carve-out.** The compiled config gains `own_read_paths`, beside `own_write_paths`, and
on `cursor-agent` it carries the run's own `cursor-data/` directory. `gate.py` allows a
`file_read` whose resolved path is under an entry of that list under every policy that grants
`file_read` at all, before the denied-parent check, the way the own-write carve-outs already
work. No other provider adds an entry. Nothing under `policies/` changes; if the carve-out
turns out to need a line in a policy file, stop there and say so.

**The row.** `awaitToolCall` stays an `unhooked` row for the replay, since the record carries
no path for it and the hook's own answer, when it fires, stands as a `Disagreement` if the
two differ. Its doc comment says what the hook actually sees.

**Tests.** `tests/gate.rs`: a Cursor gate under `standard` allows a `beforeReadFile` and a
`Read` of a file under the run's own `cursor-data/` and still refuses one under a sibling
run's spawn directory and one under `~/hydra/spawn/` itself; the same under `read-only`;
`judge` still refuses everything. `tests/spawn.rs`: the compiled config for a `cursor-agent`
launch carries the entry and a `claude` launch carries none.

## Done when

- A `cursor-agent` run under `standard` that starts a background task and awaits it gets the
  task's output back, measured on a real launch.
- The tests above pass and the rest of the suite is untouched.
- The orchestrating session rewrites the `awaitToolCall` sentence in `IMPLEMENTATION.md` and
  appends the finding to map ticket 51.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Cursor Grok 4.6 high fast, run 01a08112-fe30 under
`standard`, launched through `hydra-in-pane`: 2.89M tokens, five files, three tests, twenty
minutes. `own_read_paths` in the compiled config, one entry on `cursor-agent`, read by `gate.py`
before the denied-parent check and only when the policy grants `file_read`; no policy-file line
was needed, which the ticket had left open. The model could not do the done-when's real launch
from inside the gate and said so. The orchestrating session did it: run 01a0811e-d401, Composer
under `standard`, started `sleep 8 && echo slept` in the background, awaited it, and reported
`slept` with exit 0, the await result a success and no finding on the record. The same probe
before the carve-out, 01a08111-1e5e, had been refused and guessed.
