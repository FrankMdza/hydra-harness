# 265 - `delegate` launches `codex`

Type: build
Status: open
Blocked by:

## What

Found by the Fable diagnosis of 2026-10-03 (run `01a1047d-2669`): the `delegate` tool of the MCP
face lists `claude`, `cursor-agent` and `agy` (`src/face.rs`, the `provider` enum near line 248),
written before `codex` was routed. Nothing records a reason to leave it out. Add `codex`, with
whatever `delegation.rs` and the face's launch path need for it (its gate is read from the
worktree like `cursor-agent`'s and `agy`'s, so the same "needs a repository" note applies; it is
refused under `read-only`, so a child asked for `read-only` on `codex` is refused with that
reason). Tests in `tests/face.rs` and `tests/delegation.rs` style, launching nothing.

## Outcome
