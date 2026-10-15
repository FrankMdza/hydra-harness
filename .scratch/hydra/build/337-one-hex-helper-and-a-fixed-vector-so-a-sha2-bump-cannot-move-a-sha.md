# 337 - One hex helper and a fixed vector, so a `sha2` bump cannot move a sha

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 130's answer, question 8. Dependabot's PR 4 bumps
`sha2` from 0.10.9 to 0.11.0 and fails to compile: `LowerHex` is no longer implemented for the
digest array, and the workspace formats digests with `format!("{:x}", Sha256::digest(..))` at
eleven sites: `src/export.rs:42`, `src/export_preference.rs:74`, `src/sandbox.rs:100` and `306`,
`src/mcp.rs:123` and `621`, `src/spawn_config.rs:138`, `src/rungs/evolve.rs:1227`,
`src/policy/mod.rs:605` and `1261`, `crates/grader/src/sheet.rs:201`. Two of them are the sandbox
profile sha and the hook config sha that every `run_start` carries and that `GateMark::of`
compares; a hex string that came out different would turn every old record `Foreign`. Research 129
called the fix "a few lines in the grader"; it is a workspace change.

## What

1. One function, `hex_lower(bytes: &[u8]) -> String`, in the crate the others already depend on
   (`crates/records` if `hydra` and `hydra-grader` both use it; say which), with a doc comment
   naming this ticket. Every one of the eleven sites calls it. `crates/grader/src/judge.rs` already
   has a private `hex`; it moves.
2. A fixed-vector test beside it: the sha256 of `b"hydra"` and of the empty input as lowercase hex,
   byte for byte, the two strings written in the test as literals computed once from today's
   `sha2` 0.10 and never from the function under test.
3. A second test: `gate_sha` and the sandbox profile sha of one fixture spawn equal the strings the
   fixtures under `tests/fixtures/records/` already carry, so the walk's `Todays` mark survives the
   bump.
4. Leave `sha2 = "0.10"` pinned; the bump is Dependabot's PR and the human merges it when its
   rebase goes green. Do not edit `Cargo.toml` or `Cargo.lock` for this ticket.

## Do

1. The helper and the eleven call sites.
2. The two tests.
3. `IMPLEMENTATION.md`, one sentence where the record's shas are described.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green; `hydra doctor
--walk` over this machine's records with no new `Foreign`.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.

## Outcome

Built `hex_lower(bytes: &[u8]) -> String` twice, as `hydra::hex::hex_lower` in `src/hex.rs` and
`hydra_records::hex::hex_lower` in `crates/records/src/hex.rs`. The eleven sites call it, and so do
four sites the ticket did not list, in `tests/gate.rs` (41, 144) and `tests/sandbox_copilot.rs`
(97, 193), which used `format!("{:x}", ..)` too and would have stopped compiling on the bump.
`judge.rs`'s private `hex` is gone; its one caller uses the records copy. `sha2 = "0.10"`,
`Cargo.toml` and `Cargo.lock` are untouched.

Tests: `hex::tests::the_sha256_of_two_fixed_inputs_is_the_same_string_after_any_sha2_bump`
in both crates, with the sha256 of `b"hydra"` (`517ffce8...2fc1a549`) and of empty input
(`e3b0c442...7852b855`) as literals taken from `shasum -a 256`, plus `[0x00, 0x0f, 0xa0, 0xff]`
for leading zeros. `sandbox::tests::gate_sha_and_profile_sha_are_the_system_tools_hex_of_the_files_written`
writes a `standard` gate and a profile for one spawn and checks that `policy::gate_sha` and
`write_profile`'s return equal what `sha256sum`, or `shasum -a 256` where that is missing, prints
for the files on disk.

Measured: 1437 tests run, 1437 passed, 4 skipped under `cargo nextest run --release --workspace`,
up from 1434 by the three above. `hydra doctor --walk` was not run: it reads `~/hydra/runs/`, which
is outside this run's worktree. The session should run it before merging Dependabot's PR 4.

What was false:

- "The crate the others already depend on" does not exist for the `hydra` binary crate. Its
  `Cargo.toml` names no workspace crate, and this ticket forbade editing it, so the helper lives
  twice with the same fixed vectors. A `#[path]` include of the records file would keep one body
  but breaks `cargo package` for `hydra`, which is publishable. Adding `hydra-records` as a path
  dependency would make it one function; that is a one-line `Cargo.toml` change for a later
  ticket.
- The fixtures under `tests/fixtures/records/` carry no `gate_sha` at all, and their `sandbox` is
  the placeholder `5b5b...`, so item 3 could not compare against them. The test pins the format
  against the system's sha tool instead, which shares no code with `sha2` or `hex_lower`. A
  literal of today's `gate_sha` would also have broken on every edit to `policies/` or `gate.py`.
- The eleven are not every `{:x}` on a digest in the workspace; the four test sites above were
  missed. The many `{byte:02x}` loops elsewhere (`records.rs`, `collect.rs`, `monitor.rs`,
  `baseline.rs`, `marker.rs`, `otel`, `apprentice`, `puppets`) already format bytes and survive
  the bump unchanged; they were left alone.

Measured by the session on the Mac after applying, 2026-10-15, with this build's binary: `hydra
doctor --walk` over 963 records reports no `Foreign` mark, so the hex helper writes the same string
the formatter did. Dependabot's PR 4 rebases on this commit; the human merges it when its checks go
green, and runs the walk once more after the bump.
