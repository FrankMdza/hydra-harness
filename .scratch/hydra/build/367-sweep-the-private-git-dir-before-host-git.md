# 367 - Sweep the run's private git directory before any host-side git on its checkout

Type: build
Status: claimed
Blocked by: 364

## Objective

Carry out question 1 of map ticket 135, `.scratch/hydra/issues/135-what-bwrap-does-with-paths-that-do-not-exist-and-the-private-git-dir.md`, with the panel's reasoning in `.scratch/hydra/research/135-synthesis.md`. Read both first.

Since build 364, bubblewrap binds `<git>/worktrees/<run-id>/` writable, so a run can plant files there that host git reads afterwards, outside the sandbox. Five were measured on git 2.55 with plain git, not under bwrap. A `COMMIT_EDITMSG` symlink makes reap's `git commit` write its message through the link onto any file. `MERGE_HEAD` gives reap's commit a second parent. `locked` makes `git worktree remove` exit 128. `config.worktree` runs a filter driver or monitor when `extensions.worktreeConfig` is on. A fifo as `index` hangs `git status`. On macOS SBPL allows writes to the literal `index` path, so the last one may hold there too; nobody has measured it.

## Rule

Before any host-side git command on a run's checkout, a sweep:

1. removes every entry of the private directory except `HEAD`, `commondir`, `gitdir`, `index`, and `logs` when `logs` is a real directory. It uses `symlink_metadata`, never follows a link, and removes a directory recursively without following links inside it;
2. refuses with an error naming the path when `index` exists and is not a regular file;
3. runs on both platforms.

## Where

The panel listed the call sites. Seats Grok `01a17e94-ceb8` and Flash `01a17e95-8ed9` named them by file and line. Read `src/worktree.rs` from `CheckoutGit` down, `src/reap.rs`, `src/rungs/curate.rs`, `src/rungs/evolve.rs`, `src/status.rs`, `src/run/record.rs` and `src/interactive.rs`. The first host git in reap is `main_worktree` at `src/reap.rs` around line 617, before `clear_private_logs` at line 485 runs.

Put the sweep at one entry every host-side git on a run checkout passes through, so a new call site cannot skip it. The natural place is where a `CheckoutGit` is built for a run, `for_run` and `new`, made fallible if needed. A host git on a run checkout that does not go through `CheckoutGit` either moves onto it or calls the sweep itself, with a comment saying why. Launch-time git that runs before the child starts, `create` and `seed_tree`, needs no sweep; say so in a comment. Fold `clear_private_logs` into the sweep.

`hydra status` reads a live run, so the sweep there can race the run. Do not remove files from a live run's directory: for a live run, run only the `index` check and skip the removal. Use `src/live.rs` to tell.

## Do

1. Tests first, in `tests/cleanup.rs` beside the reap tests. Plant each of the five in a throwaway repo's private directory: a `COMMIT_EDITMSG` symlink to a file outside, `MERGE_HEAD` holding another commit's sha, `locked`, a `config.worktree` with a filter driver that touches a canary, and a fifo as `index`. Run `hydra reap`. The outside file is untouched, the reaped commit has one parent, the worktree is removed, the canary does not exist, and the fifo case refuses without hanging, with a timeout in the test. Each case must fail before the change; say which did in the Outcome.
2. A unit test that the sweep keeps exactly the five names and leaves `logs` alone only as a directory.
3. Leave `src/sandbox.rs`, `src/paths.rs` and the settings-file removal alone. Build 366 owns those and runs beside you.
4. Amend `IMPLEMENTATION.md` where it describes reap and `clear_private_logs`, and `SECURITY.md` where it says what a run can write in its private git directory. Say what the sweep removes, where it runs, the live-run exception, and that nothing was measured under bwrap.
5. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, `cargo nextest run --release -p hydra --test cleanup --test status --test curate --test evolve`, and `cargo nextest run --release -p hydra --lib worktree`.

## Outcome
