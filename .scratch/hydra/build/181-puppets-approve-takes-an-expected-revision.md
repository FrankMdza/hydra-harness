# 181 - puppets approve takes an expected revision

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. Built by `cursor-agent` grok-4.7-high on run 01a0f31d-e264 under `standard`.
The session applied it as written. `approve --expect-revision N` refuses before any other check
when the goal moved on (stderr names both revisions, exit 2, nothing written). `pending <goal>
--json` prints `kind`, `ref`, `revision` and the exact approving argv, built from one `fold` and
the helper's own `launch_refusal`, `approval_available` and `resolve_panel_seats`, so it lists
what `launch`, `tasks`, `verdict` and `choose --judge` would wait for and nothing they would
refuse for another reason. A degraded item's argv carries the `<substitute-provider[:model]>`
placeholder, which `parse_degraded_substitution` now rejects, so it writes nothing until the human
names a substitute. Tests in `test_approve_revision.py`; the puppets suite passes, 160 tests.
Four tests were added after the run's failing-first pass and never ran against the old helper.

