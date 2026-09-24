# 105 - What the review of 97 to 99 found

Type: build
Status: open
Blocked by: 103

## What

An Opus review of build tickets 97 to 99 as merged, 2026-09-24, in
`~/.claude/handoffs/2026-09-23-panel3/review-97-99.md`. Implement every numbered item under 98 and
99 there. The measured one comes first: since build 99, files the human ignores only through a
global `core.excludesFile` are seeded into a run's checkout and committed by reap.

## Done when

Each item has a test that fails before the fix, checked by reverting it, and the suite passes
outside a seatbelt. Documentation sentences go in the Outcome for the session.

## Outcome
