# 86 - A cockpit of Hydra's own, wrapped around the vendor's terminal

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-30, by the session, after the human read map ticket 85's answer and asked for
the TUI he had described.

## Question

Map ticket 85 chose the smallest step, a Herdr pane that loops `puppets.py board`, and left a
TUI of Hydra's own for later, with Textual over Ratatui if it came. The human read that and asked
for the cockpit he described on 2026-09-29: the chat of the orchestrating session in one panel,
the task table beside it, a kanban of agents beside that, drill-down into an agent, approvals,
and the mascot growing a head per agent.

His objection to "the cockpit goes beside the chat" was the right one: Hydra already wraps the
vendor's terminal. `src/pty.rs` spawns every provider on a pty Hydra owns and tees what comes back
into the record, and `hydra run --interactive` does the same with the human at the keyboard. A
cockpit that hosts that pty in one of its panels is the same wrapper with a screen. Map decision
D3 holds: Hydra draws nothing inside the vendor's rectangle.

Does Hydra build its own cockpit, in what, and what goes in the first cut?

## Evidence

- `src/pty.rs`: "Spawning a provider on a pty Hydra owns, and teeing what comes back."
  `portable-pty` is already a dependency.
- `src/interactive.rs`: the interactive launch and its harvest, the seats launcher over
  `seats.sock`, and the stderr line for a seat that dies.
- The live panel test of 2026-09-30 (session `01a0f05b`): a `panel` request launched three
  recorded seats, and a `delegate` outside every shape waited for `hydra approve` typed in
  another pane and then launched. That is the approval flow a cockpit would host.
- `integrations/puppets/puppets.py board` prints the task table; `hydra status` lists live and
  ended runs; `hydra show` gives a run's result; `integrations/herdr/bin/hydra-mascot` holds the
  mascot's states and wakes a head per live pane.
- Research 85: Herdr draws no table and takes no cell click. Textual against Ratatui: Textual has
  the mouse and modals built in; Ratatui needs the hit testing by hand, and is Rust, like Hydra.

## Answer

Resolved 2026-09-30, by the session, under the human's delegation, at the human's request. It
amends map ticket 85: step 2 is not a Textual board, it is a cockpit of Hydra's own.

**`hydra cockpit`, a subcommand in Rust with Ratatui, built in two cuts.**

Cut one, build ticket 150: the cockpit beside the chat. A read-only screen with the task table
from the goal log, a kanban of runs by state from `hydra status` and the records, drill-down into
a run with its result text and last record lines, keyboard and mouse to move and expand, a
refresh every two seconds, and one write: the approval of a waiting seat request, typed by the
human in the cockpit, which calls the same code as `hydra approve`. It runs outside any seatbelt
as a human's process, like `hydra approve` does. It needs nothing running to exist (D10): open it
and it reads files; close it and nothing changes.

Cut two, build ticket 151: the chat inside. The cockpit launches `hydra run --interactive` on a
pty it owns, as `pty.rs` already does, and paints that pty in the left panel with the terminal
widget of the Ratatui ecosystem (`tui-term` or equivalent, to be measured), passing keys and
resizes through. The vendor keeps its rectangle. This is the fragile half, which is why it is
second: a wrong resize or a mouse event that does not reach the child breaks the chat, and the
cut-one cockpit is still useful inside Herdr or tmux while that is being made right.

Out of both cuts: a policy editor that writes. Protected config stays the human's file edit. The
mascot with N heads is art, one head per live seat, and joins cut one only if the existing sprites
cover it; a fourth head is a later ticket.

Ratatui over Textual because the cockpit hosts a pty that Hydra's Rust already owns, and one
binary is what Hydra ships. The cost of hit testing by hand is paid once in cut one.

Reversed by: cut one taking more than four sessions, which would send the table and kanban back
to Textual and leave only the pty host in Rust.
