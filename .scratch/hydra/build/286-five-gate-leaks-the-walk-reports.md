# 286 - Five gate leaks the walk reports

Type: build
Status: open
Blocked by:

## What

`hydra doctor --walk` on 2026-10-04 reports `GateLeak` findings, calls the record shows ran that
today's gate replay refuses:

1. Four `claude` runs (`01a10415-de88`, `01a10445-e17a`, `01a1049c-35c1`, `01a104cf-8b1f`, on
   `~/another-private-repo` and `~/private-repo`) ran `Bash` commands that name `/users/user/hydra`,
   which `hydra/protected-path` refuses under every policy. `01a1049c` also ran `TaskStop`,
   `provider_native`, which `standard` does not allow.
2. One `cursor-agent` run on `cursor-grok-4.6-high` (`01a1051f-e4e1`) called `updateTodos` four
   times, `provider_native`, not allowed under `standard`.

For each: read the record and the gate that was compiled for it (`gate_sha`, `policy_sha`), and
say whether the live gate let the call through (a real leak: name the path in `gate.py` or the
hook config that missed it), or the replay disagrees with a gate that did refuse (a reader bug),
or the policy set moved since (`StalePolicy` should have said so). `updateTodos` is new with Grok;
say whether cursor's hook sees it at all. Fix what is a leak, with a test that fails before; a
reader bug likewise. A run cannot read `~/hydra/runs`: the session copies the five records into
the worktree's scratch before launch, or runs the reading part itself.

## Lead, from the session

In `01a10415-de88` the refused call is `cd /Users/USER/hydra/worktrees/01a10415-de88-.../docs/costos; uv run ...`:
the agent entered its own worktree by absolute path, and the command ran and printed its result.
Two readings: the live gate let it through, which is a leak, since the build tasks' own template
says a shell command spelling its worktree's absolute path is refused; or today's replay refuses
what the gate compiled for that run allowed, because the protected-path rule in `gate.py` moved
after it ran, and `StalePolicy` (build 49) notices a moved policy set but not a moved gate
script. `run_start.gate_sha` against the sha of the gate the replay compiles tells them apart.

## Outcome
