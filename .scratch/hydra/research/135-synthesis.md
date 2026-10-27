# 135 synthesis: the private git directory and missing deny paths under bubblewrap

Panel of 2026-10-27, decided by the panel because the human said so before leaving. Seats, all under `standard`, read only:

- Opus, `claude-opus-5-5`, run `01a17e93-96a0`, 1.6M tokens.
- GPT, codex `gpt-6-luna`, run `01a17e94-644a`, 0.19M tokens.
- Grok, cursor `grok-4.7-high`, run `01a17e94-ceb8`, 1.9M tokens.
- Flash, agy `gemini-3.8-flash-high`, run `01a17e95-8ed9`, 3.0M tokens. Gemini 4 Argon was out of quota that day.

## Question 1: A, three to one

GPT, Grok and Flash voted A, the sweep. Opus voted B, moving the index with `GIT_INDEX_FILE`.

Opus's case against A is the strongest point in the panel, and it changes the answer's shape without changing the vote. Reap is not the first host git on a run's checkout. The diff stat at run end (`src/run/record.rs` around line 201), `hydra status` through `tree_of_index`, the `curate` and `evolve` readers and interactive rollback all run git there first. Grok and Flash listed the same call sites and put the sweep before every one of them, so A stands, as a sweep at every entry, not only in reap.

Opus also measured a hole in B. Git writes through a symlink at `GIT_INDEX_FILE` and creates its target. B is safe only if host git never reads the run's index. Grok added that a fifo at the run's own index hangs any host reader of it. A keeps macOS SBPL as it is.

Decision: before any host-side git command on a run's checkout, a sweep removes every entry of `<git>/worktrees/<run-id>/` except `HEAD`, `commondir`, `gitdir`, `index` and a `logs` that is a directory, using `symlink_metadata` and never following a link. It refuses when `index` exists and is anything but a regular file. It runs on both platforms. On macOS the extra-name sweep finds nothing, but the index check matters there too. Opus noted that SBPL allows writes to the literal `index` path, so a symlink or fifo index may already be possible on macOS. Nobody has measured that.

Residual, named by no seat as a blocker: `hydra status` reading a live run races with the run. The sweep can make the race smaller but cannot close it.

## Question 2: one rule per kind

- **Protected config**: bind `config/` read-only whole in place of its two optional files. Opus, GPT and Flash. Grok would skip the mount, because the checkout is already read-only under `hydra_home()`. Binding the directory is harmless where that holds and covers a layout where it does not.
- **Layout directories**: `Layout::ensure()` creates `pruned`, `puppets`, `approvals`, `skills`, `skills-snapshots` and `skill-worktrees`, and the profile binds them read-only. Unanimous. Opus checked that `Store::open` accepts an empty `skills/`.
- **Skill and plugin roots in the worktree**: create an empty directory before a Linux launch and bind it read-only. Unanimous. Git does not track an empty directory. Grok ran that: `status` stayed clean and `add -A` committed nothing.
- **Settings files**: a read-only `{}`. Opus, GPT and Flash, with different mechanics. The session takes Flash's version, because it accounts for a fact Grok and Flash found in bubblewrap's source and issues: under `--dev-bind / /`, bwrap creates a missing bind destination on the host filesystem. Neither seat ran bwrap. So the `{}` lives under the spawn directory, it is bound over the destination, and reap removes the empty file bwrap leaves in the worktree. Grok dissented: bind `.claude/` read-only whole, creating it empty if absent, so no file is ever made in the worktree. That costs the run every write under `.claude/`. If a Linux measurement shows Claude Code treats `{}` differently from a missing file, Grok's rule is the fallback.
- **Backstop**, from Opus: when a deny source is still missing at compose time, refuse the launch and name the path. Today `compose_bwrap` panics through `expect`.

## What none of this measured

No seat had bubblewrap. The host mount-point creation, `{}` versus a missing settings file, and every attack under bwrap need a Linux host. Docker Desktop on this Mac gives one; see the build list.

## Build tickets

1. 366: the four missing-path rules and the backstop. Linux behavior; `Layout::ensure()` changes on both platforms.
2. 367: the private-directory sweep and the index check before every host-side git on a run checkout, both platforms.
3. 370: measure on Linux in Docker. Numbered 368 when this was written; 368 and 369 went to map 108. A real bwrap launch end to end, the five private-directory attacks, the mount-point claim and `{}` under Claude Code.
