# 92 - The launch in phases, with a record on every early return

Type: build
Status: claimed
Blocked by: 89, 90, 91, 93, 95

## What

Findings G3 and G4 of the second hiring panel, 2026-09-23, raised by `claude`.

G3: `run::run` (`src/run.rs` ~382 to ~1118) is one function of about 737 lines, with three copied
`never_started` blocks (~856, ~915, ~1026), and `refuse` (~1262) takes eleven positional arguments
under `#[allow(clippy::too_many_arguments)]`.

G4, the bug inside it: after `worktree::create` runs (~586), `mcp::write_config(...)?` (~619),
`inject_skills(...)?` (~637) and `write_spawn_config(...)?` (~829) return early with no record, which
breaks the file's own D10 comment that every launch past the checks leaves a record. Only
`reap::orphan_checkouts` (`src/reap.rs` ~675) finds what they leave.

Split the launch into named phases (checks before anything exists; the worktree and spawn
directory; what the run is handed: MCP, skills, spawn config, preamble; the spawn itself), each a
function with a small struct for what it produces. From the moment the worktree exists, any error
writes a `never_started` record through one path, a guard or a single `match` at the top, and the
three copies go. `refuse` takes a struct, the way build ticket 83 did for `run_start`.

This is a refactor with one behaviour change, G4. Nothing else a launch does changes: the same
refusals, the same record lines in the same order, the same argv.

## Done when

- A test that makes one of the three post-worktree steps fail (for example a spawn directory made
  unwritable after the worktree exists, or an MCP document that fails to write) finds a record with
  `never_started` for that run id. It fails before the change.
- The whole suite passes unchanged, `tests/spawn.rs`, `tests/detached.rs`, `tests/delegation.rs`,
  `tests/mcp.rs` and `tests/record_contract.rs` above all.
- No function in `src/run.rs` is over 150 lines, and the `too_many_arguments` allow is gone.
- One real launch per provider on `read-only` with a `pong` task still starts and records. Say the
  run ids in the Outcome.

## Notes for the run

Tests take a scratch `Layout`, never `Layout::discover()`. Ignore
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. Run `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` before you stop. Blocked by 89 to 91, 93 and 95 only so
it starts from a master that holds them; 95 changes how `src/run.rs` calls git at the run end.
