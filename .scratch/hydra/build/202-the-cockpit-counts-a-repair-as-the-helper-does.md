# 202 - The cockpit counts a repair as the helper does

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-01. Run `01a0f61c` (cursor-agent composer-2.5, `standard`, 1.0M tokens), diff
read whole by the session and applied to master as written.

Built: `repair_removed_count` in `src/cockpit/goal.rs` counts `removed` only when it is a JSON
integer from 1 to 1,000,000,000, the helper's rule since build 199, and a test with the same
cases as `crates/puppets/src/fold.rs`, which failed before on `1000000001`. The lone `\r` test
passed before the change, as the ticket said it would: the cockpit already split on `\n` only.

Measured on master: fmt and clippy clean over the workspace, `cargo test --test cockpit` 103
passed, `cargo test --lib` 224 passed.

