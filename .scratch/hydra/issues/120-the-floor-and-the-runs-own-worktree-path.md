# 120 - The pattern floor refuses a run's own worktree path: a scope it cannot name

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-13, by the session, from the human's note on Hydra's list of 2026-10-13 ("es
decisión de mapa, no de build"). Grilling: the human answers, the session only asks and brings
what it measured.

## Question

On 2026-10-13 a recorded run for another repository, `~/cerebro`, was refused by the gate for
naming the absolute path of its own worktree, `/Users/USER/hydra/worktrees/<run id>/...`, in a
shell command (run `01a1332c`, 146k tokens, nothing written). The pattern floor of harness spec
7.4 refuses any shell command that names `~/hydra/`'s runtime directories, and `worktrees/` is
one of them, so a run that names where it stands is refused by the rule that protects the other
runs. Every Cerebro ticket since then says "relative paths only", and the builders comply, but
a provider that prints `pwd` and pastes it into its next command, or an `rg` with an absolute
glob, hits the floor by accident, and the refusal reads like a policy violation when it is not.

1. Should the floor carve out the run's own worktree, meaning the one path under
   `~/hydra/worktrees/` that equals the run's `cwd` from `run_start`? The gate knows that path:
   `spawn_config.rs` writes it into the hook config. The carve-out would be one prefix compare
   before the pattern match. What it must not do: let the run name a sibling worktree, the
   `runs/` directory or `spawn/`.
2. If the carve-out is wrong, what should the refusal say? Today it reads like any other floor
   hit. The alternative is a refusal that names the cause ("your own worktree is under a
   protected directory; use a relative path") so a provider recovers in one turn instead of
   spending 146k tokens.
3. Does this change the seatbelt profile too? On macOS the profile already allows writes under
   the run's worktree by absolute path, so the seatbelt and the floor disagree about the same
   string, and the floor wins.
4. Is the Cerebro case special because its repository lives outside `~/hydra`, or does a Hydra
   self-edit run hit the same thing? Measure it: every run's worktree is under `~/hydra/worktrees/`
   whatever the repository, so the answer should be no, but the attack corpus of map 68 should
   say whether any recorded attack relied on an absolute worktree path.

## Answer

