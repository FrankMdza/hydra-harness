# 18 - `hydra show <run-id> [--events]`

Type: build
Status: resolved
Blocked by:

## Goal

Harness spec 17 lists `hydra show <run-id> [--events]` as "the full record a delegation result
deliberately omits", and spec 9 says a parent that wants more than contract 17's nine fields asks
for it here, explicitly. Nothing prints a normalized record today: `hydra logs` prints raw lines
and `hydra status <id>` prints liveness plus the projection. The graph view of a run, the Herdr
bridge and any orchestrator reading a child all want the same thing, the events in order with the
projection beside them, as JSON on stdout.

## Where the design says it

- Harness spec 9, "The result stays small", and spec 17's table row.
- Contract 7, the ten event types, and contract 3 on a file that may still be growing.
- `src/normalize/mod.rs` is the reader; `src/status.rs::one` already builds the projection.

## The slices

**`src/show.rs`.** `pub fn show(layout: &Layout, run_id: &str, events: bool, out: &mut dyn Write)
-> Result<()>`. Without `--events` it prints the same projection object `hydra status <id>` prints
under `result`, alone, pretty JSON. With `--events` it prints one object with `run_id`,
`projection`, `events` and `findings`. Each event is an object with `seq`, `type` from
`Event::type_name`, and that variant's fields by name; `thinking` carries `signed: bool` rather
than the signature bytes; `tool_result.output` is verbatim and untruncated, because contract 7.6
says the truncation marker is the reader's to see. `run_start` and `run_end` carry the body the
record holds. Findings print with their type name and fields. A record with no `run_end` shows
what it has.

**The CLI.** `Show { run_id, events }` in `src/cli.rs` beside `Logs`, dispatched in `src/main.rs`.
Doc string on the variant in the voice the others use.

**The test.** `tests/show.rs`, built the way `tests/normalize.rs` builds a synthetic record by
hand: one `claude` run with a message, a thinking block, a tool call, its result and a gate
denial. Assert the event count and order, the `type` names, that `--events` off prints only the
projection, and that a record cut before `run_end` still prints.

## Done when

- `hydra show <id>` and `hydra show <id> --events` print on a real record in `runs/`.
- `tests/show.rs` passes and the rest of the suite is untouched.
- `IMPLEMENTATION.md`'s "`hydra show` is still not built" paragraph says built.

## Outcome

Resolved 2026-09-07. Written by `claude` on Sonnet 5, run 01a07efe-97f4 under `standard`, after
`cursor-agent` refused the same task twice on a monthly limit that covered Opus 5 and then `auto`
too. Sonnet read eleven files before writing, refactored `status::one` into `build` and
`projection` so `show` takes the object rather than re-parsing a string, and wrote `src/show.rs`
at 190 lines with `tests/show.rs` at three tests: 7.35M tokens, 7.1M of them cache reads, 59k
output, 44k thinking. Applied to master by hand from the worktree diff, unchanged.

**The shape it prints.** `run_start` and `run_end` carry the record's body under `body`, so a
reader gets `policy`, `launch_mode`, `depth`, `scopes` and the rest as written and the provider
from `projection`. `thinking` carries `signed` and not the bytes. `usage.counts` carries the five
counts and no total. `hydra-graph` in `integrations/herdr/` reads exactly this and was adjusted
to it in build ticket 20.

**Two things the run found.** Its own `ls` on a path under `~/hydra/runs/` came back as a
`hydra/protected-path` denial, which is harness spec 7.5 working, and it reported that as odd
because CLAUDE.md told it nothing here is enforced. That paragraph is written for a human's
agent editing the repo, and a recorded run in a worktree reads the same file. CLAUDE.md now
says which population is reading. And its thinking blocks were all empty, 35 of 35, with the
build ticket 17 key in force: Sonnet 5 emits no summary text where Haiku 4.5 does, which is
the per-model caveat 17 named, now measured.
