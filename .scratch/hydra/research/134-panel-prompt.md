# Decision panel for map ticket 134

You are one seat on a four-seat decision panel. Read only: do not edit, create or delete a file in the repository, and do not commit. Scratch experiments under `/tmp` are fine. Your seat name is in your task line.

The human asked on 2026-11-01 for the panel to move ticket 134 forward. Your answer counts as a vote, so commit to one option per question. Publishing the repository, editing a protected config and pushing stay with the human. The panel decides what he is handed.

## Read

1. `.scratch/hydra/issues/134-the-public-gate.md`, the whole ticket, including "Where it stands on 2026-10-28".
2. `.scratch/hydra/issues/99-the-gated-copilot-can-still-write-claude-json.md`, `110-jev-on-hydra-s-own-diffs.md` and `114-what-counts-as-an-attack-that-got-through.md`. Only those from `issues/`.
3. `SECURITY.md`, all of it, and `README.md`.
4. `src/sandbox.rs`, the narrowed profile of a gated interactive session (`narrow_for_copilot` and what it allows under home).
5. `config/mcp.md` lines 38 to 44 and `policies/standard.json`.

## A measurement you cannot repeat from inside a run

Taken by the session on 2026-11-01, outside any seatbelt, `claude` 2.1.312, model `claude-haiku-5-5`. The profile was `(allow default)` plus a write deny on `^/Users/USER/\.claude\.json` and, in the last row, on `^/Users/USER/\.claude/history\.jsonl` too. A shell `printf x >> ~/.claude.json` under the same profile was refused with "Operation not permitted", so the deny was live.

| Launch | Result | `~/.claude.json` sha and mtime |
| --- | --- | --- |
| `claude -p`, cwd `~/hydra` | answered "ok", exit 0 | unchanged |
| interactive through a pty, cwd `~/hydra` (already trusted) | answered "pong", `/exit` clean | unchanged |
| interactive, a fresh `/tmp` directory never trusted | trust dialog shown, "Yes" accepted, answered "pong", no error | unchanged |
| interactive, `~/hydra`, both files denied | answered "pong", no error | both unchanged |

So `claude` runs without writing either file. The trust acceptance in the fresh directory lived in memory only. Not measured: a session long enough to trigger whatever else `claude` saves there (MCP server approvals, onboarding flags, usage counters), or a session through Hydra's own `run --interactive` launcher rather than a hand-written profile.

## Questions

1. **Map 99, given the measurement.** (a) Deny writes to `~/.claude.json*` and `~/.claude/history.jsonl*` in the narrowed profile, a build before publication; (b) give the gated session its own config directory under the spawn directory, seeded from the human's; (c) leave it writable, compare `mcpServers` before and after, and mark the record; (d) state it and fix after publication. Say what your choice costs the human in daily use of the copilot.
2. **Map 110 and map 114 at publication.** For each: state it in `SECURITY.md` and publish with it open, or fix before. Check what the code does today: does anything under `src/` still send a diff off the machine, and does `hydra bench` refuse without `--dry-run`?
3. **A fresh audit.** Pick at least five claims in `SECURITY.md` you can check against code or a test, and check them. List every claim you found false or stale, with file and line on both sides. Then vote: does publication need a new fix-before ticket, yes or no?
4. **What the human is handed.** Propose the exact replacement text for `config/mcp.md` lines 41 and 42, up to eight GitHub topics, and whether the repository needs a homepage at all. One line each.

Then list the tickets your answer needs, one line each, smallest first, each marked build or map.

At most 800 words. End with `## Pending from me`, `## Changed` and `## Found`.
