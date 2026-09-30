# 151 - The chat inside the cockpit: the vendor's pty in the left panel

Type: build
Status: resolved
Blocked by: 150

## What

Map ticket 86, cut two. Read its `## Answer`, then `src/cockpit/` as build 150 and 152 left it,
then `src/pty.rs` and `src/interactive.rs`.

The cockpit today runs beside the chat. The human wants the chat inside: `hydra cockpit
--session <provider> [--model M] [--policy P]` launches `hydra run --interactive` for that
provider on a pty the cockpit owns, and paints that pty in the left panel. The vendor keeps its
rectangle (map decision D3: Hydra draws nothing inside it); the cockpit only hosts it, as
Herdr does, and as `pty.rs` already does for every launch.

## Do

1. **Measure the crates first.** `tui-term` (a Ratatui widget over `vt100`) is the known one.
   Say in the report which crate you chose, its version, and what it does not handle. If none
   is usable, stop and say so.
2. **Launch.** With `--session`, spawn `hydra run --interactive --provider ... --policy ...` as
   a child on a pty sized to the left panel, through `portable-pty` as `pty.rs` does. Without
   `--session` the cockpit is what build 150 made. The child is the same launch the human gets
   from `hy`, so the record, the gate, the seatbelt and `seats.sock` are untouched.
3. **Paint.** The left panel shows the child's screen, updated as bytes arrive, with colours.
   A resize of the terminal resizes the panel and sends the new size to the child.
4. **Focus.** One key, `F2`, toggles focus between the chat and the cockpit panels; the status
   line says which has it. With the chat focused every key and mouse event goes to the child
   unchanged, including `q`, `a`, arrows and Enter. With the cockpit focused, build 150's keys
   apply. `Ctrl-q` quits the cockpit from either side, after the child has exited; while the
   child is alive it asks once, and `y` sends the child a hangup and waits for its harvest.
5. **Exit.** When the child exits, the panel shows its last screen and a line with the run id
   and the outcome from its record. The cockpit stays open.
6. **Tests** in `tests/cockpit.rs`, on a stub child (a shell script that prints, reads a line
   and exits, as `pty_tee.rs` does): the panel shows what the child printed; a key with the chat
   focused reaches the child; a key with the cockpit focused does not; a resize reaches the
   child (`stty size` in the stub); the child's exit puts its run id on the line. Nothing here
   launches a provider.
7. `IMPLEMENTATION.md`, the cockpit section: cut two and what stays untested without a human.
   `README.md`: the `--session` form.

## Done when

- The tests above, each failed before the change; `cargo test --test cockpit`, `cargo test
  --lib`, fmt and clippy clean.
- The report names the crate, what is untested on stubs, and what a human should try first on a
  real `hy`-style session inside the cockpit.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f0b0-fc29 under `standard`, about 6.34M
tokens. The session applied the diff as written: `src/cockpit/chat.rs` (the pty child and its
`vt100` screen through `tui-term 0.3.4`), `src/cockpit/host.rs` (which side has the keys, `F2`,
`Ctrl-q` and the hangup), `--session`, one `pub(crate)` on `status.rs` to reuse the harvest of an
abandoned interactive run, six tests on a stub child. Cockpit 19, lib 220, fmt and clippy clean.

Then the session drove the release binary on a pty at 200 by 45 with `--session claude --model
sonnet --policy standard`, a real launch: the child wrote its record (run 01a0f0c3-b058), the
prompt "Reply with the single word pong" typed into the chat panel came back with `pong`,
`/exit` ended the child and the panel showed `run 01a0f0c3-b058: completed; hydra exited 0`, and
`Ctrl-q` quit the cockpit with status 0. So `claude`'s interface draws under `vt100` and its
keys reach it, at least for typing, Enter and `/exit`.

Not tried by a human: `Esc` during an answer, a resize mid-frame, the mouse in the chat, paste,
`F2` back and forth, and the hangup of a live gated session with seats.

Map ticket 86 is built in both cuts. What the human asked for on 2026-09-29 exists:
`hydra cockpit --session claude` is the chat, the table, the kanban, the detail, the approval and
the mascot in one binary.
