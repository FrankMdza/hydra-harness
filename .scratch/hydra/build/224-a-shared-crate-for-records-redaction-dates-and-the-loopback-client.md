# 224 - A shared crate for records, redaction, dates and the loopback client

Type: build
Status: claimed
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
