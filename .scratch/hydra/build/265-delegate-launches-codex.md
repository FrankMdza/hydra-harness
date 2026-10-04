# 265 - `delegate` launches `codex`

Type: build
Status: resolved
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

Resolved 2026-10-03 by run `01a104fd-8f5a` (`cursor-agent` cursor-grok-4.6-high, `standard`,
3.08M tokens), applied without changes, its own files only.

Built: the `delegate` schema lists `codex`, and its description says `codex` reads its gate from a
worktree and is refused under `read-only`. The face refuses that pair with the routing reason
before it asks a seatbelt or a launcher (`refuse_unroutable`), and `hydra run`'s
`prepare_machine` now says the same words before it reads the binary's version, so a machine
without `codex` still gets the routing reason. Three tests; two failed against the old code, the
third pins behaviour that already held. The ticket's premise was half off: `Provider::from_str`
already parsed `codex`; the gap was the listed enum and the order of the checks.
