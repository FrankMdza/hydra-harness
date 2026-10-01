# 210 - A shared crate for Python's JSON, strings and sort

Type: build
Status: open
Blocked by: 

## What

Map ticket 94: the Python-exact code the grader's port also needs moves out of `crates/puppets`
into `crates/pyjson`, and both crates depend on it. Nothing may change in what `hydra-puppets`
prints: its 18 differential tests compare with fixtures recorded from the deleted Python, and they
are the check.

## Do

1. A new crate `crates/pyjson` (package `hydra-pyjson`, library only, same dependencies policy as
   `crates/puppets`: no `serde_json`). Move `json.rs`, `py.rs`, `pysort.rs`, `printable.rs` and
   `models.rs` there with their unit tests. `models.rs` reads a file through `write::read_text`:
   move or duplicate only that reader, and say which.
2. `crates/puppets` depends on it by path and re-exports nothing it did not export before unless
   a caller needs it. Its binary's output is unchanged: every differential test and unit test
   passes with no fixture touched. `git status crates/puppets/tests/fixtures` must be clean.
3. `py.rs` is private to puppets today (`mod py`). Make public in `pyjson` only what the grader's
   plan names (`.scratch/hydra/research/94-the-grader-port-plan.md`, "What puppets taught") and
   what puppets itself uses.
4. `IMPLEMENTATION.md`: the workspace passage names the third member and why it exists.

## Done when

- No fixture changed; `cargo test -p hydra-puppets` 65 unit (fewer by the ones that moved), 5 in
  the binary, 18 differential; `cargo test -p hydra-pyjson` passes; fmt and clippy clean over the
  workspace; `cargo test --lib` passes. The report lists every item made public.
