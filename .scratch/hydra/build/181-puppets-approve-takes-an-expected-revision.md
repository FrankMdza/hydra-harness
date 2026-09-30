# 181 - puppets approve takes an expected revision

Type: build
Status: claimed
Blocked by:

## What

The panel of 2026-09-30 (unanimous) lets the cockpit run `puppets.py approve` as a child after the
human confirms, with one condition from `codex`: the approval must be bound to the revision the
human saw, so a plan or table that changed between drawing and pressing `y` is not approved by
mistake. `cmd_approve` (`integrations/puppets/puppets.py` near 1768) writes the goal's current
`state.revision` into the event and takes no expected revision.

## Do

Tests first in `integrations/puppets/tests/`, each shown failing before the change. Stdlib only.

1. `puppets.py approve ... --expect-revision N`: when given and the goal's current revision is not
   `N`, print one line naming both revisions to stderr, write nothing, and exit 2. Without the
   flag, nothing changes.
2. A read-only `puppets.py pending <goal> --json` that prints what waits for an approval, one
   object per item: `kind` (`plan`, `table`, `launch`, `diff`, `degraded`), `ref` (task id or
   null), `revision`, and the exact argv that approves it, `--expect-revision` included. It reads
   the same `fold` the helper's own refusals read, so it can never list something the helper
   would refuse or miss something it would wait for. Say how you checked that.
3. `integrations/puppets/README.md`: both.

## Done when

- Each test failed before the change; the puppets suite passes under `.venv/bin/python` if the
  worktree has it, else `python3`.
