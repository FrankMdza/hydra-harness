# 141 - `hydra approve`, a human-typed approval outside the seatbelt

Type: build
Status: resolved
Blocked by: 140

## What

Map 82, answer 4. A seat request that matches no shape waits until the human types
`hydra approve <id>` in a terminal outside `hy`. No model approves, the orchestrating session
included. The validation run (`01a0e75a`) found the store is safe from the run if it lives under
`~/hydra` (the kernel denies the run's writes there, `src/sandbox.rs` near 80 to 85) and the
command refuses inside a seatbelt, as the face does (`inside_seatbelt()`).

## Do

Tests first, each shown failing before the code:

1. `hydra approve <id>` inside a seatbelt (the test sets up the same signal `inside_seatbelt`
   reads) refuses by name and writes nothing.
2. Outside, it writes the approval to a store under `~/hydra` (a scratch `Layout` in the test);
   the waiting launcher of 140 picks it up and launches; the face's pending call returns the
   marked results. An id that does not exist, or was already approved, refuses.
3. A write to the store from inside the seatbelt is denied by the kernel (a `tests/spawn.rs` row).
4. `hydra approve` with no id lists the pending requests with what each asks for (seats,
   policies, the session that asked), so the human reads before typing.
5. The approval is on the session's record: who approved is always the human, so record the time
   and the request id, nothing else.

Herdr: `integrations/herdr/` shows a pending request on the session's pane (metadata only,
display). Not in `src/`.

Harness spec 10 says no answering a permission prompt from outside (`docs/harness-spec.md` near
1199 to 1202); amend it for this one request type, with 82 as the reason. `IMPLEMENTATION.md`.

## Outcome

Run `01a0e7d2` (claude opus, `standard`), applied as written plus the contract and `CLAUDE.md`
lines it was not allowed to write. `src/approve.rs`: a request no shape covers is
`<id>.pending.json` in `approvals/`; `hydra approve <id>` renames it to approved, the launcher to
withdrawn when its wait (10 minutes, `face::TIMEOUT`) runs out or the session ends, one rename
each so the two cannot both happen. The command refuses inside a seatbelt before it reads anything;
an unknown id, a second approval and a withdrawn request refuse by name; with no id it lists what
each request asks for. Approval writes a `seat_approval` line (only `request_id`) and launches the
seats as asked, marked. The store is on `denied_write_paths` and every profile denies reads of it,
since a waiting request carries its session's task. Herdr's bridge tags the pane.

The run said the kernel did not deny writes to `~/hydra`; on this machine it does (the profile
carries `(deny file-write* (subpath "/Users/USER/hydra"))`), but in the tests the store sits under
a scratch root, so naming it was right. A waiting call holds one of the session's four connections
for up to ten minutes. Store files are never swept. Unmeasured: whether `claude` gives up on an MCP
call sooner than ten minutes. Master: fmt, clippy, lib 217, seats 23, spawn 36 (live-kernel store
test included), gate 111, record_contract 20, face 8, harvest 12, sandbox_read_only 3, delegation
14, mcp 22, Herdr mapping OK.
