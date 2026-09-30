# 188 - The goal log and its last line

Type: build
Status: resolved
Blocked by: 185

## What

A read-only review by `codex` on 2026-09-30 of build 185 (`03c0dac`) confirmed its fixes and found
two defects with an unterminated last line in a goal log, each reproduced in memory:

1. **A lost approval.** `integrations/puppets/puppets.py` near 342: with a fragment such as
   `{"event":"assignment",` as the last line and no newline, `approve` appends onto the fragment
   and returns 0; the joined line is not JSON, `read_log` drops it, and the next append makes it
   an interior bad line that makes reads raise. Inherited from the old `append_event`.
2. **Two readers disagree.** Near 330: the locked fold counts a complete JSON object with no
   terminating newline, while the ordinary fold ignores it, so `approve --expect-revision 0`
   refuses with "current revision 1" while `pending` still says 0.

## Do

Tests first in `integrations/puppets/tests/`, each shown failing before the change. Stdlib only.

1. One rule for both readers: only newline-terminated lines count; an unterminated tail is
   ignored by both.
2. Under the lock, before appending, an unterminated tail is left in place and the new line starts
   on a fresh line (write a newline first), so the approval is never glued to the fragment and the
   fragment stays an ignorable bad line; or refuse with a non-zero exit and write nothing. Say
   which you chose and why, and what `read_log` then does with the fragment as an interior line.
3. Every other writer that goes through `_locked_update` gets the same treatment.
4. `integrations/puppets/README.md`, one sentence.

## Done when

- Each test failed before the change; the puppets suite passes.

## Outcome

Resolved 2026-09-30. Built by `cursor-agent` grok-4.7-high on run 01a0f3bb-3b3d under `standard`.
The session applied it as written. Both readers count only newline-terminated lines. Under the
lock, an unterminated tail raises `UnterminatedLog` and every writer (`approve`, `append_event`
and so `set` and the rest) writes nothing and exits 1. The run chose refusing over writing a
leading newline because a newline would make the fragment an interior line, which `read_log`
raises on, and would make a complete unterminated object start counting. The cost: a writer that
dies mid-line leaves the goal log refusing writes until a human trims the tail. Three tests, each
failing first; the puppets suite passes, 177 tests.

