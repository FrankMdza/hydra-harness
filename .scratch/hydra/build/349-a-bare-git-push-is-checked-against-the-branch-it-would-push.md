# 349 - A bare `git push` is checked against the branch it would push, so `git push` from a checkout of `master` is refused like `git push origin master`

Type: build
Status: resolved
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

## Outcome

Built in `src/policy/gate.py`. `floor_violation` and `check_git` now take the payload's `cwd`,
which `evaluate` already holds. A `git push` with fewer than two positionals goes to
`bare_push_violation`. That reads the branch `HEAD` names, found by walking up from `cwd` to a
`.git` file or directory. It judges that name against `protected_branches` and the prefixes. If
the branch has a `branch.<name>.merge` in the shared config, it judges that name too. The shared
config sits where the worktree's `commondir` says, not in the worktree's own gitdir. A detached
`HEAD` returns nothing. The check runs no git.

Two things the ticket did not say turned out to matter. `floor_violation` never received the cwd,
so the cwd had to be threaded from `evaluate`. And the branch config of a linked worktree lives in
the parent repository, so reading only the gitdir would have missed case 4.

Tests, in `tests/gate.rs`, written first. Four new tests cover the cases: the refusals for
`master` (`git push`, `git push origin`, `git push -u origin`, `git -c push.default=current push`),
the run's own branch allowed, the upstream `main` refused, and a detached `HEAD` allowed. Before
the change the two refusal tests failed with `hydra/allow: policy permissive allows shell`. After
it, the new tests and all 18 floor tests pass. The full suite passed: 1451 run, 4 skipped.

Case 1's refusal reads, from the format string in `evaluate` and the substring the test asserts:
`hydra/floor: the pattern floor refuses git push with no refspec from the protected branch master.
Lift it for one run with --allow-floor-escape.` Case 4 names `main`: `git push with no refspec to
the protected branch main, the upstream of feature/x`.

Latency, measured with a Python harness in `target/` rather than in `tests/gate.rs`. A timing
assertion would flake, so the harness stayed out of the suite. It loads `gate.py` with
`policies/floor.json` spliced in, against a linked worktree on `master`, and takes the minimum of
five runs of 2000 calls on `/usr/bin/python3`. A bare push through `floor_violation`: 275 µs. The
same function on `git status`: 82 µs. `bare_push_violation` alone: 106 µs.

Not measured: the `hydra doctor --walk` count of recorded bare pushes. That walk reads
`~/hydra/runs`, which is outside this worktree. Someone with access should run it.

Residuals, found while writing the IMPLEMENTATION paragraph and not fixed here:

- `git push origin HEAD` is allowed from a `master` checkout. The refspec loop reads the literal
  name `HEAD`, which is not protected. The harness returned `None` for it.
- `push.default=matching` makes git push every branch whose name exists on both ends, `master`
  included. The check reads `HEAD` alone, so a run on `hydra/<run-id>` with matching set is allowed.
  This is from git's documentation. The harness did not run git.
- A `cd` earlier in the same command is not followed. The check reads the payload's cwd.

`SECURITY.md` has no git-push paragraph that names this gap, so it is unchanged. `IMPLEMENTATION.md`
lines 1063 to 1065 are rewritten to say what the check covers and what it does not.

**Measured by the session on master, 2026-10-15:** `hydra doctor --walk` over 976 records raised no new finding; the walk does not count bare `git push` commands, so that number stays unmeasured. The suite is 1460 tests, all green. The three residuals the run named, `git push origin HEAD`, `push.default=matching`, and a `cd` earlier in the command, stay open and are written in IMPLEMENTATION.md by the run.
