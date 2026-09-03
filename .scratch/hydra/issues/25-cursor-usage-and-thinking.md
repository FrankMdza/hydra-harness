# 25 - Cursor's usage and thinking fields, which ticket 09 gets wrong

Type: research
Status: resolved

## Question

Ticket 09 is resolved and one of its facts has expired. Ticket 14's evidence pass found that the
2026.08.31-4057e58 `cursor-agent` emits a `usage` block with four token fields, and emits `thinking`
in print mode. Ticket 09 says Cursor emits no token counts, gives it `usage_source: estimated` with
a model-aware tokenizer, and lists `thinking` as populating from everything except Cursor.

Ticket 16 writes the record contract from ticket 09, so this has to be settled before it runs.

Establish against the installed build, live:

- **Whether `usage` is really emitted, in which output formats, and on which frames.** `json` and
  `stream-json` may differ, and a stream may carry per-step usage as well as an aggregate the way
  `agy` does.
- **The exact field names and what they count.** Ticket 18 found `agy` separates thinking and
  cache-read tokens; say whether Cursor's four are the same four, because ticket 09's normalized
  `usage` event has to hold both.
- **Whether the counts are trustworthy.** A field that is present and always zero is worse than an
  absent one, since it would flip `usage_source` to `provider` and silently stop the estimator.
  Run something with real input and check the numbers move.
- **Whether `thinking` appears in print mode**, in which format, and whether it is the model's
  reasoning or a summary of it.
- **Whether a failed or rejected run still reports usage.** Ticket 14 found Cursor returns
  `is_error: false` with a tool rejected, so check what usage says on that path.

Then say precisely what changes in ticket 09: whether Cursor moves from `estimated` to `provider`,
whether `usage_estimator` still has a producer, and whether the `thinking` sentence needs rewriting.

Keep the runs small and report what they cost. Output:
`research/25-cursor-usage-and-thinking.md`.

## Answer

Findings: [research/25-cursor-usage-and-thinking.md](../research/25-cursor-usage-and-thinking.md).

`cursor-agent --version` printed `2026.08.31-4057e58` on 2026-09-02 at 16:51, the same build ticket
14 probed. The binary did not move under this ticket.

Ticket 14 was right on both counts, and two details it did not have change how ticket 09 should be
fixed.

**Cursor moves from `usage_source: estimated` to `provider`.** It emits
`usage: {inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens}` on the `result` frame, in
`--output-format json` and in `--output-format stream-json` alike, on the success path and on the
tool-rejection path. Only on that frame, never per step, so Cursor does not behave the way `agy`
does. `--output-format text` reports nothing. The numbers move: 74,813 bytes of added input took the
raw input from 16,933 tokens to 37,864.

**`usage_estimator` keeps a producer, so the field stays.** Three Cursor cases still report no
counts. A run that fails hard writes no `result` frame at all, because the print-mode `catch` prints
to stderr and exits 1, which is every Cursor failure by design. `--output-format text` never carries
usage. And the emitter appends the block conditionally, `we ? {usage:we} : {}`, so a successful run
may omit it. Treat `usage` as optional on the success path too, not only on failure.

**Two traps for the normalized `usage` event.** Cursor's `inputTokens` is net of cache and clamped,
`max(inputTokens - cacheReadTokens - cacheWriteTokens, 0)`, so a warm cache reports 0 input on a turn
that really consumed tens of thousands. Two runs of one identical prompt reported 14,629 and 11,557
input, both summing to 16,933 with their cache read. Store the parts and derive the total; do not
trust the single field. Separately, `cacheWriteTokens` read 0 in all seven captures across both
tickets, and I could not make it move.

**Cursor's four are not `agy`'s four.** They overlap on input, output and cache-read only, and even
there the semantics differ, since `agy` reports full input with cache read beside it. Cursor adds
`cacheWriteTokens`, which `agy` lacks. Cursor has no total and no reasoning-token count. That last
one is a drop rather than an absence: the internal `TurnEndedUpdate` carries `reasoningTokens` and
the print-mode writer never reads it.

**Ticket 09's `thinking` sentence needs rewriting.** "never from Cursor" is false. `stream-json`
emits `thinking/delta` frames carrying the reasoning text with a `timestamp_ms`, no flag required,
and a `thinking/completed` marker with neither text nor duration. `json` carries the same text as
`result.thinking_blocks[] = {text, duration_ms}`, but only behind `--show-thinking`, a flag
registered with `.hideHelp()` and therefore absent from `--help`. Choosing `stream-json` costs the
per-block duration and nothing else.

The rewrite should also split a distinction ticket 09 does not currently make. Reasoning text and
reasoning tokens are separate capabilities, and the providers split differently on them: `agy`
reports `thinking_tokens` and no text on the stream, Cursor reports text and no token count.

Whether Cursor's thinking text is the model's own trace or a provider summary is not answerable from
the CLI, and `model` reports the display name `"Auto"` rather than an id, so record it as
provider-reported reasoning text and claim no more.

Five paid runs on the subscription, 140,478 raw input tokens and 583 output tokens, 69 seconds of
wall clock. Cursor reports no cost field, so there is no dollar figure.

## Version note, 2026-09-02 17:05

`cursor-agent` self-updated to `2026.09.02-c22c1a3` within the hour after this ticket closed. Every
measurement above is against `2026.08.31-4057e58`. Nothing here is known to have changed, and
nothing here has been rechecked.
