# 34 - Does any vendor transcript record a correction as a pair?

Type: research
Status: resolved
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

## Answer

**One provider of three produces the pair: `claude`.** Findings and evidence in
[research/34-correction-producer.md](../research/34-correction-producer.md), measured 2026-09-04 on
`claude` 2.1.257, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.26.

`correction` keeps its producer and stays on the list, so the event count stays at ten and
`preference.jsonl` keeps a source. But section 7.1's column carries one provider rather than three,
and the two gaps have named reasons rather than being left blank.

**The harvest rule on `claude`.** A denial writes a `user` record carrying `toolDenialKind`,
`userFeedback` and `sourceToolAssistantUUID`. Rejected content comes from following
`sourceToolAssistantUUID` and matching the `tool_use` block whose `id` equals the denial's
`tool_result.tool_use_id`, which succeeded on 35 of 35 denials in the corpus. Chosen content comes
from walking children by `parentUuid` to the first assistant `tool_use`, and a pair is emitted only
when the tool name matches. The human's words, when they typed any, are in `userFeedback`.

**The discriminator is a conjunction, not a field.** The binary maps `behavior === "ask"` to
`toolDenialKind: "user-rejected"`, so that value means the ask path answered, not that a person did.
Under `entrypoint: sdk-cli` the answerer is a permission callback, which is code, and all three
`user-rejected` records in the corpus are exactly that: ticket 30's own harness. The human test is
`toolDenialKind == "user-rejected"` **and** `entrypoint == "cli"`. A harvester trusting the field
name alone labels a scripted deny as human preference.

**The other two.** `cursor-agent` pairs fine by `toolCallId` but puts the whole refusal in a prose
`result` string with no denial field anywhere, so a human and a hook are separable only by whether
the gate happened to write prose. `agy` is worse: it drops the denial record from
`transcript_full.jsonl` entirely, leaving a `step_index` discontinuity and the model's own narration
as the only evidence, and its `tool_calls` carry no id to pair on.

**What ticket 19 already forbids.** Every pair measured here is a policy or automode denial, which
ticket 19 ruled out of preference export in writing. So the traversal is verified 35 times on data
the contract may not export, and the exportable population is empty so far.

**The gap, and it is the whole point of the ticket.** The corpus holds no human interactive
rejection on any provider, so the conjunction above has zero observed instances. The rule is derived
from the binary's own source and verified mechanically on auto-denies. Staging one from a script
cannot close it, because a script produces `entrypoint: sdk-cli`, the case the rule filters out.
Closing it needs four minutes at a keyboard, which is ticket 35.
