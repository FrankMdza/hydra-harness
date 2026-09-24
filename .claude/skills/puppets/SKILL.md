---
name: puppets
description: Run a goal through Master of Puppets mode, Hydra's orchestration loop, from planning to applied diffs, with the human approving the plan, the assignment table and every diff. Use when the user names a goal for Hydra to plan and execute with several agents, says "puppets", "master of puppets", or asks to plan a goal with a panel.
---

# Master of Puppets

The session is the orchestrator. `integrations/puppets/puppets.py` keeps the goal's log and
refuses what the human has not approved; you read the state, launch what the log allows, review
what comes back, and ask the human at the three points the log demands. Nothing lands on master
without the human's diff approval.

Set `HYDRA_BIN=~/hydra/target/release/hydra` and run every command as
`python3 integrations/puppets/puppets.py <command>`. `show <goal>` prints the folded state; read
it before every step instead of remembering.

## The loop

1. `new "<goal text>"` prints the goal id. Record the id in your reply.
2. `plan <goal> --seat codex --seat cursor-agent:composer-2.5 --seat agy:gemini-3.1-pro-high`
   launches one proposer per seat. Keep `claude` off planning: the session's own window pays for
   the orchestration. Check swap (`sysctl vm.swapusage`) before three launches; under 1 GB free,
   launch in waves.
3. `plans <goal>` once the runs have ended (`hydra status`), then `choose <goal> --judge`:
   three judge seats (claude opus, codex under standard, composer-2.5) read the plans and must
   agree, or the command names the seat that blocked. If codex is out of credits the panel
   cannot run; then read the plans yourself and `choose <goal> <provider>`, or `choose <goal>
   <path>` for a plan you composed, and say in your message that the panel did not sit. Show
   the human the choice and the reason; `approve <goal> --plan` is theirs, through
   AskUserQuestion, one option marked recommended.
4. `tasks <goal>` imports the chosen plan's JSON block; `assign <goal>` fills the table from the
   ranking when a verdict backs it and from the default table otherwise. Print the table (task,
   class, grade, provider, model, policy) and ask the human. They may `set` any row. Every edit
   moves the revision and voids earlier approvals; `approve <goal> --table` comes after the last
   edit.
5. `launch <goal> <task>` for each task whose deps have ended. A `codex` task, or a `high` or
   `agi` task on `claude`, is refused until the human gives `approve --launch <task>`; ask, and
   say why (that gate can fail open). Inside Herdr the launch opens a pane tailing the run.
6. `wait <goal>` records ended runs. Then `hydra reap <run-id>` without `--delete-branch`: the
   diff moves to the branch `hydra/<run-id>` and the worktree goes, because nothing of the mode
   runs git or Hunk inside a checkout a run could write (build 118). `review <goal> <task>` runs
   the three judge seats over `base_sha...branch`, writes their major and critical findings into
   the Hunk session as comments, and opens Hunk on the branch. You review it the way build
   tickets are reviewed as well: read the diff in full, check the mechanism and not only the
   green suite, and look for what the agent called "preexisting". Show the human the diff and
   your review; `approve --diff <task>` is theirs and binds to that run. Apply from the branch
   (`git diff <base_sha>...hydra/<run-id> | git apply`), run the suite, commit, delete the
   branch, then `verdict <goal> <task> applied|fixed|rejected --note "..."`. `fixed` means you
   changed the diff before applying; say what.
7. A launch refused for an ambiguous intent means a crash between intent and run. Run
   `hydra status`, find the run if it exists, and `reconcile <goal> <task> <attempt> --run-id
   <id>` or `--abandon`. Never relaunch blind.

## What you never do

- Launch without the table approval, or apply without the diff approval. The helper refuses the
  first; the second is on you.
- Reassign a rate-limited task on your own. Mark it in your reply and let the human `set` it.
- Edit the log by hand. Every state change is a command.
- Hand an agent `docs/`, `config/` or `.scratch/` edits.

## Reporting

At each pause, one short message: the goal id, the step, what the human has to decide, and the
tokens each run cost (`hydra show <id>` has `usage.counts.total_tokens`). At the end, the tasks
with their verdicts and the runs behind them, and `verdict` recorded for each.
