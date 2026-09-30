# 148 - A board pane beside the session

Type: build
Status: resolved
Blocked by:

## What

Map ticket 85, resolved 2026-09-29. Read its `## Answer` and section 5 of
`.scratch/hydra/research/85-a-cockpit-for-the-orchestrating-session.md` first.

The human wants to see, beside the chat of an orchestrating session, the table of tasks, who each
went to, with which model, and what done means. `integrations/puppets/puppets.py board <goal-id>`
already prints that table and what the human still owes. `hy` in `integrations/herdr/hy.zsh`
already opens two panes beside the current one, the mascot above and the session below. Herdr
draws no table itself; a pane runs a process. So step 1 is a process that prints the board on a
loop, and a third pane that runs it.

## Do

1. `integrations/herdr/bin/hydra-board`, Python, stdlib only: prints the board of one goal, waits
   `--interval` seconds (default 5), clears the screen and prints it again. It takes `--goal
   <id>`; without it, it uses the goal whose log under `HYDRA_PUPPETS_HOME` changed most recently,
   and says which at the top. With no goal log at all it prints one line saying so and keeps
   waiting, so the pane does not die before the session creates a goal. It calls `puppets.py
   board` as a subprocess, or imports the function that prints it if `puppets.py` exposes one;
   say which and why. A `puppets.py` that fails prints the error in the pane and keeps looping.
2. `hy` opens a third pane, to the right of the session pane, running `hydra-board`, with the same
   `--cwd` and `HYDRA_BIN` as the others. The mascot pane stays where it is. Print the three pane
   ids in `hy`'s last line.
3. `integrations/herdr/README.md`: the `hy` paragraph names the board pane and how to pick a goal.

Do not change `puppets.py`'s output. Do not touch `src/`, `config/` or `policies/`. Check
`git log -3 -- integrations/herdr/hy.zsh` first, since build 145 changed it on 2026-09-29.

## Done when

- A test in `integrations/herdr/tests/` drives `hydra-board` once against a scratch
  `HYDRA_PUPPETS_HOME` with two goal logs and checks it picks the newer, and that with no logs it
  prints the waiting line. Another test in `test_hy.py`, with `herdr` and `hydra-tui-in-pane`
  stubbed, checks `hy` splits three panes and the third runs `hydra-board`. Each failed before the
  change.
- `bash -n` and `zsh -n` on `hy.zsh` pass, and the Herdr suite passes under `.venv/bin/python`.
- The report says what the pane looks like on a real Herdr if you could open one, or that you
  could not.

## Outcome

Resolved 2026-09-29. Built by `composer-2.5` on run 01a0f04b-4a85 under `standard`, about 0.85M
tokens. The session applied the diff as written.

`integrations/herdr/bin/hydra-board` prints `puppets.py board` for one goal and redraws it every
five seconds, picking the goal log that changed last under `HYDRA_PUPPETS_HOME` when `--goal` is
not given, and waiting with one line when there is none. It calls `puppets.py` as a subprocess, so
the pane stays a wrapper and a failing board shows its error and keeps looping. `hy` opens it as
a third pane, split to the right of the session pane, and prints the three pane ids.

Tests: `test_hydra_board.py` on a scratch puppets home, and `test_hy.py` with `herdr` stubbed
checks three splits and that the third runs `hydra-board`. Herdr suite 35 of 35 under
`.venv/bin/python`. The session rendered one frame against the real goal log of 2026-09-24 and
it printed the goal, the plans and the task grid.

Not measured: the layout on a real Herdr. Splitting the session pane to the right leaves the board
at the session's height, 30% of the column. If that is too short, the split point moves; it is
one line in `hy.zsh`.
