# Hydra

Hydra is a router over three agent CLIs (`claude`, `cursor-agent`, `agy`), plus Ouroboros, a
self-improvement loop that reads what the router records. The router core is Rust. `hermes` was a
fourth until ticket 22 unrouted it; its guardrail rules are still copied, its binary is not launched.

There is no application code yet. This repo currently holds a design in progress.

## Resuming work

The design lives as a wayfinder map. To continue it:

```
/mattpocock-skills:wayfinder .scratch/hydra/map.md
```

Add a ticket number to work a specific one, for example `.scratch/hydra/map.md 18`. Without one,
take the first ticket on the frontier.

Read `.scratch/hydra/map.md` first, always. It carries the destination, the settled decisions
(D1 to D16, which are constraints rather than open questions), an index of every closed ticket,
the fog, and what is out of scope.

## Layout

- `.scratch/hydra/map.md` — the map. Load this once per session.
- `.scratch/hydra/issues/NN-slug.md` — one ticket each. `Type:` is `research`, `prototype`,
  `grilling` or `task`. `Status:` is `open`, `claimed` or `resolved`. `Blocked by:` lists ticket
  numbers. A resolved ticket carries its decision under `## Answer`.
- `.scratch/hydra/research/NN-slug.md` — findings from resolved research tickets. Long. Read one
  only when its ticket is relevant to what you are doing.

No issue tracker is configured, so this is the local-markdown tracker. Run
`/setup-matt-pocock-skills` to move it to GitHub issues.

## The frontier

```
python3 .scratch/hydra/frontier.py
```

Prints which tickets are takeable now and which are still blocked. A ticket is on the frontier
when it is open, unclaimed, and every ticket it lists as `Blocked by:` is resolved.

Claim a ticket by setting `Status: claimed` **before** doing any work, so a concurrent session
skips it.

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
