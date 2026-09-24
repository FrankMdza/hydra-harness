# 118 - What the fourth panel found in `puppets.py`

Type: build
Status: claimed
Blocked by:

## What

The fourth panel's `claude` reader (`docs/reviews/2026-09-24-fourth-panel-claude.md`) read
`integrations/puppets/puppets.py` cold and found the class of problem build tickets 95, 99, 103
and 106 spent a week closing in `src/`, back in the Python that drives the mode. Pointers, by
the review's own numbering, so this ticket does not repeat the mechanisms:

1. Weakness 2, first half: `cmd_review` runs Hunk with a run's worktree as its cwd, outside the
   seatbelt, as the human. Read `src/run.rs` near 389 to 406 for what Hydra's own launch refuses
   there since build 99, and why.
2. Weakness 2, second half: `cmd_plans` copies a file out of a run-writable directory with
   `shutil.copy2`. Read what build 106 did to reap's copy in `src/worktree.rs` near 537 to 597.
3. Weakness 5 and "Master of Puppets, read cold", second and third lines: the `plan` approval
   kind is accepted and checked nowhere; a `diff` approval binds to a task and a revision, not
   to a run id; `append_event` takes no lock and `read_log` raises on a torn last line.
4. Before publishing, item 4: `integrations/puppets/README.md` line 3 says approvals are
   enforced. Say what the CLI checks.

Build ticket 111 is open on the same file and belongs to the session that owns the mode. Take
this ticket after 111 lands, or coordinate on the file.

## Done when

Each of the four has a test in `integrations/puppets/tests/test_puppets.py` that fails before
the change, and the README says what an approval is. `python3 -m pytest integrations/puppets`
green. Nothing under `src/` changes.
