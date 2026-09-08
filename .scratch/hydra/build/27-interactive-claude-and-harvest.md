# 27 - `hydra run --interactive` on `claude`, and the harvest

Type: build
Status: claimed
Blocked by:

## Goal

Harness spec 3's second launch mode. The vendor owns the terminal and draws its own interface,
Hydra launches it and gets out of the way, and the run is recorded thin from the vendor's own
transcript when the process exits. `claude` first, because it is the one provider confirmed to
write an append-only transcript during the run, and because it is the one whose transcript
path Hydra can predict: the pre-flight probe already mints a `--session-id` and reads the file
that names. This is what lets Herdr be the panel for a session a human steers, and it is the
only producer of contract 7.9's `correction`.

## Where the design says it

- Harness spec 3, both modes; 4, the second population; 10, "Interactive runs are harvested at
  exit", "Harvest is idempotent and any process can do it", and "The scenario, worked".
- Contract 3 on a harvested line wrapping a transcript record; 5.6, the interactive
  field-presence table; 7.2.1, what changes on a harvested run; 7.9, `correction`.
- Map ticket 38's `## Answer` for what was measured on two real transcripts, and ticket 34 and
  35 for the correction shape. `src/provider/claude.rs::resolve_in` for the session id and the
  transcript path.

## The slices

**The launch.** `hydra run --interactive --provider claude [--task ...]` mints a run id and a
`claude` session id, writes `run_start` with `launch_mode: interactive`, `policy` and
`policy_sha` null, `worktree` null, no `skills`, and the provider session id on the envelope,
and then spawns `claude --session-id <id>` on the caller's own terminal, in the caller's cwd,
inheriting stdio, with the child environment 6.2 gives every provider. No pty of Hydra's, no
detached half, no gate, no preamble; `--mcp-config` without the strict flag as 6.5 says. The
task, if given, is passed the way `claude` takes an opening prompt interactively. Hydra waits
for the process to exit.

**The harvest.** On exit, and again from `hydra status` for a run whose pid is gone and whose
record has no `run_end`: read the transcript at the path `claude::transcript_path` predicts for
that session, wrap every record of it as one line of the run file in order, verbatim under
`raw`, and write `run_end` with `body_source: transcript` and the exit code. Idempotent: a
record that already carries `run_end` is left alone, and two harvesters cannot both write,
which contract 6 asks of any writer.

**The reader.** `normalize` reads a harvested `claude` record by contract 7.2.1: `message`,
`tool_call` and `tool_result` from the transcript's assistant and user records, `usage` from
`message.usage`, `thinking` when a block carries text, `correction` from the rejection shape
ticket 35 measured. No `permission_decision`, no gate-leak check. `hydra show --events` then
prints it. `outcome` derives from the transcript's last turn the way 8 says, and the eight
transcript record types no stream has stay verbatim and normalize to nothing.

**Tests.** `tests/harvest.rs`, new: build a transcript file by hand in a scratch `HOME` in the
shape ticket 38 measured, run the harvest into a record, and check the record against the
contract and through `normalize`. One test with a rejected `Edit` producing a `correction`.
One test that a second harvest of the same run writes nothing. `tests/status.rs`: a dead
interactive run with no `run_end` is harvested by `hydra status`. The launch itself is checked
by the orchestrating session in a Herdr pane, since it needs a terminal.

## Done when

- `hydra run --interactive --provider claude` opens the vendor's interface in the current pane
  and leaves a record with `run_end.body_source: transcript` when it exits.
- The tests above pass and the suite is untouched.
- `IMPLEMENTATION.md` gains a section on the interactive mode saying what is harvested, what
  is not, and that `cursor-agent` and `agy` are not launched interactively yet.
