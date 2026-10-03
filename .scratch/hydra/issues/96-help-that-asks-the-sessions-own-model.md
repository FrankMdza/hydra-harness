# 96 - Help that asks the session's own model

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-02, by the session, from the human's words after his first gated session by
hand (run `01a0fdeb`): a help button that uses the same model to explain the CLI, fed by a
document written for agents "like an AGENTS.md", so the agent can answer whoever is using it,
and later by voice.

## What the session measured

- The cockpit prints none of its keys. Build ticket 240 adds a static `?` view; this ticket is
  the part that is a decision.
- Under `standard` the gate refused `claude`'s own `Agent` tool four times with
  `hydra/capability: policy standard does not allow subagent_spawn (Agent)`. The session saw
  `mcp__hydra__delegate` and `mcp__hydra__panel`, did not call them, and asked the human. Nothing
  the run is told says that delegation goes through the face. The refusal text does not say it
  either.
- A gated session reads its worktree and its skill snapshot and nothing else. A document it is
  to answer from has to be in one of the two, or in the preamble.

- Usability probes of 2026-10-02, five sessions by hand: three of them asked the session for a
  tool of the other mode. Run `01a0fe17` (gated) was told to use `mcp__hydra__delegate` and got
  three `hydra/seatbelt-delegation` refusals. An ungated `claude` and run `01a0fe2b` (ungated
  cockpit) were told to use `mcp__hydra__panel`, which neither has. Nothing on screen says which
  mode a session is in or what it can launch; the only sign is `claude under standard` against
  `claude` in the chat's title.
- The session log of those probes is the human's, outside the repo:
  `~/.claude/handoffs/2026-09-29-festus-plan/SESIONES-TESTERS.md`.

## Questions for the human

1. Where does the document live? (a) a file in the checkout, which every worktree then carries;
   (b) a skill in the store, linked into the snapshot; (c) a paragraph of `config/preamble.md`,
   protected config #2. A root `AGENTS.md` is read by `codex` and `cursor-agent` on every run,
   headless ones too, which changes what every seat is told.
2. What does the help key do? (a) types a fixed question into the chat and leaves it unsent, as
   `m` does with its note; (b) sends it; (c) opens the static view of build 240 and offers the
   question as a second key.
3. Does the same document say how to delegate under `standard`, or does the gate's refusal of
   `subagent_spawn` name `mcp__hydra__delegate` itself? The second changes a refusal string the
   gate tests pin.
4. Voice: in scope now as a constraint on the document (short spoken answers, no tables), or a
   later ticket with its own question about what leaves the machine?
5. Does a session under `read-only` or `judge` get the same help?
6. Should the session be told its own mode at launch (gated: panels only; ungated: `hydra run`
   by shell), and by whom: the preamble, which an ungated session does not get, or the cockpit?

## Answer

Resolved 2026-10-03 by a panel of three under the human's delegation: `codex` (adversary,
`standard`), `cursor-agent` grok-4.7-high (reviewer, `judge`), `agy` gemini-3.1-pro-high
(security, `judge`). One brief for maps 96 to 98, ten decisions, none split three ways, so no
judge sat. Brief and answers in the human's decision log,
`~/.claude/handoffs/2026-09-29-festus-plan/decisiones/2026-10-03/`.

- **Where the document lives (D1): a file in the repository**, 2 to 1 (codex 0.68, grok 0.63;
  agy took a skill, 0.80). Not `AGENTS.md`, which `codex` and `cursor-agent` read on every
  headless run; not the preamble, which an ungated copilot never gets. Grok's condition: a
  copilot launched outside the repository's instructions (map 97) does not see it, so the
  copilot's own document carries the same help or points at it.
- **What the help key does (D2): the static view, and a second key types a fixed question into
  the chat, unsent**, unanimous (0.85, 0.76, 0.90).
- **How a session learns its mode (D3): both**, 2 to 1 (codex 0.94, grok 0.77; agy took the
  start line alone, 0.70). The launcher gives both modes one line at start, and the `delegate`
  and `subagent_spawn` refusals name the way out. Grok: if one half is cut, keep the start line.
- **Voice (D4): a later ticket**, unanimous, with its own question about audio leaving the
  machine.

Reversed by: a five-minute test where newcomers get no answer from the file and do from a skill
(D1); refusal text that names `panel` sending more than 2 of 5 gated sessions to it for ordinary
local work (D3).

Carried by build tickets 246 (the document and the asking key, after 240) and 247 (the start
line and the refusals).
