# 325 - `hydra reap` on a checkout where the run staged a nested repository, measured and pinned in `tests/cleanup.rs`

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 123's answer. A run's git carries
`GIT_OBJECT_DIRECTORY` and `GIT_ALTERNATE_OBJECT_DIRECTORIES`, so a `git init` plus commit under
the worktree makes a `.git` with no `objects/` of its own. If the run then runs `git add -A`
itself, which agents do, its git stages a gitlink, mode 160000. Reap's pointer-less git cannot
resolve it: Opus reproduced by hand that `git status --porcelain` and `git add -A` exit 128, "not
recognized as a git repository", `unstage_missing_objects` skips 160000 entries on purpose
(`src/worktree.rs` line 758), and reap fails at `commit_everything` (line 909) leaving the
checkout behind. Nobody has reproduced it with the reap binary. This ticket does, and pins what
happens, so build 326 can change it on purpose.

## What

A case in `tests/cleanup.rs`, beside the ones that drive `hydra reap` against a throwaway
repository: create a run's checkout the way the other cases do, inside it `git init` a
sub-directory and commit one file there with the run's two pointers set the way `run_object_env`
sets them, then `git add -A` from the checkout root with the same pointers (this stages the
gitlink), then reap without the pointers, as reap runs. Assert what you measure: the exit code,
whether the checkout is still there, what the message says. Also the other half: the same
checkout where the run never staged the sub-repository, which should reap fine with the files
stored as plain files and the sub-repository's history arriving as unreferenced objects; assert
that too (count the parent's unreferenced objects before and after, or check that the committed
tree holds the files and no gitlink).

Both cases carry a comment naming build 326 as the ticket that will change the first case's
assertion.

## Do

1. Read `tests/cleanup.rs` for the fixture helpers (`tests/common/git.rs` strips the pointers
   since build 302; you need a variant that sets them for the "run's git" steps), and
   `.scratch/hydra/research/123-seat-opus.md` for the exact commands Opus ran.
2. The two cases. Do not change `reap` in this ticket.
3. In the Outcome, the measured exit code and message, and the count of objects that arrived.

## Done means

The two cases green against today's reap, `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo nextest run --release --workspace`, `cargo test --release
--workspace --doc`. Amend `IMPLEMENTATION.md` beside the `run_object_env` paragraph with the
measured reap behaviour.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Two cases in `tests/cleanup.rs`, driving `hydra::reap::reap` against a throwaway checkout whose
`sub/` was inited and committed under `run_object_env`. Reap itself was not changed.
`nested_repository` sets the two pointers for the run's git steps; fixture git still strips them
for reap, as build 302 does. Both comments name build 326.

Staged (`git add -A` from the checkout with the pointers): `Reaped::Failed`, `failures() == 1`.
The CLI would exit 1. `why` is `git status --porcelain -z --untracked-files=all failed: fatal:
not a git repository: '.git'` plus `fatal: 'git status --porcelain=2' failed in submodule sub`.
The checkout and spawn directory stay. The branch does not hold `sub/note.txt`.

Unstaged (the nested repo exists, the run never staged it): also `Reaped::Failed`,
`failures() == 1`. `commit_everything` does run: the tree is `160000 commit <nested HEAD> sub`,
not the files. Then `git worktree remove` exits 128, `why` contains `'sub/.git' not recognized
as a git repository` and `code 128`. The checkout and spawn directory stay. The nested blob,
tree and commit arrive in the parent: `git fsck --unreachable --no-reflogs` went from 0 to 3
lines, one for each, and `cat-file -e` sees all three.

False: Opus's `'sub/.git' not recognized as a git repository` is the unstaged
`worktree remove` line, not the staged `git status` line. The unstaged case does not reap, and
it does not store the files as plain files. `unstage_missing_objects` skipping 160000 is real
and was not the staged failure: `dirty_outside`'s status is. The nested `.git` having no
`objects/` was true. 326 was amended: treating only index gitlinks misses the unstaged path.
