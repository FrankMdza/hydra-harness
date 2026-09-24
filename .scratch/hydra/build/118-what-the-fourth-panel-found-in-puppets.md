# 118 - What the fourth panel found in `puppets.py`

Type: build
Status: resolved
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

## Outcome

Built 2026-09-24 by composer-2.5 on run 01a0d428-9ddf (1.35M tokens), applied as written: Hunk
and `git diff` run from the repository over `base_sha...hydra/<run-id>` and never inside a
checkout a run can write, so `review` now asks for a reap first and refuses a run with no branch;
a plan is copied out of a worktree through `O_NOFOLLOW`, regular file, one link, 10 MiB cap, the
way reap copies since build 106, with `result_text` as the fallback; `tasks` needs a plan
approval; a diff approval carries the run id it was given for and `verdict` checks it; the log is
appended under an exclusive `flock` and a torn last line is skipped on read. The README says an
approval is a log line the CLI checks, never a proof a human was present. 89 tests green on
master. The session's puppets skill now reaps before it reviews.
