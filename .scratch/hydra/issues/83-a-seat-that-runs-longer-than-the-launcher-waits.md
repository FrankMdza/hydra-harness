# 83 - A seat that runs longer than the launcher waits

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-29, by the session, from the candidates the handoff of 2026-09-28 listed and a
panel seat's finding the same day.

## Question

A gated interactive session asks its launcher for seats over `seats.sock` (map ticket 82). The
launcher launches each seat with `timeout: crate::face::TIMEOUT` (`src/seats.rs:715`), which is
600 seconds (`src/face.rs:50`). Past that the seat keeps running and the session gets a handle,
the seat's run id. The session cannot follow it: `runs/` is denied to every run under every
policy.

So a seat that reviews, and ends in under ten minutes, works. A seat that builds, which on this
repository takes about thirty, returns an id the session cannot read. What should a gated session
be able to do about a seat that outlives the wait?

## Evidence

- `src/seats.rs:715` and `src/face.rs:50`, read on 2026-09-29.
- Map ticket 82 decided that the socket carries one kind of crossing, a request for seats.
- Long work runs today from an ungated Claude Code session that launches `hydra run`. It built 72
  of the build tickets that way (`README.md:264`).
- No real session has asked for a seat. `grep -l seat_request runs/*.jsonl` names three records,
  all of them build runs that mention the word.
- Nobody has measured whether `claude` holds an MCP call open for 600 seconds. The handoff of
  2026-09-28 lists it under what is not known.

## Answer

Resolved 2026-09-29, by the session, under the human's delegation of that day ("help me decide
the rest, take control, with a panel and a reasoning over its results"). The human did not answer
this one himself and can reverse it.

**Nothing changes now.** A gated session asks for seats that end inside the wait. Long work stays
outside, where it already runs. The choice between a second request on the socket
(`result <run_id>`, answered only for that session's children) and a longer wait is made after
the first live panel has measured how long `claude` keeps a call open.

A panel of three vendors voted on four options, each seat with its own role, from a brief that
did not say what the session preferred. All three chose this one: `claude` opus as reviewer with
p 0.60, `codex` as adversary with 0.77, `agy` gemini-3.1-pro-high as security reader with 0.80.
The panel sat degraded and declared: `cursor-agent` was rate limited, and `agy` took its seat.
The session had preferred the second request, and the panel's reason against it holds: it adds a
crossing to the seatbelt for a need no session has shown yet.

`codex` added what the live panel must do for this to be decided at all: a review that ends in
under ten minutes never reaches the 600 second boundary. The live test needs one seat that is slow
on purpose.

Reversed by: a gated session's task that needs a child's result after 600 seconds, which takes the
second request; or `claude` holding a call open for 3,600 seconds, which makes the longer wait the
cheap answer.

The votes and the brief are in `~/.claude/handoffs/2026-09-29-festus-plan/decisiones/`.
