# Who wrote the code

Part of Hydra was written by the agents Hydra launches, through `hydra run`, while a Claude Code
session wrote the tickets, reviewed each worktree diff and applied it to master by hand. The
corpus in `~/hydra/runs/` and the git history are the sources. Each build ticket an agent wrote
names its run id and token count under `## Outcome`, and since build 312 `hydra-puppets report
tickets --dir .scratch/hydra/build --runs ~/hydra/runs` prints the table from the tickets and the
records: who built each ticket, whether it stood, and what it cost.

## The first 122 build tickets, resolved by 2026-09-24

80 has no row of its own, because build ticket 96 closed it.

| Written by | Build tickets |
|---|---|
| `cursor-agent` on Composer 2.5 | 20, 24, 28, 29, 31, 32, 34, 36, 48, 50, 51, 60, 61 to 75, 77 to 79, 86, 87, 90, 92, 93, 108, 110, 111, 113, 114, 116 to 122 |
| `cursor-agent` on Grok 4.6 Fast | 21, 30, 33, 35, 37, 38 |
| `cursor-agent`, no model flag, so the record says none | 17 |
| `claude` on Sonnet 5 | 18, 22, 23, 26, 27 |
| `claude` on Haiku 4.5 | 25, 82 |
| `claude` on Opus 5.5 | 91, 96 |
| `agy` on Gemini 3.1 Pro | 19, 94, and the drift test of 17 |
| `agy` on Gemini 3.8 Flash | 115 |
| `codex`, no model flag, so the record says none | 81, 83, 85, 89, 95, 109 |
| a Claude Code subagent on Opus 5.5, in a git worktree of its own and not through `hydra run` | 97 to 106, 112 |
| the Claude Code session, by hand | 01 to 16, 39 to 47, 49, 52 to 59, 76, 84, 88, 107 |

On 77, 78, 81, 82, 85, 86 and 87 an Opus subagent fixed the agent's diff in the worktree before
the session applied it, and on 82 it rebuilt Haiku's version. From 89 to 96 an Opus subagent
reviewed each diff and the session made the fixes on master; every one of those eight needed at
least one, and on 92 a second Composer run made them. That is 72 of the 121 build tickets on an
agent run through `hydra run`, none of them merged unread. Tickets 97 to 106, the third hiring
panel's, went to subagents working on the source directly. Two Opus reviews of their merged code
opened 105 and 106, and 112 holds what those two left. Ticket 27, the interactive mode and its
harvest, was the largest single run at 54.7M tokens, 54.0M of them cache reads. Ticket 25 was the
first Rust ticket given to the cheapest model, and it held.

## Since then

Build 312's report over tickets 01 to 313, run on 2026-10-14: Opus 5.5 built 52 tickets for 278M
tokens, Composer 2.5 built 78 for 192M, GPT Sol built 25; 285 of 313 tickets name no model in
their Outcome, which is why the report exists. The design documents, the policies, the record
contract and the README are the session's.
