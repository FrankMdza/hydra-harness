# Panel 72, opus

A safety classifier stopped Opus's write of this file (run 01a0cf89-8b3d, 2026-09-23), so the
seven-section answer does not exist. What follows is the run's closing report, which names the
finding without the steps; the human decided map ticket 72 on it.

A safety classifier stopped the write, so neither panel file exists and the worktree is unchanged. I won't try to write that content again in another form.

- **Ticket 72.** My answer for this ticket centred on the code-execution finding described below. Its write was the one that got stopped. The file needs a human decision on how much of that finding goes into a written panel answer, or it should go to the human directly instead.
- **Ticket 73.** I haven't written this file yet. I can write it now if you want. It doesn't depend on the stopped content. It would cover what past rung-two passes touched, whether the ticket's example allowlist would have refused them, and my recommendation.

What I found for 72, without the steps:

- **Reap commits before it checks the branch.** It reads the branch at `src/reap.rs:386`, commits at `410`, and only compares against `hydra/<run-id>` at `419`. The ticket's core claim about that order holds.
- **The bigger problem is outside the ticket.** Hydra runs git with no sandbox in a checkout whose git metadata the run can write. I confirmed the git behaviour in a scratch repo under `/tmp`, not through Hydra's own code paths. This happens in three places:
  - reap, at `src/reap.rs:381-410`
  - the diff stat at run end, `src/run.rs:1436`
  - the proposer read in `src/rungs/evolve.rs:419`, which runs before the protected-path check at `166`

  It matters most under `curate`, which has no shell but can write its checkout. The fix is to give those git calls a git directory taken from the run record, not from the checkout. `src/resume.rs:96-98` already reads it from there.
- **`SECURITY.md:52` names the wrong path in most repos.** It says `~/hydra/.git/worktrees/<run-id>`, which is only true when the parent repo is Hydra's own checkout.
- **The private-directory test measures nothing here.** `tests/spawn.rs:1509-1514` returns early when it runs inside a seatbelt, so on this machine the git commands go unmeasured.

The scratch repos are in `/tmp/panel72claude`, outside the worktree. They're safe to delete.
