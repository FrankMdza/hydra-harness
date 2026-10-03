# 246 - A help file for agents, and a key that types the question into the chat

Type: build
Status: open
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
