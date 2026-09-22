# 69 - The gate reads the paths out of an `apply_patch` body, so `codex` can write under `standard`

Type: build
Status: resolved
Blocked by:

## What

Build ticket 48 measured it and build ticket 63's first run on `codex` hit it on 2026-09-21: under
`standard` every `apply_patch` is refused `hydra/no-path`, because the row carries no path field
and a scoped `file_write` with nothing to scope is refused by design. The patch body names its
files. `tool_input.command` is the patch text, and each hunk opens with one of `*** Add File:
<path>`, `*** Update File: <path>`, `*** Delete File: <path>`, and `*** Move to: <path>` after an
update. So the gate can scope the write the way it scopes `Write` on `claude`: every path the
body names must be inside an allowed scope, and one outside refuses the whole call with the
path in the reason.

1. `src/policy/gate.py`: a path extractor for the `apply_patch` tool that returns every path the
   headers name, relative paths resolved against `cwd`; the `file_write` scope check runs over
   all of them and refuses on the first outside. A body with no header is `hydra/no-path` as
   today.
2. `src/policy/table.rs`: the `apply_patch` row says how its paths are found, so the replay in
   `src/normalize/codex.rs` derives the same answer; `file_change` items already carry
   `changes[].path`, and the replay uses those.
3. `tests/gate.rs`: an `apply_patch` adding a file inside the worktree is allowed under
   `standard`; one updating a file under `~/hydra/runs/` is refused `protected-path`; one with
   an add inside and a move outside is refused; the skill-root loop stops skipping `codex`.

Do not edit `docs/`, `IMPLEMENTATION.md` or `.scratch/`. Launch no provider.

Verify with `cargo build --release`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` and `cargo test --release --test gate --test normalize --lib codex`. Do not run the
whole suite.

## Outcome

Resolved 2026-09-21. `cursor-agent` on Composer 2.5, run 01a0c61e-e2b6 under `standard`, 2.2M
tokens: `parse_patch_paths` in `gate.py` reads every `*** Add File`, `*** Update File`, `*** Delete
File` and `*** Move to` header out of an `apply_patch` body and the scope check runs over all of
them; `table.rs` marks the row `patch_paths`; `policy/mod.rs` compiles `patch_path_tools` into
the gate config; the replay in `normalize/codex.rs` reads `changes[].path`; three gate tests and
the skill-root loop no longer skips `codex`. The session reaped the worktree before noticing the
`policy/mod.rs` hunk was not among the files it had copied, and recovered it from the dangling
reap commit with `git fsck`; `gate` is 76 of 76 on master with it. Measured the same evening: a
`codex` run under `standard` could not write with `apply_patch` before this and the first run of
build ticket 63 stopped on it. Nothing the ticket assumed turned out false.
