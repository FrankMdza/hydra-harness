# 259 - A `codex` record says what `codex` did

Type: build
Status: resolved
Blocked by:

## What

Build 255 (2026-10-03) harvested `codex` session files and its run found three ways a `codex`
record misreports, two of them in the streamed `exec --json` path too:

1. **Every tool result is `unclassified`.** The `exec` tool's output is script text, and the
   reader found no exit status it trusted, so a `completed` run derives as `unknown`. Measure
   from session files under `~/.codex/sessions/` and from the `exec --json` fixtures in
   `tests/fixtures/records/` whether an exit code is carried anywhere (`custom_tool_call_output`
   text, an `event_msg` such as `exec_command_end`, `item.completed` fields). If it is, classify
   from it in `normalize/codex.rs` for both paths; if it is not, say so and leave it.
2. **A session that hits its usage limit reads `completed`.** The session file ends the turn
   with `task_complete` carrying an error; `outcome.rs` reads the limit only off `turn.failed`.
   Make the harvested record carry the limit the same way the streamed one does, so the outcome
   is `rate_limited`. Find the exact field on the one session file that hit its limit (2026-09-25,
   written by 0.154.0, the only `source: cli` file there).
3. **`input_tokens` includes the cache.** On `codex`, `total_tokens` = `input_tokens` +
   `output_tokens`, so `cached_input_tokens` sits inside `input_tokens`. Contract 9.1 says input
   is net of cache on every provider. Fix the mapping in `counts_of` (one function, both paths):
   `input_tokens` = reported input minus cached, never below zero. Check `hydra usage recount`
   (build 31) and say whether it re-derives old `codex` records from their raw lines, which would
   correct them, or not.

## Done when

- Tests on synthetic lines built from the measured shapes (copy no user content): a tool result
  with a nonzero exit reads `error` and a zero one reads `ok`, if step 1 found a field; a session
  ending on the limit derives `rate_limited`; usage with 1000 input of which 800 cached maps to
  200 input and 800 cache read. Each fails with its change reverted; say so. fmt, clippy over the
  workspace, `cargo test --lib`, `cargo test --test normalize --test outcome --test harvest
  --test usage` and `record_contract` are clean. `IMPLEMENTATION.md`: one paragraph.

## Outcome

Resolved 2026-10-03 by run `01a103b8-a3fd` (`claude` opus 5.5, `standard`, 4.43M tokens), applied
unchanged after a measurement by the session.

Built: on the streamed path an `exit_code` of 0 reads `ok` and a nonzero one `error`, and a
command that ran and failed is never read as a refusal (`ran_and_failed`, `refusal_of`). On the
harvested path a `task_complete` with an `error` becomes the run's result, and `outcome.rs` reads
`codex_error_info: "usage_limit_exceeded"` (or the old sentence) as `rate_limited`. `counts_of`,
shared by both paths, counts `input_tokens` net of `cached_input_tokens`, as contract 9.1 says.
Three new tests; the agent reverted each part and saw them fail.

Measured by the session: over the 128 `codex` records in `runs/`, built from this diff and from
master, no outcome changed; total tokens fell, 147,824 to 76,016 and 465,037 to 254,605 on two of
them, because the cache had been counted inside input and again as cache read. Every `codex`
total before this build was inflated by roughly its cache reads; map 97's comparisons used Sonnet
and Opus copilots against mostly Sonnet agents, so they are not touched.

Assumed and false: harvested tool results stay `unclassified`, because no id ties a
`CommandExecution` exit code to a call and the script's own `Script failed` line disagrees with
its commands on 82 of 1,440 calls; reading that line is a decision. `hydra usage recount` needed
no change: every reader derives counts from the raw lines. Left for build 260: `hydra doctor
--walk` would flag a harvested limit record, and a harvested session whose last turn ended on a
non-limit error reads `completed`.
