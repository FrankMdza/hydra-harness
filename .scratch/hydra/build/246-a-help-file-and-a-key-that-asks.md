# 246 - A help file for agents, and a key that types the question into the chat

Type: build
Status: resolved
Blocked by: 240

## What

Map ticket 96, D1 and D2 (resolved 2026-10-03). The cockpit's help has two halves: the static key
view build 240 adds, and a file in the repository a session's own model can answer from. A key in
the help view types a fixed question into the chat and leaves it unsent, as `m` leaves its note.

## Do

1. `COCKPIT.md` at the repository root, for a model to read and answer a person from: the two
   modes (gated with a policy, ungated copilot) and what each can launch; the keys, generated from
   or checked against the one table build 240 introduced so they cannot drift; how to scroll, stop
   a seat, read a run; what a `hydra/` refusal code means and which ones a newcomer meets first
   (`out-of-scope`, `protected-path`, `capability`, `seatbelt-delegation`). Plain sentences a
   person could hear read aloud: no table wider than three columns, nothing past 300 lines. It is
   not `AGENTS.md` and nothing loads it automatically.
2. In the help view, `a` (or the first free key; say which) types into the chat, unsent:
   `Read COCKPIT.md in this repository and tell me how to <blank>`, with the cursor at the blank.
   With no `--session` the key is not offered.
3. A test that every key in the table appears in `COCKPIT.md`, so a new key without a sentence
   fails.
4. `README.md` names the file in one sentence; `IMPLEMENTATION.md` in one.

## Done when

- The two tests pass (the key types the line into a synthetic chat buffer unsent; every key is
  documented). fmt, clippy over the workspace and `cargo test` pass.

## Outcome

Resolved 2026-10-03 by run `01a101ae-3c68` (`cursor-agent` composer-2.5, `standard`, 2.38M
tokens), applied by the session with two changes.

Built: `COCKPIT.md` at the repository root, 135 lines, for a model to answer a person from: the
modes, every key in three-column tables, scrolling, stopping, reading a run, the four refusals a
newcomer meets first. With `--session`, `a` in the help view types `Read COCKPIT.md in this
repository and tell me how to ` into the chat, unsent; the host settles it right after the key
(`settle_stops`, the path `m`'s note takes). Tests:
`with_a_session_a_in_help_types_cockpit_question_into_chat_unsent` in `tests/cockpit.rs` and
`every_help_table_key_appears_in_cockpit_md` in `src/cockpit/keys.rs`.

Changed by the session: (1) `COCKPIT.md` described the modes wrongly, the confusion map 96 set
out to end. It called `--session` the gated session and the cockpit without `--session` the
ungated copilot. The session rewrote that section as three ways (gated: `--session` with
`--policy`; ungated copilot: `--session` without; viewer: no `--session`) and the refusals
section, which also carried em dashes and an old description of `seatbelt-delegation`. (2) The
doc test checked `text.contains(label)`, which is true for `k` in any paragraph, and did not fail
with the `k` row deleted. It now requires a table row starting `| <keys> |`; with the `k` row
deleted it fails, restored it passes. `--test cockpit` 120 passed and 1 ignored, `--lib` 235;
fmt and clippy clean.

Not checked: a model answering from the file in a real session; the human's next session.
