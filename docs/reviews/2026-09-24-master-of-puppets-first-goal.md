# The first Master of Puppets goal, and the day that built the mode

Written 2026-09-24 by the session for the human to review later. Every run below is a record
under `~/hydra/runs/<id>.jsonl`; `hydra show <id>` prints it normalized. The goal's own log is
`~/hydra/puppets/ea69da1f-0114-40fd-8a1d-bed411894331.jsonl` and `python3 integrations/puppets/puppets.py board ea69da1f-0114-40fd-8a1d-bed411894331`
prints it at a glance. The panel briefs, the raw proposals and the launch scripts are outside the
repo in `~/.claude/handoffs/2026-09-24-puppets-panel/`.

## What happened, in order

1. Map ticket 75: a four-proposer design panel for the mode (codex, composer-2.5, agy, an opus
   subagent), consolidated in `.scratch/hydra/research/75-master-of-puppets-panel.md`, then
   judged by codex. Ticket 76 holds the ten decisions; the human took A, B, C, I and J.
2. Build tickets 107 to 111, 118, 121 and 122: the `hy` launcher, `puppets.py` and
   `sheet.py`, the deny-list entry, the judged steps, the fourth panel's fixes, the local
   model's MCP server, the mascot pane. Ticket 77: where a small model trains and serves.
3. The human's first gated interactive session through `hy`.
4. Goal `ea69da1f-0114-40fd-8a1d-bed411894331`: the grader that collects training data for the local model. Six tasks, six
   diffs approved by the human, one security review. `~/hydra/grader/dataset.jsonl` has 426
   lines, seven of them labelled by this very goal.

## Every run of the session

| What | Run | Provider | Outcome | Tokens |
|---|---|---|---|---|
| 75 panel, proposer | `01a0d21e-01a1-7dc2-98d8-2908c045c3f0` | codex | completed | 2.57M |
| 75 panel, proposer | `01a0d21e-2ffe-78a2-b00e-3a46bc5b103d` | cursor-agent composer-2.5 | completed | 0.91M |
| 75 panel, proposer | `01a0d223-510f-7d62-839a-12f67e6831cd` | agy gemini-3.1-pro-high | completed | 0.50M |
| 75 panel, judge under `judge` | `01a0d229-ffd9-7fc3-b45e-3769ed084443` | codex | gated on its first read | 0.14M |
| 75 panel, judge under `standard` | `01a0d22b-6516-7553-8715-17cfd033e9da` | codex | completed, accept with changes | 0.64M |
| mascot art | `01a0d255-8f5f-7210-bb53-7cf99e1bcfc2` | codex | completed, the chosen sprite | 0.18M |
| mascot art | `01a0d255-b61a-7582-a35b-f9f0f3f14b6d` | agy gemini-3.1-pro-high | completed | 0.67M |
| build 108, puppets.py | `01a0d36f-1612-73e1-81ae-41805679498f` | cursor-agent composer-2.5 | completed, five mechanisms changed on review | 1.35M |
| build 109, sheet.py | `01a0d36f-4447-7b22-b59b-af98eb122a01` | codex | completed, applied as written | 1.56M |
| build 110, deny list | `01a0d41b-c2cf-7a90-ac05-d94a965a1ba8` | cursor-agent composer-2.5 | completed | 1.26M |
| build 111, first try | `01a0d41b-f171-7991-a9cb-0513a0fa9c79` | codex | rate_limited at launch | 0 |
| build 111, judged steps | `01a0d41e-5b15-7420-8a07-dd925979f284` | cursor-agent composer-2.5 | completed | 2.63M |
| build 121, local-llm MCP | `01a0d41e-8a6c-7be1-9fcb-42abb59e80fb` | cursor-agent composer-2.5 | completed | 0.86M |
| build 122, mascot pane | `01a0d425-68fa-7ad1-9eb3-af1fe207126b` | cursor-agent composer-2.5 | completed, tool rate reworked on review | 1.29M |
| build 118, fourth panel's fixes | `01a0d428-9ddf-7af2-85d5-cc948fa61616` | cursor-agent composer-2.5 | completed | 1.35M |
| `hy`, the human's gated session | `01a0d56e-ba1b-7c72-8a44-0dc29389cae9` | claude sonnet, interactive | exit 0, derived `failed`: no transcript to harvest | unavailable |
| goal, planner | `01a0d57e-4cf7-71c3-8c66-2fb16421a749` | codex | rate_limited until 2026-09-27 23:04 | 0 |
| goal, planner | `01a0d57e-52e4-7b50-814e-d2468052635f` | cursor-agent composer-2.5 | completed, the plan the composed one came from | 0.42M |
| goal, planner | `01a0d57e-8292-78d0-b770-87c80facaeae` | agy gemini-3.1-pro-high | completed | 0.02M |
| goal, canonical-models | `01a0d588-b40a-7002-994d-db3d5d3fe586` | cursor-agent composer-2.5 | completed, verdict applied | 1.35M |
| goal, redact | `01a0d588-ebc8-73f1-ab14-e491902ccac9` | cursor-agent composer-2.5 | completed, verdict fixed (a committed .pyc) | 0.42M |
| goal, collector | `01a0d58c-ee76-7982-8033-2c40b3c90838` | cursor-agent composer-2.5 | completed, verdict applied | 1.15M |
| goal, dataset | `01a0d595-570d-7123-8e47-88705fa3dab6` | cursor-agent composer-2.5 | completed, verdict fixed (invented benchmark, watch) | 1.01M |
| goal, end-to-end | `01a0d59a-4971-7f22-af59-50d15a940beb` | cursor-agent composer-2.5 | completed, verdict applied | 0.61M |
| goal, security-review | `01a0d59c-9f01-7810-9d33-e906bb8a2cbe` | claude opus | completed, block; verdict fixed | 1.14M |

Twenty-five runs, 22.0M tokens on the recorded ones. Not in the table: the opus subagents of the
Agent tool (the 75 panel's claude seat, 179k; ticket 77's research, 139k), which leave no record.

## What to look at

- **The security review paid for itself.** Opus found the port of export's OpenAI span rule
  indexing a byte array with character offsets, so a task text with an accent before a key left
  the key in clear. Every test was ASCII. Fixed in 47227b8 with accent tests. Read the run's
  `result_text` for the other findings; the one deferred is build ticket 128.
- **Composer invents numbers it cannot measure.** The dataset task's README named rebuild
  times of minutes; a gated run cannot read the corpus. The session measured 44.4 s and 15.7 s.
  Read what an agent calls measured the way the memory notes say to read "preexisting".
- **Your `hy` session derived as `failed` with exit 0**, because Claude wrote no transcript
  for the session id the launch passed. Either the session sent no message before it quit, or
  the transcript path differs from what the harvest expects. `hydra show 01a0d56e` has the path
  it looked for. Worth one more session that sends a message and quits.
- **codex is out of credits until 2026-09-27 23:04**, so neither judge panel sat today: the plan
  was chosen by the session and every diff was reviewed by the session, both said so at the time.
- **The ranking is still prior-only.** Six verdicts exist now; `sheet.py rank` takes them once
  the sheets are rebuilt (`python3 integrations/puppets/sheet.py build`).
- **Decisions D to H of ticket 76 are still open**, and the mode's automatic phase waits on J.

## Costs the human might want on the spec sheet

composer-2.5 did eleven of the day's tasks for 0.4M to 2.6M tokens each and every one applied,
five after a correction. codex did the hardest reading (2.57M) and the best mascot, then ran out
for three days. agy answered in 0.02M to 0.67M and was never the one chosen. opus, once, as a
reviewer: 1.14M and the only critical finding of the goal.
