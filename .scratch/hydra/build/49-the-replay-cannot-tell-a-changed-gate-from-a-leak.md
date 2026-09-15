# 49 - A gate whose code changed reads every older record as a leak

Type: build
Status: resolved
Blocked by:

## What

Build ticket 44 added paths to the compiled gate's denied list and the corpus walk went red on
run `01a09405-f451`, launched three hours earlier: its `ls -la .claude/skills/` was allowed by
the gate that ran and is refused by the gate the replay compiles today, so contract 7.7's
re-evaluation calls it a `GateLeak`. It is not one. The gate changed.

The replay already handles one kind of change: `policy_sha` on `run_start` names the policy
set the run compiled against, and `policy_moved` turns a disagreement into a `Limit` when it is
not today's. Nothing names the gate's own code. The denied list, the floor and the worktree
carve-outs are Rust and `gate.py`, and a record made before a change to any of them carries no
mark that says so.

Put the mark on the record. The hook config Hydra writes already carries the sha256 of the gate
script; write the sha of the compiled gate config (script plus the lists it embeds) on
`run_start` under contract 15's rule for a new field, the way ticket 42 added `scopes`, and have
every normalizer treat a `GateLeak` or `Disagreement` on a record whose gate sha is not today's
as a `Limit` that says which gate answered. Records before the field carry none and read as
they do today. Test: a synthetic record with a foreign gate sha and a call today's gate denies
produces the limit and no leak; one with today's sha still produces the leak.

Found 2026-09-12 by the corpus walk after build 44. The agent that built 44 predicted it in its
report and it was right.

## Outcome

`gate_sha` is on `run_start`, contract 15's rule for a new field: absent on every record
written before it and on every run that compiled no gate. `src/run.rs` hashes the script's
bytes back off the disk, which is what the hook config already checks against at every call.
`policy::gate_sha` compiles the same bytes without writing them, and that is the function the
replay calls.

**The half the ticket names and the half it does not.** Writing the field is not the fix. A
reader that only asks whether the field is there trusts the first record from three gate
versions ago the moment one carries a sha, which is the bug wearing the fix's clothes. So
`GateMark` in `src/normalize/mod.rs` compiles the gate for the spawn the replay rebuilds and
compares: `Todays` reports a `GateLeak` or a `Disagreement`, `Foreign` and `Unnamed` downgrade
to a `Limit` naming which mark is missing. One compile per record, not per call; the corpus
walk over 207 records costs no measurable time.

**Seven call sites became one door.** The three normalizers asked two different questions in
four spellings: `stood_in` in claude.rs and twice more, `gate_unnamed_or_stood_in` in cursor.rs
and agy.rs, and claude.rs never asked the gate question at all. `ReplaySpawn::cannot_back` is
the single door now, and it names every reason rather than the first, because a record can lack
the gate mark and have had its scopes rebuilt and a reader fixing one wants to know about the
other. That is one axis of build ticket 55 closed on the way.

Tests: `tests/normalize.rs` 78 of 78. The new one builds the same leaking record three ways,
with today's sha, with a foreign one and with none, and asserts the leak on the first and a
limit with no leak on the other two. `Record::gated` in `tests/common/mod.rs` now writes the
sha of the gate this build compiles for its own sandbox, which is what makes the three existing
leak and disagreement tests keep meaning what they meant.

**What is still null.** `src/interactive.rs` writes `gate_sha: null` on both arms; the gated
arm compiles a gate and does not thread the sha out of `prepare`, so a gated interactive run
reads back as `Unnamed`. The harness record `hydra evolve` writes compiles no gate and null is
the truth there. Both are commented where they are written, and `IMPLEMENTATION.md` says so.
