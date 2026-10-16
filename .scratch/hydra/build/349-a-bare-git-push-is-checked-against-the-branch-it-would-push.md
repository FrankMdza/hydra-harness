# 349 - A bare `git push` is checked against the branch it would push, so `git push` from a checkout of `master` is refused like `git push origin master`

Type: build
Status: open
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from the plan of that night and its panel: the opus and grok
seats both named it as a fix before publication, and IMPLEMENTATION.md lines 1063 to 1065 have
admitted it since build 58. `gate.py::check_git`, the `push` branch near line 929, refuses the
forbidden flags, a `+refspec`, and a positional refspec whose branch is in
`policies/floor.json`'s `protected_branches` or starts with a protected prefix. With no positional
after the remote, or no positional at all, it returns `None`, and git pushes the current branch
to its upstream, which inside a run is whatever the checkout has. A run seeded from a checkout of
`master` with an upstream can `git push` and reach it.

## What

When `push` has no refspec, the check reads the branch it would push: the worktree's `HEAD`
through the `.git` file a worktree carries, resolved to `refs/heads/<name>`, and that name is
judged by the same two lists. A detached `HEAD` pushes nothing by default and is left alone. If
the upstream is configured under another name (`branch.<name>.merge` in the checkout's config),
the upstream name is judged too, since that is the branch the remote would receive. The check
reads files and runs no git. The reason reads `git push with no refspec from the protected branch
master`. The rule is the floor's, so it holds under every policy and `--allow-floor-escape` lifts
it like the rest.

## Contract

Input: a `Bash` payload whose command is a `git push` with no refspec, and a cwd whose `.git`
names a branch. Output: the decision and the reason. Latency: two file reads; measured once in
`tests/gate.rs` and stated in the Outcome.

## Cases

In `tests/gate.rs` beside the existing push cases, each with a scratch worktree whose `HEAD` is
set by the test, written first:

1. `git push` from a worktree on `master` is refused with the reason above, under `permissive`.
2. `git push origin` from the same worktree is refused the same way.
3. `git push` from a worktree on `hydra/<run-id>` is allowed, as every run's own branch is.
4. `git push` from a worktree on `feature/x` whose `branch.feature/x.merge` is `refs/heads/main`
   is refused, naming `main`.
5. `git push` from a detached `HEAD` is allowed.
6. `git -c push.default=current push` is judged by `HEAD` alone, which is case 1 or 3.
7. Every existing push case keeps its verdict.

## Do

1. The cases, red.
2. The check, the cases green, the suite green.
3. IMPLEMENTATION.md lines 1063 to 1065 rewritten; SECURITY.md's git paragraph if it names the
   residual.
4. `hydra doctor --walk`: how many recorded `git push` commands had no refspec, in the Outcome.

## Done means

The four checks green, and the Outcome quotes case 1's refusal.
