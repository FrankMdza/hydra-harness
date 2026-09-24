# 119 - Stale "three providers" comments, and the authorship rows 108, 109 and 112

Type: build
Status: open
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
