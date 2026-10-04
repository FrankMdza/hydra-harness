# 234 - apprentice in Rust: the cutover

Type: build
Status: claimed
Blocked by: 232, 233

## What

Map ticket 95, "A5" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`, and question D: `bitnet_serve.py` is dropped, not ported.

## Do

Close what the reviews found. The plist template runs `hydra-apprentice attempt --minutes 30`
and its test becomes Rust. Delete `apprentice.py`, `bitnet_serve.py`, `records.py`, `models.py`
and their tests; `redact.py` moves to `integrations/otel/` while `record_to_otlp.py` loads it.
The session runs `bank build` both ways over the real runs before master takes the diff, and the
human replaces the installed launchd job if there is one.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.
