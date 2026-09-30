# 185 - Puppets approvals bind to what was shown, and a review opens nothing

Type: build
Status: claimed
Blocked by: 182

## What

Three findings on `integrations/puppets/puppets.py`, all Python:

1. **A review that opens nothing** (found by build 183). `cmd_review` ends by `execvp` of `hunk show
   <branch>`. The cockpit starts it detached with no terminal, so Hunk will likely fail or hang,
   and the cockpit row stays on `review running`.
2. **A saved argv approves a different thing** (codex review of 181, reproduced with the real fold
   in memory). Run and plan events do not move the goal's revision, so `pending`'s diff argv for
   task `t` run `r1` at revision 2, run after a `run` event replaced it with `r2`, records a diff
   approval for `r2` and exits 0. A replaced chosen plan does the same for `--plan`. Also (build
   182) an approval does not move the revision, so the same approval can be recorded twice.
3. **Check and append are not atomic** (same review). `cmd_approve` checks `--expect-revision`
   against its first fold, then appends under the log lock without checking again, so a writer in
   between leaves a revision-2 approval on a revision-3 goal.

## Do

Tests first in `integrations/puppets/tests/`, each shown failing before the change, with the stubs
the suite already uses for judge launches and Hunk (find them; launch nothing real). Stdlib only.

1. `review ... --no-open`: panel, verdict and findings as now, then print the `hunk show ...`
   command and exit 0 without exec. Without the flag nothing changes.
2. `approve` takes `--expect-run <run_id>` for `--diff` and `--expect-plan <path>` for `--plan`,
   refusing with exit 2 when the task's current run or the chosen plan differs. `pending` puts
   them in the argv it prints. `approve` refuses a duplicate of an approval already in the log at
   the current revision for the same kind, ref and run, exit 3, nothing written.
3. Every `--expect-*` check runs again under the same lock as the append, on a fold read under
   that lock; a mismatch writes nothing.
4. `integrations/puppets/README.md`.

## Done when

- Each test failed before the change; the puppets suite passes.
