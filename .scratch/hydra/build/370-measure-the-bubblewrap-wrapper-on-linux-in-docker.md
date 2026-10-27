# 370 - Measure the bubblewrap wrapper on a real Linux in Docker

Type: build
Status: open
Blocked by: 366, 367

## Objective

Builds 318a, 318b, 363, 364, 366 and 367 wrote the Linux wrapper from reading bubblewrap and git, and every Outcome says no Linux host ran it. Map 135's panel named four claims nobody measured. This ticket measures them on the Docker Desktop VM of this Mac, with the recipe in the session's memory of build 331: `rust:1-bookworm`, an unprivileged `ci` user, `git archive HEAD` into `/home/ci/hydra-harness`. Add `apt-get install bubblewrap`. bwrap needs user namespaces inside the container, so run it with `--security-opt seccomp=unconfined --security-opt apparmor=unconfined` and, if that is not enough, `--privileged`. Say which was needed.

The session runs this itself, not a `hydra run`. The VM takes 4 GB of the Mac's 8, so no Hydra run may be live while the container compiles.

## Measure

1. `cargo test --release --workspace` as `ci`, with the CI job's environment and its skip list. Then the same run without `--skip sandbox`, to see which sandbox tests pass on Linux with bwrap present.
2. A real launch: `hydra run` with the stub provider `tests/pty_tee.rs` uses, under `standard`, in a scratch repository. Does bwrap start? Read `sandbox_kind` and the witness on the record.
3. Inside that launch, from the stub's shell: write to each deny path and to the worktree, read the corpus, connect to a sibling's socket. Each result goes in a table: path, expected, measured.
4. Map 135's four claims: whether `--dev-bind / /` plus `--ro-bind SRC DEST` creates DEST on the host; whether git's index rename works under the whole-directory bind; whether Claude Code reads a `{}` settings file as absence (skip if Claude Code cannot run in the container, and say so); and the five private-directory attacks against `hydra reap` after build 367.

## Outcome

A table per measurement. Any claim that turns out false gets a build ticket or, if it is a decision, a map ticket. Amend `IMPLEMENTATION.md`'s Linux paragraphs and `README.md`'s Linux sentence with what was measured.
