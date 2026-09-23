# 92 - The launch in phases, with a record on every early return

Type: build
Status: resolved
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

## Outcome

Four runs. `codex` first, 01a0cfd5-cdda, stopped when the gate refused a command spelling its own
worktree's absolute path; the task text now says to use relative paths. Its resume, 01a0cfda-7012,
found `hydra resume` on `codex` broken under the seatbelt, fixed in `3878399`; the next resume,
01a0cfdb-e5c3, hit `codex`'s usage limit. `cursor-agent` on composer-2.5 then did the refactor,
01a0cfdc-ac67, 10.5M tokens, and a second composer run, 01a0cfec-faa6, 2.0M, made the review's fixes.

`run()` is phases now, `preflight_inputs`, `reserve_run`, `prepare_machine`, `phase_worktree`,
`phase_handoff`, `phase_spawn`, each with a struct for what it produces; every function in
`src/run.rs` but one is under 150 lines, and `phase_spawn` is 163, which the session's Outcome
first got wrong and the third hiring panel caught; and `refuse` takes `RefuseRequest` with no `too_many_arguments`
allow. The Opus review compared the phases with master line by line: same refusals, same order of
side effects, same line 0 and argv, the reservation released on every path. What it sent back:
the refactor had deleted 174 lines of design comments, now restored onto the phases that do the
work; two `?` in the spawn phase still escaped without a record; an error from `refuse()` itself
reached the failure path and printed a second, misleading message; and the agent's test called the
failure writer directly, so it passed with the wiring removed.

G4, measured on master outside a seatbelt: `tests/launch_phases.rs` calls `run::run` with a
directory planted where the gate goes, after the worktree exists, and finds a `never_started`
record that validates. With the handoff error path changed back to a bare `return Err`, it fails.
lib 202, spawn 32, detached 8, delegation 14, mcp 16, record_contract 20, status 20, cleanup 39,
normalize 83, evolve 35, curate 22 passed; fmt and clippy clean. Real launches on `read-only` with
the refactored binary: `claude` haiku 01a0cff3-6576, `cursor-agent` composer-2.5 01a0cff3-8452 and
`agy` gemini-3.8-flash-low 01a0cff3-a299 all answered `pong`; `codex` is out of quota and refused
on `read-only` by map ticket 63.
