# 77 - The gate reads the payload it was compiled for, and says what its raw pass misses

Type: build
Status: resolved
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

All four items built as the ticket asked, with the dialect fix shaped as the research proposed:
`toolCall` read only on `agy`, `beforeReadFile` only on `cursor`, anything else `hydra/wrong-envelope`.
Tests in `tests/gate.rs`: a non-object `tool_input` refused, an `agy` `toolCall.args` that is a
string or list refused, a missing `tool_input` still allowed for `Task` and an MCP tool, each gate
refusing each foreign envelope, the Slack floor on a `WebFetch` URL and not on a search query or a
prompt field, and the seven pinned spellings with controls. The ticket assumed `policies/floor.json`
would need editing; it did not. It also assumed `tool_input` was the only field coerced, and that
was false: `agy`'s `toolCall.args` was coerced the same way one function earlier, found by the Opus
review. The first version walked every string in `tool_input` for Slack methods, which refused a
search query that mentioned one; narrowed to the URL field. `hydra/wrong-envelope` was missing from
`normalize::GATE_CODES` and the lib test written for that caught it on master. Measured on master
with 78 applied: gate 90, spawn 30, normalize 81, outcome 42, show 3, lib 193 passed; clippy and
fmt clean. Built by `cursor-agent` on Composer 2.5 under `standard`, run 01a0cc2a-d2a1, 1.9M tokens;
an Opus subagent fixed the review's six findings in the worktree.

A second reader, `codex` under `standard` (run 01a0cc4a-ad67, 973k tokens, the review map 68's
answer asked for), found three more, none a widened permission: an explicit `tool_input: null` was
read as absent, a foreign `toolCall` key was refused only when it held an object, and two MCP pins
asked a gate that did not protect their target, so they would have passed with the hole closed.
Fixed by an Opus subagent: key presence decides both, and `replay.rs` now omits `tool_input` when a
record's arguments are null so the replay does not report refusals the live gate never made. The
review is `.scratch/hydra/research/68-review-of-the-fixes-codex.md`. Measured: gate 96, lib 196, and
the corpus walks over the real 321 records green.

