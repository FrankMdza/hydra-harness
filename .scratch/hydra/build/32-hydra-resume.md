# 32 - `hydra resume <run-id>`

Type: build
Status: resolved
Blocked by: 31

## Goal

Harness spec 17's ninth command. A finished or abandoned run has the provider's own session id
on its record, and each of the three binaries can continue a conversation by that id. Build
the command that reads it and launches the continuation as a run of its own.

## Where the design says it

- `docs/harness-spec.md` section 5, "Session identity", and the `hydra resume` row of section 17.
- `docs/run-record-contract.md` section 5.1: `command` takes the value `resume`, and the new
  row `resumed_from`, added for this ticket, names the run the session came from.
- `src/run.rs` for the launch a recorded run does; `src/record/lines.rs` for `provider_session_id`
  and how each provider backfills it; `src/provider/*.rs` for each argv.
- The map's fog, "What a resumed run's usage is scoped to", for the one thing this does not settle.

## The slices

**The command.** `hydra resume <run-id> [--task "..."] [--timeout <dur>]` in `src/cli.rs` and
`src/main.rs`, implemented in `src/resume.rs`. It reads the old record, refuses when that run is
still live, when it has no `provider_session_id`, when it was interactive, or when its provider
has no resume concept, each with one sentence naming the reason. Otherwise it mints a fresh run
id and launches through the same pre-flight, worktree seeding, gate and spawn a `hydra run` does,
with the old run's provider, model, policy and cwd, and `--task` defaulting to a one-line
continuation prompt. `run_start` carries `command: "resume"` and `resumed_from: "<old-run-id>"`.

**The provider flags, measured on this machine on 2026-09-08.** `claude` takes `--resume
<session-id>`; `cursor-agent` takes `--resume <chatId>`; `agy` takes `--conversation <id>`. Each
provider module gains one function that returns its resume arguments, and the launch appends them
to the ordinary argv. Where the provider's own session id has a different shape from what the
record holds, say so in the outcome rather than guessing.

**Usage.** Take counts the way an ordinary run does. Contract 9's note says `agy` reports the whole
conversation on `result`, and that is the fog: leave a `Finding::Limit` on a resumed `agy` run
saying its `result` usage covers earlier turns, and change nothing else.

**Tests.** `tests/resume.rs`: from a synthetic closed record with a session id, the argv each
provider would get carries the right flag and id, `run_start` carries `command: resume` and
`resumed_from`, and a record with no session id, a live record and an interactive record are each
refused with their sentence. Nothing launches; build the argv the way `tests/spawn.rs` does.

## Done when

- `hydra resume <old-id>` on this machine launches a `claude` haiku continuation whose record
  says `command: resume` and `resumed_from`, and the old record is untouched.
- The tests above pass and the rest of the suite is untouched.
- The orchestrating session adds the row to contract 5.1 and the command to `IMPLEMENTATION.md`.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Composer 2.5, run 01a0801d-7382 under `standard`:
6.58M tokens, twenty files, eight tests, fifty minutes, in a Herdr pane the bridge labelled.
Every slice landed: `resume_in`, the four refusals, `resumed_from` and `command` on
`run_start`, one `resume_args` per provider, the `agy` usage limit. The model's stated
assumptions: the default task wording is its own; session ids pass through as recorded, so
whether Cursor's recorded id is the `chatId` its flag wants is unverified; `provider` is read
off the envelope because line 0's body has no such field.

**What the run did that the gate could not see.** Its first tests called `resume()` through
`Layout::discover()`, which found the real `~/hydra`, and three spawn directories appeared
there with a compiled gate and a `launch.json` naming `claude --resume <fake-id>`; the seeded
cwd was a reaped worktree, so nothing launched and no record was written, and `hydra reap`
swept them. The orchestrating session changed one test that still read the real layout for
its lineage, and wrote the finding into `IMPLEMENTATION.md`'s rough edges: a run's test
suite reaches the harness through `shell`, and only a test author's layout argument stands
between a worktree and the corpus. The run also used two Cursor tools the table does not
know, `getMcpToolsToolCall` and `awaitToolCall`, both run without the hook; map ticket 51
now covers the class. Not measured on a live launch: the done-when's haiku continuation
waits on a session Frank chooses to continue.
