# 236 - otel in Rust: the cutover

Type: build
Status: open
Blocked by: 234, 235

## What

Map ticket 95, "O2" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`.

## Do

Close what the review found; delete `record_to_otlp.py`, its test and `redact.py`. The session
runs `--all` both ways over the real runs first.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.
