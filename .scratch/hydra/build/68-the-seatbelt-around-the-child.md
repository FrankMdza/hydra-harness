# 68 - The macOS seatbelt around the vendor binary denies writes to Hydra's directories under every spelling

Type: build
Status: resolved
Blocked by: 66

## What

Map ticket 62, resolved 2026-09-21. On macOS every recorded launch wraps the vendor binary in
`/usr/bin/sandbox-exec -p <profile> <binary> <argv>`, same user, same `HOME`, so the keychain
and the logins are untouched. Measured 2026-09-21: `deny file-write* (subpath ...)` stops a
direct write, a variable, Python and a symlink with EPERM, a hard link cannot be created from
inside, the directory cannot be renamed, and all four binaries start underneath with `codex`
logged in.

1. `src/sandbox.rs` composes the profile: `(version 1)(allow default)`, then `deny file-write*`
   for every path `policy::denied_write_paths` returns minus `own_write_paths`, and `deny
   file-read*` for the corpus, `layout.runs()`, alone. The profile is written to the spawn
   directory beside the gate so the record can name it, and `run_start` gains `sandbox` with
   the profile's sha, null where no sandbox wrapped the launch. Contract 5.1 is the session's to
   amend; the field is added by type, the way ticket 54 adds a later field.
2. `src/pty.rs` or wherever argv becomes a process: the wrapper goes in front on macOS, and on
   any other platform the launch proceeds unwrapped and `sandbox` is null. `sandbox-exec` missing
   on a Mac refuses the launch in Hydra's words rather than launching bare.
3. The gate changes nothing. It still decides capabilities and writes the record; the kernel is
   what refuses the write.

Tests: `tests/spawn.rs` reads the profile a spawn writes and asserts each denied path is in it
and the worktree is not; a test that runs `sandbox-exec` with the composed profile around `sh`
against a scratch layout, skipped where the binary is absent, asserts the write into the scratch
corpus fails and one into the scratch worktree succeeds. Launch no provider. Do not edit
`docs/`, `SECURITY.md`, `IMPLEMENTATION.md` or `.scratch/`.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test spawn --test gate --test record_contract`. Do not run
the whole suite.

## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c61d-4289 under `standard`, 5.5M
tokens: `src/sandbox.rs` composes the profile, `spawn_config` writes `sandbox.sb` beside the gate
and carries its sha, `pty::spawn` prepends `/usr/bin/sandbox-exec -f <profile>` on macOS, a Mac
without the binary refuses the launch, `run_start` gains `sandbox` validated as sha-or-null, and
two tests in `tests/spawn.rs` read the profile and drive `sandbox-exec` against a scratch layout.
Two things the run found false in the ticket and fixed: `-p` takes profile text and `-f` a file,
and a deny on a non-canonical `/var/folders` path does not enforce, so paths are canonicalised.

Three corrections by the session, each measured. The module skipped a denied parent when it held
an own path, which left sibling worktrees open at the kernel; SBPL takes the last matching rule,
a deny on `worktrees/` followed by an allow on the own tree leaves siblings refused, so every
parent stays denied. Codex's own `--sandbox workspace-write` is a seatbelt and cannot be applied
inside Hydra's unless identical, every command on run 01a0c62d-b98d died with `sandbox_apply:
Operation not permitted`; where Hydra wraps, Codex runs with `danger-full-access` and the kernel
layer is Hydra's. And `cursor-agent` creates its data root under the spawn directory before the
model runs, so its first `mkdir` failed with EPERM on run 01a0c6a5-be8c; the profile allows
writes under `own_read_paths`, which names that root and not the gate beside it.

Measured on real runs after the corrections, one per provider, all `completed`: `claude` haiku,
`cursor-agent` composer, `agy` flash and `codex` each ran `echo seatbelt-hi` under the profile.
On `codex`, run 01a0c6a7-9315, a corpus write spelled as two halves the string floor cannot see
failed with "operation not permitted", a corpus listing failed the same way, and the write in
its own worktree succeeded, which is the whole point of map ticket 62.

One thing the ticket did not foresee and the human decided: the profile is inherited by every
descendant, so a delegated child, Hydra itself, cannot read its parent's record or write its own.
The corpus stays denied and delegation from inside a run is map ticket 64.
