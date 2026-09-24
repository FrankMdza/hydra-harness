# 119 - Stale "three providers" comments, and the authorship rows 108, 109 and 112

Type: build
Status: resolved
Blocked by:

## What

Two leftovers the fourth panel's `claude` reader listed under "Before publishing", items 6
and 7 (`docs/reviews/2026-09-24-fourth-panel-claude.md`).

1. Comments that still say three providers, or that `codex` is unmeasured, after build 87
   routed `codex` and run 01a0c62b-d005 measured it on a gated exit: `src/policy/gate.py`
   near 17 to 19, `src/mcp.rs` near 4, 55 and 80, `src/policy/mod.rs` near 965,
   `src/normalize/mod.rs` near 106 and 265. Build 115 fixed the two in `src/outcome.rs`; read
   its diff for the voice. `grep -rn "three providers\|codex.*unmeasured\|unmeasured.*codex" src/`
   finds any the list misses. Read each before editing: a sentence that names a measurement
   from before `codex` existed may be right as history and only need a date.
2. `README.md`, the authorship table near lines 240 to 266. Its rows sum to 106 tickets and the
   paragraph above it says 112 resolved. Rows for 108 (`composer-2.5`), 109 (`codex`) and 112
   (an Opus 5.5 subagent, no run id) are missing; their Outcomes under `.scratch/hydra/build/`
   carry the run id and token count where there is one. Add the rows and make the paragraph and
   the rows use one denominator. Nothing else in that file.

## Done when

`grep -rn "three providers" src/` returns only sentences that are true as history and dated.
The table's rows sum to the number the paragraph gives. `cargo fmt --check` clean. Comments
and a table only: run `cargo test --lib` once and nothing more.

## Outcome

Built by `cursor-agent` on `composer-2.5` under `standard`, run 01a0d3f8-5272, 0.86M tokens.
Applied to master by hand except the README hunk, which the session redid because the table had
moved under it (`~/.claude/handoffs/2026-09-24-panel4/diff-119.patch`). Thirteen files: the
agent went past the ticket's list by grep, as invited, and left `src/normalize/codex.rs:116`
("the other three providers") alone, correctly.

Four of its sentences said more than the corpus does and the session rewrote them: `codex`
does not read `.codex/hooks.json` from a worktree (its hook rides its argv,
`src/provider/codex.rs`), so `src/policy/mod.rs`'s deny-list doc says two of four again and why
`codex` is not the third; `src/paths.rs` no longer counts the providers a wrong `HOME` logs out;
`src/logs.rs` says a print-mode provider cannot be asked anything mid-run rather than "two of
four"; `src/mcp.rs`'s `undeliverable` doc names the three that answer on the tree instead of
"the other three ... both". The README rows: 108, 113, 114, 116, 117, 119 and 120 under
Composer, 115 under Gemini 3.8 Flash, 109 under `codex`, 112 under the Opus subagent, and one
denominator, 67 of 116. `cargo fmt --check` and clippy clean; lib 210.
