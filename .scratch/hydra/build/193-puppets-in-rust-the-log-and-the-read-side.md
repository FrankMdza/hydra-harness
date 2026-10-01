# 193 - puppets in Rust: the workspace, the log, and the read side

Type: build
Status: resolved
Blocked by:

## What

Map ticket 90 (2026-09-30): the integrations move to Rust as crates of a Cargo workspace,
`puppets` first, each proved by differential runs against the Python before the Python goes. This
is the first of four tickets for `puppets` (193 read side, 194 writes without launches, 195
launches, 196 cutover). `integrations/puppets/puppets.py` is 2,599 lines, with `sheet.py`,
`models.py`, `aliases.json`, `vendors.json`, `priors.json` and `prompts/` beside it.

## Do

Read `integrations/puppets/README.md` and `puppets.py` first: `read_log`, `fold` and `GoalState`,
`_locked_update`, `UnterminatedLog`, `cmd_repair`, and the read-only commands.

1. **The workspace.** The root `Cargo.toml` gains `[workspace]` with `members = ["crates/*"]`; the
   `hydra` package stays at the root, unchanged, and depends on no member. A new crate
   `crates/puppets` (package `hydra-puppets`, binary `hydra-puppets`). Say how `cargo test` at the
   root and `cargo test --workspace` behave after it.
2. **The log layer** in the crate's library: reading a goal log (only newline-terminated lines
   count, a bad line is skipped as `read_log` does), the fold into a goal state with every field
   the read commands use, the lock-held update, and `repair`. Same file paths and environment
   (`HYDRA_PUPPETS_HOME`, `HYDRA_ROOT`) as the Python.
3. **The read commands** in the binary, with the same arguments and the same output as the
   Python: `show`, `board`, `pending --json`, `queue`, `repair` (dry run and `--yes`).
4. **Differential proof.** A test in `crates/puppets/tests/` that runs `python3
   integrations/puppets/puppets.py <cmd>` and `hydra-puppets <cmd>` on the same inputs and
   requires identical stdout, stderr and exit code: every goal log the Python tests build (reuse
   their fixtures or write equivalent ones into a temp home), plus edge logs (unterminated tail,
   CR tail, bad interior line, empty). Where output carries a path or a time, say so and normalize
   only that. A second test, ignored by default and run by the session, walks the real
   `~/hydra/puppets/*.jsonl` read-only.
5. Nothing deletes or changes the Python in this ticket. `IMPLEMENTATION.md`: a short section on
   the workspace and the crate.

## Done when

- fmt and clippy clean over the workspace (`cargo clippy --workspace --all-targets -- -D
  warnings`); `cargo test --lib`, `cargo test -p hydra-puppets` pass; the differential test
  passes on every input; the report lists each command and input compared, and any output that
  needed normalizing and why.

## Outcome

Resolved 2026-09-30. Run `01a0f485` (claude opus, `standard`), diff read whole by the session and
applied to master.

Built: `[workspace]` in the root `Cargo.toml` with `members = ["crates/*"]`, and `crates/puppets`
(package and binary `hydra-puppets`): `json.rs` (a reader and writer that match Python's
`json.loads` and `json.dumps`), `py.rs` (the Python operations the fold applies to log values),
`log.rs` (`read_log`, `locked_update` under one `flock` held across the read and the append,
`repair`), `fold.rs`, `read.rs` and the clap CLI with `show`, `board`, `pending --json`, `queue`
and `repair [--yes]`. `hydra` is the only default member, so `cargo test` at the root tests
`hydra` alone and `cargo test --workspace` adds the crate. No `serde_json` in the crate: its
`preserve_order` feature would reach `hydra` in a workspace build.

Measured on master: `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets -- -D
warnings` clean; `cargo test --lib` 224 passed; `cargo test -p hydra-puppets` 10 unit tests and
the differential test, 714 command runs (119 inputs by six commands: 102 logs harvested from the
Python tests, 17 edge logs) with identical stdout, stderr and exit code. The run broke one word of
`board` to check the test: 111 of 714 failed. The ignored real-log test passed too, but
`~/hydra/puppets` holds one goal log, so that is five comparisons and not much evidence.

Normalized, and only these: `pending`'s `argv[0]` (`sys.executable` against `python3`), the
timestamp in the torn file's name, and a Python traceback, where the Rust must exit 1 with the
same stdout and name the same exception class (16 runs).

What the ticket assumed that is false: "a bad line is skipped as `read_log` does". `read_log`
skips only a bad last line and raises on a bad or blank interior one; `_locked_update` skips any.
The Rust copies both. Whether that should change is map ticket 91.

Left open: a lone UTF-16 surrogate in a string is readable by Python and exits 1 in Rust. Values
of a type `puppets.py` never writes are compared only in the 17 edge logs. With invalid UTF-8 in
the file, the Python writer raises `UnicodeDecodeError` where `locked_update` decodes line by line
and can return `UnterminatedLog` instead; no command of this ticket writes, so 194 should pin it.
`pending`'s `argv` still names `python3 integrations/puppets/puppets.py approve`; 196 changes it.
The run killed every `cat` of the user with `pkill -x cat` after one of its commands hung; later
tasks say not to.
