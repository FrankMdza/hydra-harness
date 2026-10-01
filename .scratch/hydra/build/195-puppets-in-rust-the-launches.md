# 195 - puppets in Rust: the commands that launch

Type: build
Status: open
Blocked by: 194

## What

Third of four tickets for map 90's `puppets` port: the commands that start runs or other
programs: `plan`, `plans`, `launch`, `wait`, `review` (including `--no-open` and the Hunk comment
pusher), and the judge panel `choose` and `review` start.

## Do

Port them with the same arguments, output, exit codes and log lines. Launching goes through the
same `hydra run` argv the Python builds (compare the argv, never launch a real provider in a
test): the tests put a stub `hydra` and a stub `hunk` on `PATH`, record their argv and environment,
and compare what the Python and the Rust hand them, and the resulting logs. The Python stays.

## Notes from 194

- `crates/puppets/src/write.rs` holds the eight writers. `choose --judge` stops after its three
  refusals with nothing launched; the launch it leaves is `run_judge_panel` and `spawn_judge_run`
  in `cmd_choose`. Start there.
- `sheet.rs` has `rank`, `beta_lower` and `ranked_assignment` only. `build`, `budget` and
  `corpus_records` in `sheet.py` call `hydra show` per run; port them here if a launch command
  reaches them, and say so if none does.
- `models.rs` is ported and nothing calls it yet.
- The differential test's comparator masks appended `ts` values and a generated goal id. For
  launches it will also need run ids and whatever a stub `hydra` prints: use a stub binary named
  by `HYDRA_BIN`, never a real provider.
- Build 204 changes `replace_log` and stdout encoding in the same crate; rebase on it if it has
  landed.

## Done when

- fmt and clippy over the workspace; `cargo test -p hydra-puppets`; every command compared.
