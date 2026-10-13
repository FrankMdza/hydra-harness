# 122 - Whose plan was it when the panel composed one: crediting a composed plan's outcome

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-13, by the session, from build 303's Outcome. Grilling: the human answers, the
session only asks and brings what it measured.

## Question

Build 303 joined each plan proposal to whether it was chosen and, when it was, to what became
of its tasks. On this machine the join found one goal with outcomes, `ea69da1f`, and on it the
panel chose `composed`: a plan stitched from the proposals, written to `plans/composed.md`, a
file no proposer produced. So every proposer of that goal is `chosen: false`, its six verdicts
(three applied, three fixed) land on no proposer row, and the one row that says "chosen and
how it ended" is a goal still pending. The training row map 118 wants does not exist yet, and
the first reason is this one.

1. When the panel composes, is any proposal credited? Three readings: none (a composed plan is
   the panel's work, the proposers only drafted); all, with a share (each proposal gets the
   outcome weighted by how many of its tasks survived into the composition, which `write.rs`
   does not record today); or the nearest (the proposal most of the composition came from,
   which someone would have to measure or the panel would have to declare).
2. If the answer is a share or the nearest, what does the panel write at composition time so
   the join can read it later? A list of source proposal ids per task in `plans/composed.md`'s
   JSON is the smallest change, and it is a change to a protected prompt and to the goal log's
   contract.
3. Does a `false` proposer row teach anything? The 118 panel kept preference and outcome apart
   so that a rejected draft is still a preference example. If `chosen: false` with no outcome
   is enough for that, question 1 only matters for the outcome label.

The session's own view, which is not an answer: credit none, keep composed goals out of the
outcome label, and count how many goals compose before paying for a share. If most goals
compose, the outcome label is empty and question 2 becomes a build.

## Answer

## Panel

Two seats sat on 2026-10-13, Opus 5.5 and Gemini 3.8 Flash; the GPT seat died on codex's usage limit at its first turn and there is no Grok seat while Cursor is on the free plan. Memos: `research/122-seat-opus.md`, `122-seat-gemini.md`. The session's synthesis with a recommendation per question: `research/122-synthesis.md`. Its first finding, confirmed in the review of 2026-09-24: no panel composed `ea69da1f`, the session chose a file; and the live panel cannot compose on its own prompt, which never asks for the task array the verdict requires. Nothing under Answer is written; the human answers.
