# 28 - `hydra run --interactive --policy <name>` on `claude`

Type: build
Status: open
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
