# 184 - Stepping in on a seat

Type: build
Status: resolved
Blocked by: 183

## What

The panel of 2026-09-30 (unanimous B): a key that stops the selected seat (after `y`) and puts a
line in the chat's input, unsent, naming the seat and its task, so the human edits it and sends
it to the session. Conditions: the line is never sent on its own (`cursor-agent`: 0 of 20); it
must not wipe what the human already typed (`codex`); without `--session`, only the stop and a
hint. The chat already has `paste(text)` (`src/cockpit/chat.rs` near 413), which writes to the
child's pty.

## Do

Read `src/cockpit/chat.rs` (`paste`, bracketed paste) and `src/stop.rs` first. **Check first:**
does a bracketed paste into the `claude` TUI append to its input box without submitting, and
without clearing existing text? Say how you know (the code, a test with a stub pty that records
bytes, the provider's docs); if it cannot be known without a live TUI, say so and do the rest.

1. `m` on a live seat card (a run whose `parent_run_id` is the session) opens the confirmation
   "stop <short id> and draft a note to the session? y". `y` calls `stop::stop` for that run, then
   pastes, with no trailing newline: `Seat <short id> (<provider> <model>) was stopped by the human.
   Its task was: <first 200 chars of the task>. Instead: ` and moves focus to the chat.
2. Without `--session`, `m` does the stop and shows "tell the session yourself".
3. The bytes written to the pty never include `\r` or `\n` at the end; a test with a stub pty
   proves it.
4. `IMPLEMENTATION.md`.

## Done when

- The check is in the report; each test failed before the change; fmt, clippy, `cargo test
  --lib`, `--test cockpit`, `--test stop` pass.

## Outcome

Resolved 2026-09-30. A first launch pointed at ticket 170 by the session's mistake; that run,
01a0f328-452b, saw 170 was built and stopped without a change. Built by `claude` opus on run
01a0f328-f1e8 under `standard`; the session applied it as written.

`m` on a live seat asks, `y` calls `stop::stop` and the host pastes the note into the chat and
gives it the keys: `Seat <id> (<provider> <model>) was stopped by the human. Its task was: <200
chars>. Instead: `, never with a trailing `\r` or `\n`. The task is line 0's, with the preamble
removed, and every field goes through `one_line`, which turns whitespace and control characters
into spaces, so a newline in a task cannot send the note and an `ESC[201~` cannot end the paste.
With a chat, `m` only asks about that session's seats; without one it stops any seat and says to
tell the session. Refused inside a seatbelt. Nine tests, each failing first; fmt and clippy clean,
`--test cockpit` 61, `--test stop` 9, `--lib` 224 on master.

Not known: whether the `claude` TUI appends a bracketed paste without submitting or clearing;
only a live TUI shows it. The cockpit does not draw while the stop waits, up to 40 s.

