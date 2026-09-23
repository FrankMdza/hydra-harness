# 78 - A non-ASCII root is refused before launch

Type: build
Status: resolved
Blocked by:

## What

Map ticket 68, answer 1, resolved 2026-09-22. Build ticket 60 made the compiler and the gate fold
case the same way, ASCII only on both sides (`src/policy/gate.py:114`). The macOS filesystem folds
non-ASCII letters too: `CAFÉ` opens `Café`. So on a root whose path holds a cased non-ASCII letter,
every denial and scope under that root misses the other case, and under `permissive` that reaches
the gate script itself. The decision is to keep the ASCII fold and make its assumption a checked
precondition.

Every directory whose path the compiler folds into the gate's deny list or scopes comes from two
roots: `Layout`'s root (`~/hydra` or `HYDRA_ROOT`) and `crate::paths::hydra_home()` (the checkout).
Find where each is read on the way into the compiled config and confirm there is no third.

## Done when

`hydra run`, `hydra resume`, `hydra evolve` and `hydra curate` refuse to launch when either root's
absolute path contains a byte outside ASCII, with an error that names the root, the path, and map
ticket 68, before anything is created. `hydra doctor` reports the same condition as a failing line.
Tests: a scratch `Layout` rooted under a directory named with a non-ASCII letter is refused, the same
with an ASCII name is not; a unit test on the check itself. Tests take a scratch `Layout`, never
`Layout::discover()`. `IMPLEMENTATION.md` is the session's to amend. This ticket edits `src/` and
`tests/` only.

## Outcome

`paths::refuse_non_ascii_roots` checks `policy::resolved_path` of the data root and of
`hydra_home()`, so an ASCII symlink to a non-ASCII directory is refused too; wired into `run::run`
before `layout.ensure()`, both interactive paths, `curate`, `evolve` and `review_blocked`, and the
face through `run::run`. `hydra doctor` prints `data root <path> ascii|non-ascii` and
`checkout <path> ...` and fails on the second. Unit tests in `src/paths.rs` (including the symlink
case) and `src/doctor.rs`; `tests/spawn.rs` refuses a scratch layout under `Café/`. The ticket said
to confirm there was no third root, and there is one: the launching repo's `.git`
(`Spawn::parent_git_dir`) is folded into the deny list and scopes and is not checked. A comment at
the check and `IMPLEMENTATION.md` state it; whether a non-ASCII repo is refused goes to the human.
The first version checked raw bytes (symlink bypass, measured by the Opus review) and wrote a doctor
test that would fail on CI, where no vendor binary is installed; both fixed by an Opus subagent.
Measured on master: spawn 30, lib 193 passed; clippy and fmt clean. Built by `cursor-agent` on
Composer 2.5 under `standard`, run 01a0cc32-1103, 1.2M tokens.
