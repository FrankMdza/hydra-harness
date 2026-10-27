# 371 - Host git never trusts the run's index, and every kept entry of the private directory is checked

Type: build
Status: claimed
Blocked by: 367

## Objective

Close what the adversarial review of `05a9e21..435f156` found in build 367's sweep and build 366. Prompt: `.scratch/hydra/research/panel-review-435f156.md`. Seats: GPT `01a17eb3-86c2`, Grok `01a17eb4-20d0`, Flash `01a17eb4-c167`. Read `sweep_private` in `src/worktree.rs` and the 367 Outcome first.

## Findings this ticket closes

1. **High, measured by Grok on git 2.55, and older than 367.** The run may write its index on both platforms: SBPL allows `index` and `index.lock`, and bwrap binds the private directory. A regular index whose entry carries the skip-worktree bit and points at a blob the file on disk does not hold makes reap commit that blob: `git add -A` leaves the bit, `git commit` records the planted bytes, and the worktree shows something else. A reviewer reading the checkout never sees what the branch holds.
2. **Medium, Flash.** A zero-byte or corrupt regular index passes the sweep and makes host git exit 128.
3. **Medium, Grok and Flash, measured.** `HEAD`, `commondir` and `gitdir` are kept without a type check. A fifo at `HEAD` hung `git rev-parse` for 19 s until killed. Both sandboxes deny writes to these today, so this is defence in depth, and the check is cheap.
4. **Low, Flash, measured.** A `logs/HEAD` with mode 000 makes reap's `git commit` fail on the reflog append, and an unreadable `logs/` makes `sweep_logs` fail on `read_dir`.
5. **Low, Grok, measured.** The bubblewrap preparation's `create_dir_all` on `.claude/skills`, `.agents/plugins` and `.codex/skills` in `src/sandbox.rs` follows a symlink seeded into the worktree, and creates the directory where it points.
6. **Low, Flash and Grok.** `src/reap.rs` around line 301 runs `git_common_dir(&layout.skills())`. Since build 366 `skills/` can exist empty, and git then walks up to Hydra's own repository. Check `skills/.git` first, as `curate::store` now does.

GPT's finding that the sweep removes `info/sparse-checkout` and `config.worktree` from a sparse repository was measured by the session on git 2.55: after removing both, `git add -A` staged nothing, because the index entries keep their skip-worktree bits. Fix 1 changes that, since a rebuilt index has no skip-worktree bits. Read rule 1's last sentence.

## Rules

1. **Host git never reads the index the run left.** On a full sweep, the run's index is removed, whatever its type, and the host rebuilds it before its first git command that reads the index: `git read-tree HEAD`, then whatever the caller already does, such as `add -A`. Do it once, inside the sweep's owner, so no caller can forget it. A fifo `index` is removed, not refused. A rebuilt index has no skip-worktree bits, so in a sparse repository reap would commit the files outside the cone as deleted. So `hydra run` refuses to launch in a repository with sparse checkout on, which `core.sparseCheckout` in the common config or a `sparse-checkout` file in the parent's git directory shows, and names it. Refusing at launch costs nothing; refusing at reap would strand a run that already spent its quota. Measure the deletion in a test before writing the refusal.
2. **Kept entries are typed.** `HEAD`, `commondir` and `gitdir` must be regular files with one link, or the sweep refuses and names the path.
3. **`logs`** goes whole on a full sweep, as `clear_private_logs` did before 367. Git recreates it, and the reflog of a run checkout has no reader. If something needs it kept, say what in the Outcome.
4. **The skill and plugin roots** are created only when no component from the worktree root down is a symlink. A symlinked component refuses the launch, naming it.
5. **`reap`'s skill-store lookup** checks `skills/.git` before `git_common_dir`.

## Do

1. Tests first in `tests/cleanup.rs` and `src/worktree.rs` tests: the skip-worktree smuggle, using Grok's recipe of a planted blob, a different file on disk and the bit set, where the reaped commit holds the bytes on disk; a zero-byte index; a fifo index; a fifo `HEAD`; an unreadable `logs/HEAD`; a sparse repository refused by name. In `src/sandbox.rs` tests: a seeded `.claude` symlink refuses. Each must fail before the change; say which did.
2. Keep the live-run branch of `for_run_in`: a live run gets the type checks and nothing removed.
3. Amend the 367 paragraphs in `IMPLEMENTATION.md` and `SECURITY.md`.
4. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, `cargo nextest run --release -p hydra --test cleanup --test status --test curate --test evolve --test worktree --test spawn`, and `cargo nextest run --release -p hydra --lib worktree --lib sandbox`.

Build 369 runs beside you in `src/cockpit/`, `src/record/secrets.rs` and the docs. Do not touch those.

## Outcome
