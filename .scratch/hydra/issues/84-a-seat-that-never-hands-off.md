# 84 - A seat that never hands off has no place on the record

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-29, by the session, from what build ticket 142's Outcome left open for the map.

## Question

Build 142 counts a seat from its `seat_request` line until its watcher detaches and `handed_off`
fires. When the session ends, `stop` waits up to `HANDOFF_BUDGET`, 60 seconds
(`src/seats.rs:75`). A seat still inside `run::run` after that dies with the process, and the
launcher names it on stderr. Its run id is already in the request line's `runs`, and nothing else
about it is written anywhere.

D10 says what is not in a record did not happen. Where does a seat that died this way go?

## Evidence

- Build 142's Outcome: "a seat that never hands off before the session ends has no place on the
  record. The contract gives `seat_request` three decisions; the launcher names such seats on
  stderr after `claude` exits, and their run ids are already in the request line's `runs`."
- `src/seats.rs:1043`: the launcher shares the human's terminal with the session, so a line on
  stderr lands in the middle of the provider's screen.
- A launch that fails after its worktree exists already writes a record with outcome
  `never_started` (`src/run.rs:936`, `record_never_started`).
- Nobody has measured whether the four providers start under `read-only` or `judge` as a seat. The
  first live panel is where a seat is most likely to die.

## Answer

Resolved 2026-09-29, by the session, under the human's delegation of that day. The human did not
answer this one himself and can reverse it.

**The seat gets its own record, in the shape a failed launch already writes, `never_started`.**
Build 147 found that `never_started` names that shape in the code and is not an outcome: the
record derives `failed`.
The session's record is not given a third line type.

The same panel as map ticket 83 voted, three vendors and three roles, degraded and declared. All
three chose this, and all three named the new line type as their second choice: `claude` opus with
p 0.50, `codex` with 0.64, `agy` with 0.75. Their reason: it makes the death countable without
amending the record contract for a case nobody has seen yet.

The brief the panel read described the case as a provider that fails before its record is handed
off. The case in the code is narrower, a seat still launching when the session ends. The answer
covers both, since in both the run id exists and the record does not.

Two seats asked for the same fact, whether a run id exists before the provider starts. It does:
the request line carries it.

What the build ticket must check before it builds: that the contract admits a `never_started`
record for a run whose worktree may not exist. If it does not, this answer amends the contract
too, the reason for preferring it is gone, and the ticket stops and says so.

Reversed by: that check failing, which takes the new line type.

Build ticket 147. It lands before the first live panel.
