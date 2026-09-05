# Hydra

Hydra is a router over three agent CLIs (`claude`, `cursor-agent`, `agy`), plus Ouroboros, a
self-improvement loop that reads what the router records. The router core is Rust. `hermes` was a
fourth until ticket 22 unrouted it; its guardrail rules are still copied, its binary is not launched.

This repo holds a finished design, three documents in `docs/`, written from a wayfinder map that
closed on 2026-09-05 with 38 of 38 tickets resolved and all four seam checks passing. It also holds
a first slice of Rust that implements one command against that design.

**Read `docs/` first, not the map. Then read `IMPLEMENTATION.md`**, which is the gap between the
design and the code. The slice is deliberately not spec-complete: it compiles no gate, so its run
records carry a null policy, and that file says what else follows from it.

The three documents are the design. `docs/harness-spec.md` is how Hydra launches, feeds and records
the providers; `docs/run-record-contract.md` is the format it writes and Ouroboros reads;
`docs/ouroboros-spec.md` is both rungs of self-improvement. They are meant to be read together, and
nothing crosses between the harness and Ouroboros except the run record and the skill store.

## The map

`.scratch/hydra/map.md` is the history: the destination, the settled decisions D1 to D16, a one-line
index of all 38 closed tickets, what is still fog, and what is out of scope. Read it when you need to
know **why** a document says what it says. The ticket behind any decision holds the reasoning the
document compressed.

The map has no open tickets. If new work needs charting rather than building:

```
/mattpocock-skills:wayfinder .scratch/hydra/map.md
```

The map carries the destination, the settled decisions (D1 to D16, which are constraints rather than
open questions), an index of every closed ticket, the fog, and what is out of scope.

## Layout

- `src/`, the binary. `run.rs` is the launch sequence and the place to start reading; `record/` is
  the run record and the validator the tests drive; `env.rs` is the allowlist; `pty.rs` owns the
  child's pty; `provider/claude.rs` holds every fact that is true of that binary and not of a model.
- `IMPLEMENTATION.md`, what is built, what is not, and every departure from the design with its
  reason. Amend it in the same change that narrows the gap.
- `tests/`, three files. `record_contract.rs` checks a written record against the contract and walks
  every record on this machine; `pty_tee.rs` drives a stub provider so the pty path costs no quota;
  `worktree.rs` covers the seeding.
- `.scratch/hydra/map.md`, the map. One load gives you the whole design at low resolution.
- `.scratch/hydra/issues/NN-slug.md`, one ticket each. `Type:` is `research`, `prototype`,
  `grilling` or `task`. `Status:` is `open`, `claimed` or `resolved`. `Blocked by:` lists ticket
  numbers. A resolved ticket carries its decision under `## Answer`.
- `.scratch/hydra/research/NN-slug.md`, findings from resolved research tickets. Long. Read one
  only when its ticket is relevant to what you are doing.

No issue tracker is configured, so this is the local-markdown tracker. Run
`/setup-matt-pocock-skills` to move it to GitHub issues.

## The frontier

```
python3 .scratch/hydra/frontier.py
```

Prints which tickets are takeable now and which are still blocked. It reads 38 of 38 resolved and
an empty frontier, and it stays useful if the map ever reopens.

If you do open a ticket: claim it by setting `Status: claimed` **before** doing any work, so a
concurrent session skips it.

## Closing a ticket

1. Append the decision under `## Answer` in the ticket file, and set `Status: resolved`.
2. Add one line to `## Decisions so far` in the map: a gist plus a link. Never restate the
   decision there, the ticket holds it.
3. Amend any ticket the answer changed, so the next session starts from the finding rather than
   rediscovering it.
4. Graduate anything the answer made specifiable out of `## Not yet specified` into a real
   ticket, and clear it from the fog.

## House rules

- Prose follows `~/.claude/skills/unslop/SKILL.md`. No em dashes.
- Grilling tickets need the human. Never answer your own questions on one.
- The originating research document, `~/Downloads/Harness CLI Unificado para Cursor, Gemini y
  Claude.md`, is a starting point and not a source. Several of its claims turned out wrong.

## Nothing here is enforced

Hydra's gate applies to runs Hydra launches, not to you. You are an agent working on Hydra's own
source, which is the one population the harness does not gate: no compiled policy, no hook config,
no injected skills, no preamble. Ticket 28 found three populations rather than two, and this is the
third.

The five protected configs in `docs/harness-spec.md` section 15.1 are protected against Hydra's own
runs. This file is what stands in their place for you. Treat them as read-only, and if a change
genuinely needs one of them, say so and let a human make it.
