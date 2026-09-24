# 115 - Three comments in `src/` that build 102 found false

Type: build
Status: claimed
Blocked by:

## What

Build ticket 102 rewrote the documents after the third panel and, on the way, found three doc
comments in `src/` that say something the corpus has since disproved. It did not touch `src/`,
so they are still there. Each is a comment, not code; nothing compiles differently after this.

1. `src/outcome.rs`, the module doc, the sentence "The three providers measured, `claude`,
   `cursor-agent` and `agy`, exit 0 and report success on a run the gate refused (`codex` is
   unmeasured)". Run 01a0c62b-d005 measured `codex` on 2026-09-21: it also exits 0 on a run the
   gate refused. `docs/harness-spec.md` already says so (`grep -n "exits 0" docs/harness-spec.md`).
   Make it four providers measured and drop the parenthesis.
2. `src/outcome.rs`, the doc of `fn codex_limit_refusal`: "Before this reading all three derived
   `failed`, and rung one learned from them." Build ticket 100's Outcome says 01a0cc9d-8f15, the
   one whose sentence hides behind "Error running remote compact task: ", derived `completed`
   before, not `failed`, because its answer was finished when the quota ran out. Say that: two
   derived `failed` and one `completed`, and rung one learned from all three.
3. `src/rungs/evolve.rs`, the doc comment that describes what admission denies and names the two
   `.cargo` files (`grep -n "\.cargo" src/rungs/evolve.rs`). Build ticket 105 added a deny by name
   beside those two; `grep -n "build ticket 105" IMPLEMENTATION.md` and read 40 lines around the
   rung two passage to see what it is called. Name it in the comment.

## Do

Edit those three comments and nothing else. Do not build: this ticket runs on a machine shared
with other agents, and a comment change needs no compiler. Run `cargo fmt --check` only.

## Done when

The three comments say what the corpus and the tickets say, in the file's own voice, no em
dashes. The report quotes each comment before and after.
