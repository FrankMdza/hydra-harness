# 132 - The human's pending list in the cockpit

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-15, by the session, at the human's request. The orchestrating session keeps what
the human owes in `~/.claude/handoffs/2026-09-29-festus-plan/PENDIENTES-FRANK.md`, newest first,
`[ ]` open and `[x]` done. The human reads it only when he opens it. The first idea was a band
above the Claude Code prompt from a mod; map 103 blocks user mods on this Mac, so the cockpit,
which already sits beside the orchestrator's chat, is the place left.

## Question

The cockpit's waiting lane already lists what `hydra-puppets pending <goal> --json` says a goal
waits on (build 182). This list is a different thing: it is the human's own, written in prose by
the session, and outside the repo.

1. Where the list lives. A path under `~/.claude/handoffs/` is a date-named handoff folder, and
   the cockpit reading it hard-codes one session's history. Options: a fixed path the session
   writes to (`~/hydra/puppets/PENDING.md`, private, ignored by git), a path in Hydra's config,
   or a flag on `hydra cockpit`.
2. What the cockpit shows. The count of open items in the header, a page of their first lines,
   or both. Items are multi-line bullets with a bold lead; the first line up to the period is
   the title.
3. Whether the cockpit may tick an item. D10 and build 150 say it writes nothing but an approval,
   so the session leans to read-only, with the human ticking in the file.
4. Language. The file is in Spanish today; the cockpit's own text is English.

## Answer
