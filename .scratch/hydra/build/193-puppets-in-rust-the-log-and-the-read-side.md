# 193 - puppets in Rust: the workspace, the log, and the read side

Type: build
Status: claimed
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
