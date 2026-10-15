# 337 - One hex helper and a fixed vector, so a `sha2` bump cannot move a sha

Type: build
Status: open
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
