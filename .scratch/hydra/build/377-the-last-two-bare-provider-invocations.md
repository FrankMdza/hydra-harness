# 377 - The last two bare provider invocations run inside the run's wrapper

Type: build
Status: resolved
Blocked by: 375

## Objective

Build 375 put `claude`'s skill probe and `agy`'s `/skills` and `/hooks` inside the run's own profile, through `provider::probe_command`. Its builder found two more provider invocations that a launch makes outside any wrapper:

- `cursor::preflight` in `src/provider/cursor.rs` runs `cursor-agent mcp enable`, `disable` and `list` through `mcp_command`, a plain `std::process::Command`, from `src/run/mod.rs`. `list` asks about the servers the worktree offers, so a server named in a committed project config may start under no wrapper. Whether `cursor-agent mcp list` starts a server is not measured; the fix does not depend on it.
- A gated interactive `claude` session in `src/interactive.rs` calls `inject_skills`, which probes with no profile, and only then calls `write_interactive_spawn_config`. Build 370 measured on macOS that the probe argv runs a committed `.claude/settings.local.json`'s `SessionStart` and `UserPromptSubmit` hooks.

## Scope

- `mcp_command` goes through `provider::probe_command` with the run's profile. In `src/run/mod.rs`, check that the profile exists by the time `cursor::preflight` runs; if it does not, move the call or the write, the way 375 did, and say which in the Outcome.
- `src/interactive.rs` uses `link_skills`, then writes the spawn config, then `probe_skills` with the profile. Decide whether the probe runs under the profile before or after `narrow_for_copilot`, and say why in the Outcome. The narrowed one is what the session itself runs under.
- After this, `inject_skills` with no profile may have no caller outside tests. If so, remove it or say why it stays.
- A test that fails before the change, for at least the interactive path, in the shape of `tests/launch_phases.rs::the_probe_runs_inside_the_runs_wrapper`: a stub on `PATH` that writes into the corpus from the probe argv. It must skip inside a seatbelt like the others. If the interactive path cannot be driven without a terminal, test the ordering through the function `src/interactive.rs` exposes and say so.
- `SECURITY.md`'s bullet "The provider probes ran unwrapped until build ticket 375" names these two as still bare. Amend it, and `IMPLEMENTATION.md`'s build 375 paragraph.

## Checks

`cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --release --target x86_64-unknown-linux-gnu`, and `cargo nextest run --release --workspace --no-fail-fast`.

## Outcome

Built. `cursor::preflight` takes the run's profile, and `mcp_command` builds `cursor-agent mcp <args>` through `provider::probe_command`. In `src/run/mod.rs` the profile already existed when the pre-flight ran, because 375 put `write_spawn_config` ahead of the probe, so neither the call nor the write moved. `phase_handoff` passes `spawn.sandbox_profile`. `CURSOR_DATA_DIR` is a write carve-out in `rules_of`, so the pre-flight's `disable` still lands.

`finish_policy_launch` in `src/interactive.rs` now runs `link_skills`, `write_interactive_spawn_config`, `narrow_for_copilot`, `probe_skills` with the profile, then the collision check. The probe runs after the narrowing. The narrowed profile is the one the session runs under and the one `run_start.sandbox` hashes. Probing under the wider one would let the committed hooks write under home where the session cannot. A collision refusal now comes after the spawn directory exists, and `abandon_policy_launch` already removes that directory.

`inject_skills` had no caller left outside two unit tests in `src/run/mod.rs`, so I removed it. Its doc paragraphs moved to `link_skills` and `probe_skills`. The tests call those two directly, with `None` as the profile for `codex`, which spawns nothing. The first test keeps its name, because tickets 93, 302 and 305 cite it.

Tests: `tests/launch_phases.rs` gained `the_interactive_probe_runs_inside_the_sessions_wrapper` and `the_cursor_preflight_runs_inside_the_runs_wrapper`. Both drive the built binary with a stub on `PATH` that writes into the worktree and into the corpus. The interactive one runs `hydra run --interactive --policy standard` with stdin null and a stub session that exits at once, so it needs no terminal. It runs on macOS alone, because the gated interactive mode refuses a bubblewrap profile. Both skip inside a seatbelt, and this run is inside one, so neither ran here and the red before the change is not measured. The four checks are green: fmt, clippy, the linux check, and nextest at 1544 passed, 4 skipped.

What the ticket assumed that turned out incomplete: these were not the last two bare provider calls. `Provider::version` runs `<binary> --version` before any worktree exists, and `codex::login_status` runs `codex login status` from the launcher's working directory. Neither has a profile to run under at that point, and neither was measured running a hook or a server. `SECURITY.md` names both.

Measured by the session on 2026-10-28, on macOS outside a seatbelt. `the_cursor_preflight_runs_inside_the_runs_wrapper` passed. `the_interactive_probe_runs_inside_the_sessions_wrapper` failed at first for a reason outside the fix: its root under `/var/folders/...` pushed the seat socket path past `SUN_LEN`, the launch aborted, and `abandon_policy_launch` removed the worktree the test reads. The probe's own write had already been refused with "Operation not permitted". The session gave that test a root under `/tmp`, and then both passed. With the profile forced to `None` at the `cursor::preflight` call and at the interactive `probe_skills` call, both failed. The suite ran 1544 tests, all passed, 4 skipped.

Of the two calls still bare, `claude --version` 2.1.312 ran no hook from a committed `.claude/settings.local.json`, measured in the scratch repository build 370 used. `codex login status` runs from the launcher's working directory, the human's own repository and not a run's worktree, and was not measured for hooks.
