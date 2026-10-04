# 234 - apprentice in Rust: the cutover

Type: build
Status: resolved
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

## Outcome

Built 2026-10-04 by an Opus run (`01a10691-e77a`), reviewed and applied by the session after 236.

Deleted `apprentice.py`, `bitnet_serve.py` (dropped by map 95 D), `records.py`, `models.py`,
`redact.py` (deleted rather than moved, since 236 removed its other user) and their four test
files. The launchd template runs `target/release/hydra-apprentice attempt --minutes 30` with no
interpreter and no `PATH`; `crates/apprentice/tests/launchd.rs` is the Rust form of the Python's
`LaunchdTests` plus three checks, two of them failing on the old template and README. The README
names the binary's commands. No apprentice job is installed on this machine (checked by the
session: `~/Library/LaunchAgents` holds only the grader's).

The both-ways checks were 231's and 232's: a byte-identical bank of 400 tasks from the real runs,
and matching attempts and score against a live Ollama.

**Closed by the session, outside the run's reach:** CI's last step ran
`unittest discover -s integrations/apprentice/tests -p 'test_re*.py'`, which the deletion left
pointing at nothing; it and its comment are gone. `README.md`, `integrations/grader/README.md`,
`integrations/puppets/README.md` and two passages of `IMPLEMENTATION.md` still described the
deleted files as present and now say they were deleted. Port comments that name the Python they
came from stay, as history.
