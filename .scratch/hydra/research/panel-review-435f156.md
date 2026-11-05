# Review panel on 3f5e6a8..95e01a9

You are one seat on an adversarial review panel. Read only: do not edit, create or delete a file in the repository, and do not commit. Scratch experiments under `/tmp` are fine. Your seat name is in your task line.

## What changed

Run `git log --oneline 3f5e6a8..95e01a9` and `git diff 3f5e6a8..95e01a9 -- src tests SECURITY.md IMPLEMENTATION.md`. Two builds carry map ticket 135, `.scratch/hydra/issues/135-*.md`, whose reasoning is in `.scratch/hydra/research/135-synthesis.md`:

- 366, `src/sandbox.rs`, `src/paths.rs`, `src/reap.rs`, `src/rungs/curate.rs`: under bubblewrap every missing deny path gets a source. `config/` is bound read-only whole, `Layout::ensure()` creates six directories, skill and plugin roots are created empty, a `{}` under the spawn directory is bound over a missing settings file, reap removes an untracked empty or `{}` settings file, and compose refuses by name instead of panicking.
- 367, `src/worktree.rs` and its callers: before host-side git on a run checkout, `sweep_private` removes every entry of the private git directory except `HEAD`, `commondir`, `gitdir`, `index` and a real `logs` directory, and refuses a non-regular `index`. A live run gets the index check only.

Ticket 365 also landed in this range, `src/policy/gate.py`; review it only if you have words left.

## Questions

1. Find a host-side git call on a run checkout that runs before the sweep, or that builds its context without it. Name file and line and how a run reaches it.
2. Find an entry the sweep keeps that a run can still use against host git: `HEAD`, `commondir` or `gitdir` replaced on a platform where they are writable, a `logs` directory holding something other than links and multi-linked files, the `index` itself as a regular file with hostile content.
3. Is there a race between the sweep and the git that follows it? Who can write the private directory at that moment?
4. Does the sweep delete anything git needs in a legitimate checkout: sparse checkout's `info/sparse-checkout`, a legitimate `config.worktree`, `refs/` for bisect? What breaks for a user whose repo uses them?
5. For 366: can a run use `drop_empty_settings`, the `{}` source or the created roots to delete or write something it should not? Does `Layout::ensure()` creating `skills/` empty break anything beyond `curate::store`, which the session already fixed?

## Answer

At most 800 words. Each finding gets a severity, a file and line, and the input that shows it. Say which findings you checked by running something. End with `## Pending from me`, `## Changed` and `## Found`.
