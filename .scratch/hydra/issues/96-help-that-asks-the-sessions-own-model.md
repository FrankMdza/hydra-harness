# 96 - Help that asks the session's own model

Type: grilling
Status: open
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

## Answer
