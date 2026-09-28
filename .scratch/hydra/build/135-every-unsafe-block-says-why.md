# 135 - Every unsafe block says why it is sound, and clippy holds it there

Type: build
Status: resolved
Blocked by:

## What

A review on 2026-09-28 against the Rust Book, the Rustonomicon and the API Guidelines found that
`Cargo.toml` has no `[lints]` table and the repo has no `clippy.toml`, and that eight `unsafe`
blocks carry no `// SAFETY:` comment: `src/pty.rs` near lines 216, 226, 343, 348, 400 and 404,
and `src/procinfo.rs` near 29 and 51. `src/worktree.rs` from about 610 to 671 is the house
example of a block that states its invariant. Line numbers are from `67c25f1`; find the blocks
by `grep -n unsafe`, not by the number.

The work runs test first: the lint goes in, clippy must fail on those blocks, and only then do
the comments go in.

## Do

1. Add to `Cargo.toml`:
   ```toml
   [lints.clippy]
   undocumented_unsafe_blocks = "warn"
   ```
   Run `cargo clippy --all-targets -- -D warnings` and confirm it fails, naming each block. Put
   the failing output, trimmed to the lint lines, in your report. If it fails on blocks other
   than the eight above, list them too; they are in scope.
2. Give each flagged block a `// SAFETY:` comment on the line above that says which invariant
   makes the call sound: the fd is open and owned by what, the pointer is valid for how long,
   the buffer is how big. Read the surrounding code to find it. Where you cannot state an
   invariant because the code does not guarantee one, do not invent it: leave the comment as
   `// SAFETY: unclear, see report` and explain in the report.
3. `src/pty.rs` near line 67 stores a raw `c_int` that is valid only while `_master` lives.
   Do not restructure it. Say in the report whether a `BorrowedFd` or an `OwnedFd` field would
   remove the hazard, in two or three sentences.
4. The crate doc at `src/lib.rs` line 3 says the router launches two providers. It launches four
   (`claude`, `cursor-agent`, `agy`, `codex`). Fix that sentence and nothing else in the file.
5. Clean `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --lib`.

No behaviour changes. A diff that touches anything but comments, the lint table and the doc line
is out of scope.

## Outcome

Run `01a0e70c-f8cc` (cursor-agent, composer-2.5, `standard`), applied as written. With the lint
alone, clippy failed on 11 library blocks, not the 8 the ticket named: `src/env.rs:193` and two in
`src/worktree.rs` (`fstat` near 636, `CStr::from_ptr` near 664) were also bare. Once the library
was clean, `--all-targets` surfaced six more in `tests/detached.rs`. Every one now has a
`// SAFETY:` line naming the fd or pointer and why it is live; none is marked unclear. `src/lib.rs`
names four providers. On `src/pty.rs`'s stored fd the run judged an `OwnedFd` field wrong (a second
owner beside `_master`, double close) and a per-call `BorrowedFd` from `_master` the right shape,
but an API change this ticket forbade; it stays a documented raw fd. fmt and clippy clean in the
run and on master; `cargo test --lib` shows only the three known worktree failures in the run.
