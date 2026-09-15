# 57 - A run can relaunch the binary and name its own policy

Type: build
Status: resolved
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

## Outcome

`src/containment.rs` is the one place that decides whether a call comes from inside a run.
`hydra run` sets `HYDRA_RUN_ID` in every gated child's environment, reads it back out of its own at
launch, and looks it up in the corpus. Three answers: `Outside` is a human, `Inside` is a live run
under this layout, and `Stale` is a marker naming an ended run or no run at all. Stale is refused
rather than read as permission, which also catches the `HYDRA_ROOT` half of the ticket for free: a
child that moves the root moves the corpus the marker is looked up in, and the lookup fails.

A caller inside a run that names no parent, or names another run, writes a `containment` refusal and
launches nothing. Contract 8.3 has a fifth row now and section 5 of that document says the code
derives to `gated`, the arm `policy_narrowing` takes, because the two are one rule caught at two
moments: the policy named too wide, and the flag left off entirely.

**Measured, not assumed.** Disabling the check and rerunning the test launched a real `claude` under
`permissive` from a marked process with no `--parent`, outcome `completed`. The refusal text and the
record are what stop it now. Five tests in `tests/delegation.rs`, three on the classifier and two
driving the built binary as a subprocess so the marker is set in the child's environment rather than
the test process's. The refusal lands before the provider is probed, so they cost no quota.

**The residual, stated in `SECURITY.md`.** A child that unsets the variable walks through. A
variable is a thing a process removes from its own children's environment, and no argv-level check
survives that. What this closes is the absent flag, which was the whole of the attack as the review
measured it: no forging was needed. Map ticket 62 is where the uid alternative belongs, and it is
still open and still Frank's.

**One thing the ticket assumed that is not quite true.** It says `standard` grants shell and shell
reaches the binary. It does, but not by name: `hydra` is not on PATH on this machine, so a run
reaches it as `~/hydra/target/release/hydra`, which no deny list covers. That does not change the
finding and it does change how a reader reproduces it.

Also fixed while here: `CLAUDE.md` claimed a read of `~/.claude/` is refused under `standard`. The
review found that true of the `Read` tool and false of `cat`, and the file says so now.
