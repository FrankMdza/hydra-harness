# 209 - A codex usage limit with a curly apostrophe is `rate_limited`

Type: build
Status: open
Blocked by:

## What

On 2026-10-01 three `codex` runs (`01a0f653`, `01a0f650`, `01a0f654`) ended when the account
ran out of quota. Each record's last frames are `{"type":"error","message":"You\u2019ve hit your
usage limit. ... try again at 4:48 AM."}` and a `turn.failed` with the same message, and each
derived `failed`. `src/outcome.rs` near 434 matches `You've hit your usage limit` with a straight
apostrophe; codex now writes U+2019. So the cockpit's lane, rung one's corpus and the grader all
read a quota stop as a failed task.

## Do

A test first in `tests/outcome.rs` with a synthetic record whose `turn.failed` message carries
U+2019, shown failing before the change. Match on `hit your usage limit`, which does not depend
on the apostrophe, and keep the old sentence passing. Check `cursor-agent`'s text match in the
same file for the same weakness and say what you found. `IMPLEMENTATION.md`: the sentence on
`turn.failed`.

## Done when

- The test failed before the change; fmt, clippy, `cargo test --lib`, `cargo test --test outcome`
  pass.
