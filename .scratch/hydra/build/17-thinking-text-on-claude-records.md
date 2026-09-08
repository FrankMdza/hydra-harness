# 17 - `claude` runs record their thinking text again

Type: build
Status: resolved
Blocked by:

## Goal

Every `claude` record on this machine carries thinking blocks with a signature and an empty
string where the text was. Contract 7.5 says `claude` gives text and count, and it did until
Claude Code 2.1.72 started sending a `redact-thinking-2026-02-12` beta header that empties the
block before it reaches the stream. The text comes back through one settings key, measured today.
Land it on every recorded `claude` run, say so in the contract, and put a test on the binary so
the next regression is a failing test rather than a discovery.

## What was measured

2026-09-07, `claude` 2.1.263, four probes on haiku, `--print --output-format stream-json
--verbose`, task "Think hard step by step, then answer: what is 17*23?":

| argv | thinking blocks | text chars | `thinking_tokens` |
|---|---|---|---|
| nothing | 1 | 0 | 190 |
| `--settings '{"showThinkingSummaries":true}'` | 1 | 504 | 266 |
| `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING=1` | 1 | 0 | 181 |
| both | 1 | 272 | 173 |

The env var does nothing. The settings key is the whole fix. What comes back is the summary
Anthropic generates, which is the only thinking text a Claude 4 model ever emits.

Across the 27 records in `runs/`: 43 `claude` thinking blocks, none with text; 35 `cursor-agent`
blocks, all with text. Issue 32810 on anthropics/claude-code dates the break to 2.1.72, issue
20127 to an earlier one at 2.1.8, and issue 56356 reports Opus 4.7 ignoring the display setting
entirely. So it is per model and it has regressed twice.

## Where the design says it

- Contract 7.5, the `claude` row: "`thinking` content blocks, with `signature`". True again
  after this ticket, false on every record before it.
- Harness spec 6.6, the hook config `--settings` points at. The key rides in the same file.

## The slices

**The key.** `settings()` in `src/provider/claude.rs` gains `"showThinkingSummaries": true` at the
top level beside `hooks`. A unit test in the same file's `tests` module reads the key back. Nothing
else changes: the run already passes `--settings` to this file on every policy.

**The contract.** 7.5 gets a paragraph: the text is a summary, it is off by default since 2.1.72,
Hydra turns it on through the spawn settings, and a record written before build ticket 17 has
empty blocks with a signature and a non-zero count. The count was always right.

**The drift test.** One test in `tests/drift.rs` that writes `claude::settings()` to a scratch
file, runs the binary with `--settings` on that file, haiku, stream-json, and asserts one thinking
block with non-empty text. It costs a few hundred tokens, so it runs only when `HYDRA_DRIFT_PAID=1`
is set and skips otherwise, and it skips without `claude` on the path like the two beside it. The
assertion message names the version the binary reported.

**The implementation file.** A note under the `claude` sections of `IMPLEMENTATION.md` with the
table above and the two caveats: summary rather than trace, and Opus 4.7 unmeasured.

## Done when

- `claude::settings()` carries the key and a unit test says so.
- A recorded `claude` run on haiku writes a thinking block with text. Check the record.
- Contract 7.5 says what changed and when.
- The drift test passes with `HYDRA_DRIFT_PAID=1` and skips without it.
- `IMPLEMENTATION.md` carries the measurement.

## Outcome

Resolved 2026-09-07. All four slices built, and the ticket was the first one whose code two
other providers wrote through `hydra run` while a `claude` session orchestrated and reviewed.

**The key.** `claude::settings` carries `"showThinkingSummaries": true` beside the hook config,
and `thinking_text_is_turned_back_on_in_the_settings` reads it back. Written by `cursor-agent` on
run 01a07eed-700a under `standard`: one file, 17 insertions, 162k tokens of which 132k were cache
reads, 26 unit tests passing in its worktree. Applied to master by hand from the worktree diff;
the doc comment it wrote was reworded because it carried parentheses and a comma splice.

**Measured on a recorded run.** Run 01a07eef-329c, haiku under `read-only`, with the rebuilt
binary: one thinking block, 360 characters of text, `thinking_tokens` 108. Every `claude` record
before it holds an empty string in the same place.

**The drift test.** `a_claude_run_still_writes_its_thinking_text` in `tests/drift.rs`, written
by `agy` on run 01a07eed-9c4a under `standard`. It skips without `HYDRA_DRIFT_PAID=1`, skips
without `claude`, writes `claude::settings()` to a scratch file, runs haiku on it with a
120-second kill, and names the version and the token count in its failure. Passes both ways on
2.1.263. `agy` spent 50 steps reading before it wrote a line, 24 `view_file` and 24
`run_command`, and then wrote the whole test in one go; the reading was the cost of a long
prompt on a provider that takes it literally.

**The contract and the implementation file.** 7.5 says the text is a summary, off since 2.1.72,
turned on by the spawn settings, and empty on records before this ticket. `IMPLEMENTATION.md`
carries the four-probe table under "What the record does not carry".

**What the ticket assumed that held.** The env var was measured useless before the ticket opened
and stayed useless. What it could not check: Opus 4.7, reported ignoring the setting upstream,
is unmeasured here because no run on this machine has used it.
