# 78 - A non-ASCII root is refused before launch

Type: build
Status: open
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
