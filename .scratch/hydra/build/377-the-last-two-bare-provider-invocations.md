# 377 - The last two bare provider invocations run inside the run's wrapper

Type: build
Status: claimed
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
