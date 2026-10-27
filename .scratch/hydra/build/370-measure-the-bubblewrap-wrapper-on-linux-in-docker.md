# 370 - Measure the bubblewrap wrapper on a real Linux in Docker

Type: build
Status: claimed
Blocked by: 366, 367

## Objective

Builds 318a, 318b, 363, 364, 366 and 367 wrote the Linux wrapper from reading bubblewrap and git, and every Outcome says no Linux host ran it. Map 135's panel named four claims nobody measured. This ticket measures them on the Docker Desktop VM of this Mac, with the recipe in the session's memory of build 331: `rust:1-bookworm`, an unprivileged `ci` user, `git archive HEAD` into `/home/ci/hydra-harness`. Add `apt-get install bubblewrap`. bwrap needs user namespaces inside the container, so run it with `--security-opt seccomp=unconfined --security-opt apparmor=unconfined` and, if that is not enough, `--privileged`. Say which was needed.

The session runs this itself, not a `hydra run`. The VM takes 4 GB of the Mac's 8, so no Hydra run may be live while the container compiles.

## Measure

1. `cargo test --release --workspace` as `ci`, with the CI job's environment and its skip list. Then the same run without `--skip sandbox`, to see which sandbox tests pass on Linux with bwrap present.
2. A real launch: `hydra run` with the stub provider `tests/pty_tee.rs` uses, under `standard`, in a scratch repository. Does bwrap start? Read `sandbox_kind` and the witness on the record.
3. Inside that launch, from the stub's shell: write to each deny path and to the worktree, read the corpus, connect to a sibling's socket. Each result goes in a table: path, expected, measured.
4. Map 135's four claims: whether `--dev-bind / /` plus `--ro-bind SRC DEST` creates DEST on the host; whether git's index rename works under the whole-directory bind; whether Claude Code reads a `{}` settings file as absence (skip if Claude Code cannot run in the container, and say so); and the five private-directory attacks against `hydra reap` after build 367, plus build 371's plants: a skip-worktree index, a zero-byte index, a fifo `HEAD` and an unreadable `logs/HEAD`.
5. Map 136: with `--new-session`, does a provider's child stay in the group of the pid Hydra records? Does `--unshare-pid` kill a `setsid` escapee when bwrap exits, and does it kill a delegated child run? Measure both before build 372's group kill is trusted on Linux.

## Outcome

A table per measurement. Any claim that turns out false gets a build ticket or, if it is a decision, a map ticket. Amend `IMPLEMENTATION.md`'s Linux paragraphs and `README.md`'s Linux sentence with what was measured.

Partial, 2026-10-27, by the session. Docker Desktop, `rust:1-bookworm`, bubblewrap 0.8.0, an unprivileged `ci` user. bwrap needed `--security-opt seccomp=unconfined --security-opt apparmor=unconfined` and not `--privileged`.

Measured:

- `--ro-bind SRC DEST` with a missing DEST under `--dev-bind / /` creates DEST on the host as an empty read-only file, and the sandbox reads SRC through it. Map 135's claim holds, and build 366's reap removal of an empty settings file is needed.
- A missing SRC aborts bwrap with "Can't find source path". Builds 364 and 366 were right to give every source a path.
- With the private git directory bound whole and its metadata read-only, `git add` works when `GIT_OBJECT_DIRECTORY` points at a writable directory. With `index` and `index.lock` bound as single files, git cannot take its lock. Build 364's reasoning holds.
- Under `--new-session` the child leads a new session and process group, apart from the launcher's. Map 136's Linux concern holds: build 372's group kill targets the wrong group on Linux.
- With `--unshare-pid`, a `setsid` descendant dies when bwrap's command exits; without it, it survives. Delegation from inside a run is refused on Linux too, through `inside_seatbelt()`'s mount-namespace witness, so `--unshare-pid` would kill no delegated child.
- On a clean home, `~/.claude/plugins` and `~/.claude/skills` did not exist, and compose refused every launch: five `sandbox::tests` and one `curate` test failed, as GitHub's Linux job would. The session fixed it in `src/sandbox.rs`; the home roots are created before the profile is written. After that, `sandbox::tests` passed 15 of 15 and `tests/curate.rs` 23 of 23 in the container.
- One recorded launch under `standard` with a stand-in `claude` started under bwrap, recorded `sandbox_kind: bubblewrap`, and closed with exit 0. The deny paths the stand-in tried were refused.

Open, and the reason this ticket stays claimed: in that launch git did not work inside the sandbox, and the run's private git directory was left unusable afterwards. The cause was not traced. Before anyone trusts a Linux launch, a session should trace it with a provider that only runs `git status` and `git add`, then fix it. The full workspace suite in the container passed apart from the failures above.

Not measured: Claude Code reading `{}`, which needs the real binary in the container.
