# 130 - An interactive run on `cursor-agent`, `agy` and `codex`, with the gate measured in each TUI

Type: build
Status: open
Blocked by: 129

## What

Map ticket 76-F, resolved 2026-09-24. Read its `## Answer, the rest` first.

`hydra run --interactive` launches `claude` only (`src/main.rs` near line 26, `src/interactive.rs`).
76-F lets an interactive run go to the other three once each one's gate is measured in its own
TUI. Until Hydra can harvest that provider, its interactive record is `run_start` and `run_end`
and says so. Nobody has measured whether any of the three fires its hook in interactive mode the
way it does headless.

One provider per Outcome, in this order: `cursor-agent`, `agy`, `codex`. `codex` is out of
credits until 2026-09-27 23:04 and Cursor premium models until 2026-09-29, so a provider that
cannot be launched when the ticket is taken is left for a second pass and the ticket stays open.

## Do, per provider

1. Read how the headless launch writes that provider's hook config (`src/spawn_config.rs`,
   `src/provider/<name>.rs`) and what its interactive binary expects (`--help`). Write the
   interactive argv beside `claude::interactive_argv`, with the same hook config and seatbelt.
2. Measure, with the human at the TUI: launch it interactively under `standard` and ask it to
   write a file outside the worktree. The gate refusal has to reach the TUI and the `hydra/`
   code has to show. Paste what the TUI printed into `## Outcome`. If the hook does not fire,
   stop for that provider: the answer to 76-F says it is not offered, and the Outcome says why.
3. Its record without a harvest: `run_start`, then a `run_end` whose body says there is no
   harvest for this provider, in the contract's own words (`docs/run-record-contract.md` on a
   harvested run). The derived outcome should not claim `completed` from an empty body; say what
   it derives and why.
4. On a measured gate, add the provider to build 129's list, and lift the refusal in
   `src/main.rs` for that provider only.

## Done when

- Per provider: a test builds its interactive argv and hook config without launching it, and the
  Outcome carries the measured refusal or the reason the provider stays off the list.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` and
  `hydra doctor --walk` pass on master.
