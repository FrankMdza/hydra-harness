# 140 - Two videos: a frontier model editing weights, and AWS Infinity's sliced agent runtime

Type: research
Status: resolved
Blocked by:
Opened: 2026-11-04, by the session, from the human's request after maps 138 and 139.

## The human's words

Translated from Spanish: "before going on, look again at the YouTube short we talked about, now it
can be done, and this one <https://www.youtube.com/watch?v=os5eNeAVRpc>, to refine a bit more. If it
needs it, research more on both, and launch a panel to see how we integrate them into the plan."

## What the session read

`research/140-video-notes.md`. The short's title confirms the thumbnail reading of research 138: a
frontier model (GPT-6) fixing another model by editing its weights. The video is about Infinity,
AWS's Rust runtime that runs agents in slices and keeps no process alive while they wait.

## Why each one touches the plan

- **The short** is map 138's question 3 (retraining chosen parts) and question 4's second item
  (frontier weight diffs), which the human has not answered yet. Research 138 read it through news
  coverage only.
- **Infinity** touches three things:
  - The RAM ceiling. Hydra holds one provider CLI process per run, about 200 MB each, on an 8 GB
    Mac. Infinity's claim is 75,000 agents in 8 GB, because a waiting agent costs no process.
  - Map 137's cloud arc, where agents may run in the cloud. Infinity scales to zero on Lambda.
  - The nano agents of map 138. A model the lab owns could run under a sliced runtime, while
    `claude`, `codex`, `agy` and `cursor-agent` own their own loop and cannot be sliced from
    outside. `hydra resume` (harness spec, around line 1223) is the nearest thing Hydra has.

  It also crosses one decision. Infinity keeps the message history in durable storage between
  slices, and map 137 answered that no agent record stays in the cloud.

## Research questions

1. **The short, from its source.** Open the short and its transcript. Name the primary source
   (OpenAI's post, the MIT CSAIL paper, a repository) and say what was edited, on which model and
   size, how it was checked, and what failed. Correct research 138 section 1 where it was wrong.
2. **Infinity, from its source.** The AWS post, the repository, licence, version, language, the RAP
   protocol's shape, what storage it needs, which model APIs it calls, and the 75,000 figure with
   how it was measured.
3. **Infinity against Hydra.** Read `docs/harness-spec.md` (detached children, `hydra resume`, the
   run record) and `src/watch.rs`. Say what Hydra already does that resembles a slice, what it cannot
   do because it wraps CLIs, and whether its record could serve as a slice's durable history.
4. **Where it could fit, without deciding.** Three options at least: borrow ideas only (synthetic
   tool calls for events, wake on message), a sliced runtime for the lab's own nano agents, or
   Infinity itself in the cloud. For each, cost in RAM and dollars, and what it does to map 137's
   no-records-in-the-cloud answer and to map 139's credential work.

Defensive and read-only. No account creation, no AWS spend.

## Output

`research/140-two-videos.md`, at most 2,000 words, every claim with its source and the date read,
gaps named. Then a panel under `research/140-panel-prompt.md`.

## Panel

Sat 2026-11-04, three seats (Grok not seated, Cursor login). Synthesis in
`research/140-synthesis.md`. Waiting on the human.

## Answer

Resolved 2026-11-04 by the human on the three-seat panel ("agree con todo, medir primero, el 389
va ya").

- The short changes none of map 138's votes. Build 386, when opened, carries a third arm: one
  direct weight edit on the gate model for one class of false allows, written by a human or a
  script from activations, measured like the other two arms, $0 on the Mac. Who may write edits
  stays the human's open item on map 138.
- Infinity: borrow ideas only. Infinity in AWS is out under map 137. A sliced runtime for the
  lab's own agents waits for a nano model that calls tools, and writes its own loop with history
  as linked runs.
- Measure first: build 390 counts the waiting parents against a threshold written before the run.
  Map 141, wake on message, is blocked by 390.
- Build 389, a resumed run keeps its depth, goes now.
- Events are derived from records, not received. No RAP endpoint, no network listener.
