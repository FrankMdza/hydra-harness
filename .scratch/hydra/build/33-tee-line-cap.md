# 33 - The tee caps a line

Type: build
Status: resolved
Blocked by: 05

## Goal

`pty::Tee` accumulates bytes until a newline and writes the line as one record frame. A provider
that streams one enormous frame with no newline grows that buffer without limit. Frames of 200 KB
are tested and fine. Put a cap on it, and say on the record when the cap was hit.

## Where the design says it

- `docs/run-record-contract.md` section 3, frames verbatim, and section 6 on `run_end.error`,
  which already carries the count of lines that were not valid UTF-8 and were stored lossily.
- `src/pty.rs`, the read loop with `pending`; `tests/pty_tee.rs`, the 200 KB test.
- `IMPLEMENTATION.md`, "Rough edges worth knowing", the tee bullet.

## The slices

**The cap.** 16 MiB per line, a constant in `src/pty.rs` with its reason in the doc comment. When
`pending` reaches it with no newline, the tee writes what it holds as a frame of its own, marked
the way a lossy UTF-8 line is marked, and keeps reading; the remainder of the provider's line
becomes the next frame. Nothing is dropped. The count of capped lines lands in `run_end.error`
beside the UTF-8 count, in the same sentence shape, and stays null when nothing was capped.

**The reader.** A capped frame normalizes as `noise` when it does not parse as JSON, which is what
an ordinary non-JSON line already does, so no new event type. `Finding::Limit` on a run whose
`run_end.error` names capped lines, saying that many frames were split by the tee.

**Tests.** `tests/pty_tee.rs`: a stub that writes one 20 MiB line with no newline produces two
frames, both on the record, `run_end.error` naming one capped line, and the record still
validates; the existing 200 KB test is untouched. `tests/normalize.rs`: that record reads back
with one limit finding.

## Done when

- The tests above pass and the rest of the suite is untouched.
- The orchestrating session removes the rough-edge bullet and says in the detached-spawn
  section what the cap is and what a capped frame looks like.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Cursor Grok 4.6 high fast, run 01a0801d-7832 under
`standard`: 2.97M tokens, eight files, 177 insertions, two tests, forty minutes, launched in
a Herdr pane the bridge labelled from the record. `LINE_CAP` is 16 MiB in `src/pty.rs`, the
split lands on `run_end.error` in the lossy-UTF-8 sentence shape, and every adapter pushes
the limit finding. Two assumptions the ticket made turned out false: a 20 MiB payload cannot
ride argv, so the stub is `dd | tr`; and the normalize test reads the finding off a synthetic
`run_end.error` rather than replaying the 20 MiB record. The model reported the corpus walk
failing on record 01a07efb-de49 inside its worktree, which master does not reproduce; every
agent run from a worktree has said the same since ticket 31, and the launch text now tells
them to ignore that one test.
