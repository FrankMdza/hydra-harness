# 77 - The gate reads the payload it was compiled for, and says what its raw pass misses

Type: build
Status: open
Blocked by:

## What

Map ticket 68, answers 2, 3 and 4, resolved 2026-09-22. Four changes to `src/policy/gate.py`, each
with a test in `tests/gate.rs` driven against a compiled gate the way the file already does.

1. **H6.** `evaluate` coerces a `tool_input` that is not a JSON object to `{}` at
   `src/policy/gate.py:675`, so a shell call with `tool_input` as a list or a string arrives with
   an empty command and the path check and the floor both read nothing. It is refused instead, as
   `hydra/bad-payload`, the way a non-string command already is at `:782`. A missing `tool_input`
   stays what it is today for tools that take no input; decide from the dialect tables which tools
   those are and say so in a comment.
2. **H5.** `as_tool_call` (called at the top of `evaluate`) recognises a payload by its shape, so
   a `claude` payload that also carries a `toolCall` object, or `hook_event_name:
   "beforeReadFile"`, is evaluated as the other provider's call and the real `tool_name` is never
   checked. The gate knows its compiled dialect (`HYDRA_CONFIG["dialect"]`). It reads the payload
   in that dialect only, and a payload that carries another dialect's envelope is refused with a
   `hydra/` code, the same reasoning as the argv check at `:858`. Every existing test that feeds
   one dialect's payload to another dialect's gate has to be read: if one does that on purpose,
   say which and why in the Outcome.
3. **The floor on `network_read`.** The floor's Slack rules run only when `capability ==
   "shell"` (`:831`), so `curl` to a Slack API method is refused and `WebFetch` to the same URL is
   allowed. The URL field of every fetch tool in the dialect tables goes through the same Slack
   check under `network_read`. Only the Slack rules; the rest of the floor is shell patterns and
   stays shell-only. `policies/floor.json` is protected config #1: if the change needs that file
   edited, stop and say so in the Outcome rather than editing it.
4. **H1 to H3, the claim.** The raw pass is a byte-substring test on the case-folded command. The
   docstring above it (`:285` to `:301`) says the corpus, the store, the snapshots, the spawn
   directory and the per-run config files are covered. They are covered when the path is written
   out in one piece. A path split by an adjacent quote inside an interpreter token, assembled by
   string concatenation inside `python3 -c`, or reached through `worktrees/..` is not. Narrow the
   docstring to that. Add one sentence to `SECURITY.md` under the "does not interpret shell" limit
   naming the assembled path as part of it. Pin each of the three as an open residual in
   `tests/gate.rs` beside the existing pin near line 1766, same comment convention: "if this now
   denies, the residual is closed and this comment is stale". The session will hand you the
   three spellings in the launch text.

## Done when

The four items above, each with a test that fails before the change and passes after it
(items 1 to 3) or pins today's behaviour (item 4). `cargo test --release --test gate` green,
`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean. This ticket may edit
`src/policy/gate.py`, `tests/gate.rs`, `SECURITY.md` and nothing under `docs/`, `config/`,
`policies/` or `.scratch/`.

## Outcome
