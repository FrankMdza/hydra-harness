# 176 - The token header survives a reset and follows sheet.py

Type: build
Status: resolved
Blocked by: 172

## What

A read-only review by `codex` on 2026-09-30 of build 172 (`eaa425a`) found the normalizer refactor
unchanged, byte for byte, and three defects in the header, each checked by arithmetic:

1. **A record that shrinks keeps its old baseline.** `src/cockpit/snapshot.rs` near 365 and 432:
   a live run at 1,000 output tokens whose file shrinks to a record at 10, then 110 a minute
   later, shows 0 tok/min; the old opening timestamp survives too, and an equal-length replacement
   keeps stale counts entirely.
2. **The claude exclusion comes before aliases.** `src/cockpit/window.rs` near 99 checks the
   recorded provider; `sheet.py` checks the canonical one, so an alias mapping onto `claude` gives
   a share Rust should not show.
3. **A test that supplies its own answer.** `tests/cockpit.rs` near 1721,
   `a_null_budget_and_claude_show_a_dash_for_the_window`, builds a `Spend` with `window_share:
   None` and only tests formatting; removing the claude exclusion still passes.

## Do

Tests first, each shown failing before the change.

1. A cached run is keyed by more than its length: when the file shrinks, or its line 0 changes,
   drop the cached counts and samples and reread the opening timestamp. Test the three inputs
   above.
2. `window.rs` canonicalizes first, then excludes `claude`, exactly as `sheet.py` orders it; a
   test with an alias onto `claude` and a positive budget.
3. Rewrite the test in 3 so it computes the share for a `claude` run with counts and a positive
   budget, and asserts `-`; show it failing with the exclusion removed.
4. `IMPLEMENTATION.md`, the header paragraph, if anything it says changes.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f30a-e273 under `standard`. The session
applied it as written. A live run's cache now keeps its line 0 and drops counts and samples when
the file shrinks or line 0 differs, rereading the start. `Budgets::share` canonicalizes first and
then excludes a canonical `claude`, as `sheet.py`'s `build` does. The claude test now computes a
share with real counts and a positive budget, and fails with the exclusion removed. Five tests,
each failing first; fmt and clippy clean, `--test cockpit` 39, `--lib` 221 on master. Left: a
replacement with the same length and the same line 0 would still keep stale counts.

