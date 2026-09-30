# 191 - A kanban by agent

Type: build
Status: claimed
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
