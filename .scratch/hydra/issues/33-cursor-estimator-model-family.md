# 33 - What the Cursor usage estimator keys on, when `model` says "Auto"

Type: research
Status: resolved
Blocked by:

## Question

Surfaced writing `docs/run-record-contract.md`. Ticket 09 gave `usage_source: estimated` a design:
a model-aware tokenizer, picked by model family, on the grounds that `cursor-agent` emits `model` on
its `system/init` frame so the estimator "picks a tokenizer by model family instead of guessing".

**Ticket 25 found `model` reports the display name `Auto` rather than an id.** So the input the
estimator selects on may not exist in the record at all, and the design falls back to exactly the
guessing it was written to avoid.

This is not a corner. Every hard `cursor-agent` failure lands here: its print-mode `catch` writes to
stderr and exits 1 without a `result` frame, so no run that fails hard reports counts. The emitter
also appends the usage block conditionally on the success path, so a successful run can omit it too.

Establish, against the installed build:

- **What `model` actually reports across runs.** Always `Auto`, or does it ever resolve to an id?
  Check `system/init` and the `result` frame, and check whether passing `--model <id>` explicitly
  changes what is reported.
- **Whether the resolved model is recoverable anywhere else**: another frame, the chat directory
  under `~/.cursor/chats`, or a free subcommand.
- **How wrong a family-agnostic tokenizer is.** Take a run where Cursor did report counts, estimate
  the same text with a generic tokenizer, and report the error. If it is within a few percent the
  whole question shrinks to a footnote.
- **Whether `estimated` should survive at all on this provider**, or whether a run with no counts
  is honestly `unavailable`. Ticket 09 rejected `unavailable` for Cursor on a premise that has since
  expired twice.

Then say what changes in `docs/run-record-contract.md` section 7.8 and section 9: whether
`usage_estimator` keeps a producer, and what it records when the family is unknown.

Keep the runs small and report what they cost. Output:
`research/33-cursor-estimator-model-family.md`.

## Answer

**The resolved model is recoverable, and not from the stream.** Findings in
[research/33-cursor-estimator-model-family.md](../research/33-cursor-estimator-model-family.md),
measured 2026-09-04 on `cursor-agent` 2026.09.02-c22c1a3 across four runs.

**`model` is the selection's display name.** Under `--model auto` it says `Auto` and the served
model never appears in any frame. Pinned, it reports a display name that matches neither the id nor
the `--list-models` label: `claude-sonnet-5-thinking-high` is listed as `Claude Sonnet 5 1M Thinking`
and reported as `Claude Sonnet 5 300K High`. Three names, no published mapping. The leading vendor
word is the only thing the display name reliably gives, and `Auto` does not even give that.

**Ticket 25's `Auto` reading has already expired, without a version change.** The same build now
reports `Composer 2.5`, because the account's selection changed. The field tracks config, so its
value has a shelf life measured in whatever the user last clicked.

**The id lives in the chat store**, at `providerOptions.cursor.modelName`, per assistant message, in
the directory named by the run's `session_id`. The Auto probe resolved to `cursor-grok-4.5-high`, a
model `--list-models` does not offer. So ticket 09's model-aware tokenizer is buildable, at the cost
of reading a second source at harvest, in the two formats ticket 34 found.

**The premise underneath the estimator is the part that fails.** Summing every text, result and
reasoning string in a run's store against the provider's own input count gives 2.11 and 1.93
characters per token, against roughly 4 for prose and 3 for structured text, with under 2.7 KB of
unread blobs to explain the gap. Cursor counts about twice what the transcript keeps, most likely
tool schemas that no chat store records. An estimator over recoverable text is low by half, no
tokenizer choice closes that, and the only ground truth is computed over an input Hydra cannot see.

So the recommendation to ticket 16 is that **`estimated` does not look earned on this provider and
`unavailable` says the true thing**, which is what 7.8's "never a bare null" rule exists to protect.
If `estimated` survives anyway, `usage_estimator` should carry the resolved `modelName` and record
that it covers recoverable text only.

**One bonus, reproducible until 9/29.** Pinning any premium model on this Pro account is refused
before a model runs: exit 1, `system/init` and `user` written, no `result` frame, no usage. That is
the `catch` path ticket 25 read in the emitter and never triggered. `model` is present on those runs
while usage is not, so the two fail independently and the record cannot assume one implies the other.
