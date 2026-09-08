# 28 - `hydra run --interactive --policy <name>` on `claude`

Type: build
Status: resolved
Blocked by: 27

## Goal

Map ticket 50 measured the gate applying to an interactive `claude` session through
`--settings`, and the human decided the launch takes `--policy` as an option with none by
default. Build it on top of ticket 27's interactive launch and harvest.

## Where the design says it

- Map ticket 50's `## Answer` and the decision beneath it. Harness spec 3's new paragraph.
- Build ticket 27 for the launch and the harvest it extends. `src/run.rs` for how a recorded
  launch compiles the gate, seeds the worktree and links the snapshot, which this reuses.

## The slices

**The launch.** With `--policy`, the interactive launch does what a recorded one does before the
spawn: pre-flight, worktree seeded from the parent's uncommitted work, snapshot linked, gate
compiled into the spawn directory, `settings.json` written, and `claude` started interactively
in the worktree with `--settings` on that file, `--mcp-config` as 6.5 says for the mode. The
record's `run_start` carries `policy`, `policy_sha`, `scopes`, `worktree` with `base_tree`, and
`skills`, and `launch_mode` stays `interactive`.

**The harvest reader.** On a harvested record whose `policy` is not null, `normalize` derives a
`permission_decision` of `deny` by `hydra_policy` from every `tool_result` whose text starts
with `hydra/`, joined by `tool_use_id`, with the reason verbatim. It pushes one
`Finding::Limit` per such run saying allows were not recorded on this launch mode, and runs no
gate-leak check, since a leak is defined against allows it cannot see. Contract 7.7's "absent
interactively" gains the exception in the same change, written by the orchestrating session.

**Reap.** An interactive run with a worktree is reaped like any other. `hydra status` reports it
as waiting for a reap after the session ends.

**Tests.** `tests/harvest.rs`: a harvested record with a policy and one `hydra/capability`
result derives one denial and one limit finding; the same record with `policy` null derives
neither. `tests/spawn.rs`: an interactive launch with `--policy` writes the gate and the settings
into the spawn directory the way a recorded one does, launching nothing.

## Done when

- An interactive `claude` session launched with `--policy read-only` refuses a write with
  Hydra's reason, and its harvested record carries the denial as a `permission_decision`.
- The tests above pass and the suite is untouched.
- `IMPLEMENTATION.md`'s interactive section says which of the two shapes a record is in and how
  a reader tells.

## Outcome

Resolved 2026-09-08, with one measurement still owed. `cursor-agent` on Composer 2.5, run
01a07f62-7061 under `standard`, the first Rust ticket given to that model because Claude's
five-hour window stood at 95 percent: 5.65M tokens, 5.5M cache reads, eight files, 628
insertions, every test green. `interactive::prepare_policy_launch` reuses the recorded launch's
pre-flight, worktree, snapshot, gate and settings rather than copying them, and the harvest
reader derives a `deny` by `hydra_policy` from every error `tool_result` whose text starts with
`hydra/`, with one `Finding::Limit` per gated interactive run saying allows were not recorded.

**Two things the first real launch found, fixed by the orchestrating session.** The gated argv
carried `--permission-mode bypassPermissions` like a recorded run, and interactively that flag
opens a consent dialog before the first prompt; Herdr read the pane as `blocked` and nobody was
there to answer. The flag is gone from the interactive argv. That session exited on the dialog
and wrote no transcript, and the harvest then failed on the missing file at every `hydra status`;
a missing transcript now writes a `run_end` with an error naming the path.

**Measured on the second launch.** `hydra run --interactive --provider claude --model haiku
--policy read-only` in a Herdr pane came up `claude idle`, wrote `run_start` with the policy,
the worktree with `base_tree` and the linked skills, and harvested 23 lines with the `limit`
finding on the record. **The denial itself is unmeasured on a real launch**: the prompt that
should have been refused met "You've hit your session limit · resets 1:50am" instead, so no
tool call was made. Map ticket 50 measured the same settings refusing a write on the same binary
the day before, and `tests/harvest.rs` derives the denial from that result shape, so the gap is
one prompt wide. The check to run after the window opens:

```
hydra run --interactive --provider claude --model haiku --policy read-only
# in the session: ask it to Write a file, then /exit
hydra show <run-id> --events   # expect one permission_decision deny by hydra_policy
```

**What the same record showed on the way.** A harvested run whose only assistant turn is the
provider's rate-limit sentence derives `completed`. Contract 8's `rate_limited` reads a stream
frame, and the harvest path has no rule for the sentence yet. Rough edge, noted in
`IMPLEMENTATION.md`.
