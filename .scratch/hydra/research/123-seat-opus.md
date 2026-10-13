# 123 - Seat opus: the run's git pointers and the repositories a run makes

Panel memo for map ticket 123. Advice only. The human answers under `## Answer`.

Every measurement below ran on 2026-10-13 inside run 01a13753, in scratch directories under
this worktree's `target/`, which `.gitignore` covers. All were removed afterwards. The run's
own store was never written: each probe pointed `GIT_OBJECT_DIRECTORY` at a scratch store and
reproduced the run's layout by hand. I launched no provider and did not run `hydra reap`.
The gate refused `git worktree` in every form, so one claim below is from git's documentation
and marked as unmeasured.

The short version is that the ticket asks the wrong question first. A repository inside the
checkout breaks reap today whether or not the pointers reach it, and the pointers are the
smaller half of the problem.

## 1. Is this the design?

**What the code does.** `run_object_env`, `src/worktree.rs:358-375`, hands the run both
pointers, and `prepare_run_objects`, `src/worktree.rs:380-393`, is called from
`compose_child_env` at `src/run/mod.rs:438-442`. Its doc comment ends at line 366-367 with
"The cost is that git inside the run, in any repository, reads and writes this store."
`IMPLEMENTATION.md:5069-5070` says the same: "Git inside a run, in any repository, uses that
store." So the leak is written down twice. Map 74's answer did not name it, but build 103
saw it and shipped it as a known cost. It is the design by acceptance, never by decision.

Every git Hydra starts drops both pointers in `GitLocation::configured`,
`src/worktree.rs:129-136`. That is why Hydra disagrees with the run about where a
sub-repository's objects live.

**What I measured.**

- `git init` plus `git commit` in a new directory under a checkout, with the pointers set,
  creates a `.git` with `HEAD`, `config`, `refs`, `index` and **no `objects/` directory at
  all**. The scratch store gained 3 files. With the pointers dropped, `git rev-parse
  --show-toplevel` in that directory answered the enclosing checkout and `git log` printed the
  checkout's history. Git does not treat a `.git` without `objects/` as a repository, so the
  reader walks up without an error. That matches build 309's account.
- `git clone --no-local` of a two-commit repository, pointers set: exit 0, the store gained 3
  files, the clone's `.git` has no `objects/`. With the pointers it logs its own history.
  Without them it logs **this worktree's** history, `2ffd151` and `285bf99`, again with no
  error. A run that clones a dependency puts the whole pack in its store, and reap imports it
  into the parent, up to `RUN_STORE_IMPORT_CAP`, 1 GiB.
- `git clone` of a local path, pointers set and the source outside the alternates: exit 128,
  "trying to write ref 'refs/remotes/origin/master' with nonexistent object". A run cannot
  clone a local repository whose objects its alternates do not reach.
- Reap's staging, done by hand with reap's own git calls on a checkout holding such a
  sub-repository:
  - If the run never staged, pointer-less `git add -A` stores `sub/f` as a plain file. The
    sub-repository's files travel, its history does not, and its commit and tree objects
    arrive in the parent as unreferenced objects.
  - If the run ran `git add -A` itself, which agents do, the run's git sees a valid repository
    through `GIT_OBJECT_DIRECTORY` and stages a gitlink, mode 160000. Reap's pointer-less
    `git status --porcelain` and `git add -A` then both exit 128 with "fatal: 'sub/.git' not
    recognized as a git repository". `unstage_missing_objects` skips 160000 entries on purpose,
    `src/worktree.rs:758`, and `dirty_outside` runs that status first,
    `src/worktree.rs:932-935`. Reap would fail at `commit_everything`, `src/worktree.rs:918`,
    and leave the checkout behind.
- The option the ticket offers, a sub-repository that owns its objects, measured the same way:
  pointer-less `git add -A` in the parent warns "adding embedded git repository" and stages
  `160000 ec66a16... own`. The commit holds a gitlink and none of the sub-repository's files.
  Git's documentation for `git worktree remove` says a worktree with submodules is removed
  only with `--force`. Reap does not pass it, so I expect reap to refuse at
  `src/reap.rs:512`. If it did not refuse, the files would go with the checkout. Not measured,
  because the gate refuses `git worktree`.

**Recommendation.** Keep the pointers and say plainly that they reach every repository. The
unreferenced objects reap carries are harmless: `index-pack --strict` checks them, the cap
bounds them and `git gc` prunes them. The defect worth a decision is what reap does with a
repository nested in a checkout. Today the result depends on whether the run staged, and one
of the two branches fails reap outright. "Its own and travel as files" is not what reap would
do with a self-contained sub-repository either. Reap would record a gitlink.

**What would change my mind.** A real run, rather than Hydra-on-Hydra tests, losing work or
failing reap on a pointer-carried sub-repository more often than the gitlink case would. Also
a measured `git worktree remove` on a checkout with an embedded repository that succeeds
without `--force` and leaves the files recoverable.

## 2. If not the design, where is the fix?

**What the code does.** No config key moves a checkout's object directory. The doc comment
at `src/worktree.rs:362-364` says so, and a private `commondir` would move the refs too. The
seatbelt grants the run's store at `src/policy/mod.rs:888-899` and allows a metadata read of
the spawn parent at `src/sandbox.rs:191-201` so that git can resolve the pointer.

**Measured.**

- A shim's extra `git rev-parse --git-common-dir` costs 7.3 ms a call: 200 calls took 1.458 s,
  load average 4.1. For scale, 200 `git status --porcelain -uno` took 2.314 s, 11.6 ms each.
  The shim would add about 60% to a cheap git call. Real money in a loop, not in a session.
- The condition the ticket proposes, "set the pointers only when the cwd's `--git-common-dir`
  is the checkout's", is wrong for exactly the case it targets. Before `git init` or `git
  clone` runs, the cwd is an ordinary directory of the checkout, and rev-parse answers the
  checkout's common dir. The shim would set the pointers and `init` would again make a `.git`
  without `objects/`. To get it right the shim has to parse git's argv: `-C`, `--git-dir`,
  `-c`, `GIT_DIR`, and aliases that expand to `init` or `clone`. That is a git front end in
  shell, and every git release can break it.
- A shim widens nothing. Dropping a pointer is already fail-safe, because the parent's
  `objects/` is denied, as the comment at `src/worktree.rs:365-366` says. It is still a new
  executable on every run's PATH that the run must not be able to write, which puts it in
  `spawn/<run-id>/` beside the gate, plus an allowlist change to PATH and a seatbelt read
  grant. All of that to fix a case the run's own git already handles consistently.

**Recommendation.** No shim and no allowlist change. The fix belongs in reap, outside the
seatbelt, where Hydra already rewrites the index before it commits: before
`unstage_missing_objects`, find every index entry of mode 160000 and every directory under
the checkout holding a `.git`, and decide once what happens to them. Two candidates are
"commit the files and drop the gitlink" and "refuse with a message naming the path". Which of
the two is the human's decision.

**What would change my mind.** Evidence that Hydra's users routinely run tools that read
repositories without git's environment, such as libgit2 or gitoxide based editors and
language servers inside a run, against repositories the run made. I have not checked whether
`git2` honours `GIT_OBJECT_DIRECTORY` through `Repository::open_from_env`. If those readers
are common, the walk-up is a correctness bug for runs, and a narrower shim that unsets the
pointers for `init` and `clone` alone could be worth its argv parsing.

## 3. Does the product need to change at all?

**What the code does.** Build 302 moved every fixture git onto `tests/common/git.rs`, which
strips both pointers, and `IMPLEMENTATION.md:6034-6040` records it. `config/preamble.md` says
nothing about git or object stores. It is protected config #4 and goes, verbatim, at the
front of every recorded run's task string, `config/README.md:12-18`.

**Measured.** The suite needs nothing more. The product does: the gitlink case above makes
reap's own git exit 128, and no test covers it. I reproduced it with reap's git commands by
hand, not with the reap binary, so a test in `tests/cleanup.rs` should confirm it first. Make
a checkout, `git init` a sub-directory under the run's pointers, stage from the run, reap.

**Recommendation.** Two changes. A build ticket for reap and a nested repository, once the
human picks from question 2's candidates. And one line in `IMPLEMENTATION.md` beside line
5069 naming the measured consequences: a repository a run makes or clones has no `objects/`,
and a reader without the pointers walks up to the checkout with no error. The preamble line
is the human's call. I lean yes, one sentence, because the walk-up is silent and four
builders this week needed it told. Something like: "Git in this run writes objects to the
run's own store in every repository, including one you create or clone; a tool that does not
read git's environment will not see them."

**What would change my mind on the preamble line.** If the only population that ever hits
this is Hydra building Hydra, the line costs tokens on every run on every repository to help
one. Then the right place is the build tickets' own text, which 302 already made unnecessary
for the suite.

## Recommendation summary

1. Not a decided design but an accepted cost, written at `src/worktree.rs:366` and `IMPLEMENTATION.md:5069`; keep the pointers, and decide instead what reap does with a repository nested in a checkout, which fails reap with exit 128 when the run staged it.
2. No shim: its proposed condition misroutes `git init` and `git clone`, it adds 7.3 ms per git call and a new executable on PATH; fix reap's handling of 160000 entries and nested `.git` directories outside the seatbelt.
3. Yes, the product changes in reap, behind a test in `tests/cleanup.rs` that reproduces the failure, plus one `IMPLEMENTATION.md` line; the preamble sentence is the human's, and I lean toward adding it.
