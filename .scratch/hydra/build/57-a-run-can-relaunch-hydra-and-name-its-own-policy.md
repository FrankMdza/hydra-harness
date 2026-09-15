# 57 - A run can relaunch the binary and name its own policy

Type: build
Status: open
Blocked by:

## What

Harness spec 7.3 says a child is never wider than its parent and that a human at depth 0 is the
only thing that widens anything. Nothing enforces it. `Lineage::of` returns depth 0 with no
bound when `--parent` is absent, and no mark says a process is inside a run: no environment
variable, no cwd test, nothing on the record. `standard` grants shell, and shell reaches the
binary, so a run under `standard` can launch `hydra run --policy permissive` and get a child
bounded by nothing. No forging is needed; the flag is simply absent.

`HYDRA_ROOT` on the same command line relocates the whole deny list too, which is the same
hole wearing a second hat.

The shape of a fix: a spawn-time marker the child cannot unset, checked by `hydra run` before
anything else. The environment allowlist is the natural carrier and is also the thing a child
can rewrite for its own children, so the marker has to be something a child cannot forge in a
way that matters: a run id the parent recorded, checked against the corpus. A caller inside a
live run with no `--parent` is an attempted escalation and writes a `gated` record rather than
launching. Map ticket 62 asks whether this whole class should be answered by a uid instead.

Found by the adversarial review, `docs/reviews/2026-09-14-the-gate-under-attack.md`.
