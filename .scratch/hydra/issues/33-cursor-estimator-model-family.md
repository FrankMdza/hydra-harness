# 33 - What the Cursor usage estimator keys on, when `model` says "Auto"

Type: research
Status: open
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
