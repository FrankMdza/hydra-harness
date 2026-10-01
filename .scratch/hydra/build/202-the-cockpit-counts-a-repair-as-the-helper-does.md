# 202 - The cockpit counts a repair as the helper does

Type: build
Status: open
Blocked by: 199

## What

Two items split from build 201 because they copy a rule build 199 gives the helper. From the
`codex` review of builds 198 and 200 on 2026-10-01.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change unless the item says
otherwise.

1. **`removed` on a `repair` event** (`goal.rs` near 208). Read `fold` in `puppets.py` and
   `crates/puppets/src/fold.rs` first: the rule since build 199 is the
   helper's: `removed` counts when it is an int from 1 to 1,000,000,000, else zero, and the
   revision then adds one. `as_i64()` on `9223372036854775808` gives a different answer. Use that
   rule here.
2. **Lines split on `\n` only.** The cockpit already does; since build 199 the helper does too,
   so `{}\rjunk\n` is one bad line in both. Add the test that pins it, and nothing else.

`IMPLEMENTATION.md`: the cockpit passage on the damaged goal, if it names the revision.

## Done when

- fmt, clippy over the workspace, `cargo test --lib` and `cargo test --test cockpit` pass.
