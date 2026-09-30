# 191 - A kanban by agent

Type: build
Status: resolved
Blocked by:

## What

The human's cockpit redesign asks for "kanban panes with each agent of the table". The middle
column sorts runs by state into four lanes (`src/cockpit/snapshot.rs`, `Lane`). Whether a view by
agent replaces it is undecided, so this adds one beside it without removing anything.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. `k` toggles the middle column between the four state lanes (today, the default) and a view by
   agent: one lane per provider that has a run in the snapshot, in a fixed order (`claude`,
   `cursor-agent`, `agy`, `codex`, then any other), each card showing short id, model, state or
   outcome, and the task id when the goal table names the run.
2. With `--session`, the agent view shows only the session and its seats, as the dragon counts
   them (build 177).
3. Selection, `Enter`, the detail, `m`, `a` and `d` work from either view on the same card. The
   waiting-approval items (seat requests and Puppets steps) stay reachable: in the agent view they
   sit in their own last lane.
4. The mode shows in the column's title; the key hint lists `k`.
5. `IMPLEMENTATION.md`, the cockpit passage; `README.md`, one row in the key table.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f3e3-7d64 under `standard`. The session
applied it as written. `k` toggles the middle column between lanes by state and one lane per
agent (`claude`, `cursor-agent`, `agy`, `codex`, then others by name) with the waiting lane last,
keeping the selected run selected. `Focus::Agent` and focus-generic helpers make `Enter`, click,
wheel, `m`, `a` and `d` work the same in both views. With a watched session the agent lanes show
the session and its seats; the waiting lane stays unfiltered, since a request from another session
still waits on the human. Three tests, each failing first; fmt and clippy clean, `--test cockpit`
74, `--test stop` 13, `--lib` 224 on master. A selection carried across by `k` can sit below a
crowded lane's visible rows until the next arrow key.

