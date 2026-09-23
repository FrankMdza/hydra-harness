# 50 - The binary is not relocatable and nothing says so at runtime

Type: build
Status: resolved
Blocked by:

## What

`policies/` and `config/` resolve through `env!("CARGO_MANIFEST_DIR")`, baked in at compile
time. `strings target/release/hydra | grep -c <checkout path>` returns 3. Copy the binary
anywhere and move or rename the source directory, and it fails on a path nobody told the user
about. The README now says so; the binary does not.

Make the resolution explicit and checkable: keep the compile-time path as the default, let an
environment variable or a flag override it, and have the launch refuse with a sentence naming
the missing directory rather than failing deeper in. `hydra status` should report where it read
the policy set from, beside the root it already prints. Found by the cold-clone review,
`docs/reviews/2026-09-14-first-ten-minutes.md`.

## Outcome

`paths::hydra_home` is the one place that answers where `policies/` and `config/` live:
`HYDRA_HOME` when it is set, the compile-time `CARGO_MANIFEST_DIR` when it is not. Five call sites
go through it, `PolicySet::discover`, `preamble::path`, `mcp::path`, `export::allowlist_path` and
the config directory inside `denied_write_paths`. `ensure_shipped_config` runs before any policy
file is opened and refuses with a sentence naming the directory it looked in and the variable that
moves it. `hydra status` prints `policies <path>` under the `root` line it already printed.

Measured: `strings target/release/hydra | grep -c /Users/USER/hydra` went from 3 to 1, the one
left being the compile-time default inside `hydra_home` itself. `HYDRA_HOME=/nonexistent hydra
status` answers `cannot find policies at /nonexistent/policies: set HYDRA_HOME to the directory
that contains policies/ and config/`. `HYDRA_HOME=~/hydra hydra status` from `/tmp` works. Unset,
nothing changed.

**Two things found while reviewing the diff.** The agent's unit test set `HYDRA_HOME` on the
process and restored it, which races: the crate's unit tests share one binary and run in parallel,
and `PolicySet::discover` calls `hydra_home` from tests of its own, so the mutation would have
broken those at random under a plain `cargo test`. `hydra_home_from` takes the value as an argument
and the test drives that, which is the same split `src/containment.rs` uses and for the same reason.
An empty value now reads as unset, because a variable somebody exported and never set would
otherwise resolve to the filesystem root.

The agent also reported `cargo clippy -- -D warnings` failing on a `too_many_arguments` in
`src/normalize/mod.rs` and called it pre-existing. It was two hours old and mine: build ticket 49
took `ReplaySpawn::of` from seven parameters to nine. **The CI has never run, and it runs clippy
with `-D warnings`, so the first push would have gone red.** The six fields the three adapters read
off a record travel as one `Recorded` struct now, which takes it to four. Seven more warnings across
the test targets went with it, and `cargo clippy --all-targets` and `cargo fmt --check` are both
clean for the first time.

Built by `cursor-agent` on `composer-2.5` under `standard`, run `01a0a59c-c395`, 9 files and 145
insertions, 1.3M tokens.
