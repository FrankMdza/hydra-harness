# 34 - Does any vendor transcript record a correction as a pair?

Type: research
Status: open
Blocked by:

## Question

Surfaced writing `docs/run-record-contract.md`, and it is the one place the D12 seam check currently
fails.

`correction` is one of the ten event types and it has exactly one producer: a human correcting a
tool call in an **interactive** run, recovered at harvest. It is also the only valid source of
preference pairs in v1, since ticket 19 ruled policy denials out in writing: `user-rejected` in a
recorded run is Hydra's own policy answering, and training on it teaches a model to obey a config
that changes.

**Nobody has established that the producer exists.** Ticket 14's evidence pass said a human
rejecting a diff "has no producer" and ticket 14 then recovered the *rejection* by subtraction:
interactive plus `user-rejected` is a person. That gives a rejection. The event needs a **pair**,
the rejected content and the chosen content, in the same trajectory, or preference export has
nothing to emit.

Establish, per provider, from the vendor's own transcript rather than the stream:

- **`claude`**, `~/.claude/projects/<slug>/*.jsonl`. Does a rejected tool call and whatever happened
  next survive in a form where the two can be paired? Is there a marker distinguishing a human's
  rejection from an auto-deny, given the label is the same string?
- **`cursor-agent`**, the per-chat directories under `~/.cursor/chats`.
- **`agy`**, `conversation_summaries.db` and the `brain` directory under `~/.gemini/antigravity-cli/`,
  plus `transcript_full.jsonl` at the path ticket 26 found.

For each, do it by hand: run an interactive session, reject a tool call, let the agent try something
else, then read what the transcript holds.

Then answer the question the contract cannot:

- **If at least one provider produces a pair**, say which, and what the harvest rule is per provider.
- **If none does**, `correction` has no producer and section 7.1's own rule says it comes off the
  list, taking the event count from ten to nine and leaving `preference.jsonl` with no source in v1.
  Say so plainly rather than leaving a type in the contract that nothing writes. Ticket 19's second
  corpus, rung one's merged-or-dropped proposals and rung two's panel verdicts, is then the only
  preference signal Hydra has, and it is currently fog.

Output: `research/34-correction-producer.md`.
