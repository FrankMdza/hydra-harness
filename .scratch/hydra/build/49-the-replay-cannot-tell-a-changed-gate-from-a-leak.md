# 49 - A gate whose code changed reads every older record as a leak

Type: build
Status: open
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
