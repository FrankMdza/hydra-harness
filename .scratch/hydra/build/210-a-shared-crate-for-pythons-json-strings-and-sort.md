# 210 - A shared crate for Python's JSON, strings and sort

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-01. Run `01a0f6c3` (claude opus 5.5, `standard`, 0.7M tokens), diff read whole
by the session and applied to master as written.

Built: `crates/pyjson` (package `hydra-pyjson`, library only, `anyhow` its one dependency).
`json.rs`, `py.rs`, `pysort.rs`, `printable.rs` and `models.rs` moved there with `git mv`, their
content unchanged but for `models.rs`'s import and its test's path to `aliases.json`. A new
`file.rs` holds `read_text` and `os_error`, moved out of `write.rs` and not duplicated.
`crates/puppets` depends on it by path and exports what it exported before.

Made public that was not: the `py` module's 21 functions (`eq`, `same_key`, `eq_str`, `truthy`,
`hashable`, `type_name`, `str`, `repr`, `ljust`, `pad`, `len`, `head`, `list`, `is_space`,
`strip_str`, `strip`, `int_literal`, `int_value`, `item`, `get`, `less`), each of which puppets
calls, and `file::read_text` and `file::os_error`.

Measured on master: no file under `crates/puppets/tests/fixtures` changed; `cargo test -p
hydra-pyjson` 21; `cargo test -p hydra-puppets` 44 unit, 5 in the binary, 18 differential (44 and
21 are the 65 of before); `cargo test --lib` 224; fmt and clippy clean over the workspace. The
run printed `True` for `true` in the moved `dumps` and 3 of the 18 differential tests failed,
then restored it.

Left open: `IMPLEMENTATION.md` lines near 3254 to 3364 still name the five files as if they were
in `crates/puppets`; the new paragraph says where they live. Whether the public items are the
ones the grader needs is known only when build 212 uses them.

