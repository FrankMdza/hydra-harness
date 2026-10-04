# 224 - A shared crate for records, redaction, dates and the loopback client

Type: build
Status: resolved
Blocked by: 220

## What

Map ticket 95, question C: the apprentice, otel and `local-llm` need four things that live in
`crates/grader`: `records.rs`, `redact.rs`, `isotime.rs` and `judge_http.rs`. They move to a new
crate, as build 210 moved the JSON code to `crates/pyjson`, and nothing the grader prints may
change.

## Do

1. A new crate `crates/records` (package `hydra-records`, library only). Move the four files
   with `git mv`, their unit tests with them; `judge_http.rs` becomes `loopback_http.rs`.
   `crates/grader` depends on it and re-exports nothing new.
2. The client's failure for a status other than 2xx carries the reply's body, which `local-llm`
   prints; the judge's output does not change.
3. No fixture of `crates/grader` or `crates/puppets` changes, and every differential test passes.
4. `IMPLEMENTATION.md`: the workspace passage names the fourth library and why.

## Done when

- No fixture changed; `cargo test --workspace` passes; fmt and clippy clean. The report lists what
  became public.

## Outcome

Built 2026-10-04 by an Opus run (`01a10603`), reviewed and applied by the session.

`crates/records` (package `hydra-records`, library, on `anyhow`, `hydra-pyjson` and `sha2`) holds
`records.rs`, `redact.rs`, `isotime.rs` and `loopback_http.rs` (was `judge_http.rs`), moved with
`git mv`, their unit tests with them. The grader depends on it and re-exports nothing. Public
now: the four modules, and `records::refuse_nan`, which was `pub(crate)`.

Step 2: `Failure::Status` is `Status(u16, Vec<u8>)`, the reply's body, empty when unreadable. One
test, a 404 with a 14-byte body, fails with the body dropped. The judge ignores the body, so its
output does not change; the one cost is that a server that announces a body and never sends it
holds the judge one timeout longer before the same `HTTP {code}` line.

No fixture changed. Suite on master 1,271 passed, 0 failed; fmt and clippy clean. The session
pointed the two port passages of `IMPLEMENTATION.md` that still placed these files under
`crates/grader/src/` at the new crate. The ticket's "fourth library" is the fifth workspace
member, since the root `hydra` package counts.
