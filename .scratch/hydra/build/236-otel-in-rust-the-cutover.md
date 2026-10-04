# 236 - otel in Rust: the cutover

Type: build
Status: resolved
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


**From 235 (2026-10-04):** refuse a run id holding `/` or `..` in `convert.rs`'s `normalized`, which joins it into a path; the Python does not, and the port kept parity.

## Outcome

Built 2026-10-04 by a Grok 4.6 run (`01a10691-e755`), reviewed, amended and applied by the
session, committed before 234.

Deleted `integrations/otel/record_to_otlp.py` and its test; `integrations/otel/README.md` names
`hydra-otel`. `normalized` refuses a run id holding `/` or `..` before it is joined into a path;
`crates/otel/tests/cutover.rs` plants `../../pwned` and an absolute id and checks no canary is
written, failing without the check. The `--all` comparison both ways was build 235's: 80
byte-identical files from 40 real records.

**Assumed and false:** `integrations/otel/redact.py` never existed; the Python loaded
`integrations/apprentice/redact.py`, which 234 deleted.

**Found by the run and closed by the session:** `session_id` joins `parent_run_id` into a sibling
path the same way and reads the record there. It now refuses `/` and `..` too, with a second test
(a stub `hydra` answers `{}` so the conversion reaches it) that exits 0 without the check, having
read `../outside.jsonl`.
