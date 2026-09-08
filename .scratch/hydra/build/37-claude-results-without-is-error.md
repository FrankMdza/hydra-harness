# 37 - A `claude` tool result with no `is_error` is classified from `tool_use_result`

Type: build
Status: resolved
Blocked by: 03

## Goal

On `claude` 2.1.263, a successful `Read`, `Edit` or `Write` result arrives with no `is_error`
field, and the normalizer files it `unclassified`, contract 7.6. On this machine that is half
the results of every Sonnet run that did real work: run 01a07f37-9cab has 159 tool results and
92 of them are unclassified, so its `outcome_source` is `unknown`, `hydra evolve --dry-run`
lists it as `completed/unknown`, and `hydra export` will refuse it for `unknown_outcome`. The
same frame carries a second field that says what happened. Read it.

## Where the design says it

- `docs/run-record-contract.md` section 7.6, the result status and why an absent `is_error`
  must not read as success; section 8.2 for `outcome_source: unknown`.
- `src/normalize/claude.rs`, the `tool_result` arm that matches `is_error`, with the 2.1.261
  measurement in its comment.
- `IMPLEMENTATION.md`, the `claude` normalizer section.

## The measured shapes, 2.1.263, 2026-09-08, runs 01a07f37-9cab and 01a07f18-910f

The `user` frame that carries a `tool_result` content item also carries a top-level
`tool_use_result` beside `message`. Tallied by tool, `is_error` and that field's shape:

- `Bash`: `is_error: false` on every success, `tool_use_result` an object with `stdout`,
  `stderr`, `interrupted`, `isImage`, `noOutputExpected` and sometimes `returnCodeInterpretation`
  or `persistedOutputPath`; `is_error: true` on failure with `tool_use_result` a string
  starting `Error: `.
- `Read`: `is_error` absent on success, `tool_use_result` an object `{"type": "text", "file":
  {"filePath": ..., ...}}`; `is_error: true` on a refusal with the string `Error: hydra/...`.
- `Edit`: `is_error` absent on success, `tool_use_result` an object with `filePath`,
  `oldString`, `newString`, `originalFile`, `replaceAll`, `structuredPatch`.
- `Write`: `is_error` absent on success, `tool_use_result` an object with `type: "create"`,
  `filePath`, `content`, `originalFile`, `structuredPatch`.

No success was seen with `tool_use_result` as a string and no error with it as an object.

## The slices

**The rule.** In the `tool_result` arm: `is_error` present decides as today. Absent, read the
frame's top-level `tool_use_result`: an object is `Ok`; a string starting with `Error:` is
`Error`; anything else, absent included, stays `Unclassified`. The frame is the `user`
envelope the arm is already inside, so the field is one level up from the content item; thread
it through however the arm is called. The doc comment keeps the 2.1.261 sentence and adds the
2.1.263 shapes above, dated.

**A finding when the rule fires.** None. A classified result is the ordinary case, and contract
7.6's rule against reading absence as success is honoured because the object is presence.

**Tests.** `src/normalize/claude.rs` unit tests or `tests/normalize.rs`: a `Read` frame with
no `is_error` and an object `tool_use_result` classifies `Ok`; the same with a string
`Error: x` classifies `Error`; the same with no `tool_use_result` at all stays
`Unclassified`; a `Bash` frame with `is_error: false` and a string `tool_use_result` is still
`Ok`, since `is_error` wins. The corpus walks must still pass.

## Done when

- `hydra show 01a07f37-9cab-71c2-b114-73bb18b2b23a` prints `"outcome_source": "derived"` and
  `hydra evolve --dry-run` lists it as `completed/derived`.
- The tests above pass and the rest of the suite is untouched.
- The orchestrating session amends the `claude` normalizer section of `IMPLEMENTATION.md`.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Cursor Grok 4.6 high fast, run 01a08067-548b under
`standard`, launched through `hydra-in-pane` on the first try after the launcher fix: 2.57M
tokens, two files, five tests, twenty-five minutes. `result_status` in `src/normalize/claude.rs`
reads `is_error` when present and the user frame's `tool_use_result` when not, an object as
`Ok`, a string starting `Error:` as `Error`, anything else `Unclassified`; `message_blocks`
takes the frame so the field one level up is visible, and harvest shares it. On master,
`hydra show 01a07f37-9cab` prints `outcome_source: derived` where it printed `unknown`, and
`hydra evolve --dry-run` still lists twelve runs as `/unknown`, which are the Cursor runs
carrying an `unknown_tool` and the older records, not this rule. The model could not run the
done-when's two commands itself because `~/hydra/runs/` is denied under every policy, and said
so rather than guessing.

**What its own record showed.** One `Disagreement` on `awaitToolCall`: the replay derives
`allow` from ticket 35's `unhooked` row, and the gate at run time said `out-of-scope` for
`file_read`, the third run in which that tool's result carried Hydra's refusal. So that tool
is hooked after all, and the row is wrong for it; the probe run 01a08111-1e5e measures what
the hook sees.
