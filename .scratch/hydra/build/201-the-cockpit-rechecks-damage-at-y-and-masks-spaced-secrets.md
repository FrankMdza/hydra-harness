# 201 - The cockpit rechecks damage at `y` and masks spaced secrets

Type: build
Status: claimed
Blocked by:

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of builds 198 (`d09d459`) and 200
(`7c6d4e1`) found six defects in the cockpit. It found none for a BOM, CRLF, duplicate keys or
invalid UTF-8, and traced that `repair` brings the table back on the next snapshot.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. **A confirmation already open bypasses the damage guard** (`src/cockpit/app.rs` near 1004 and
   1297). Open an approval with `a`, or a review with `v`, on a healthy goal; a snapshot then
   marks the goal damaged; `y` still runs the helper, because the guard runs only when the
   confirmation opens. `approve_step` and `start_review` check again at `y`, close the
   confirmation and show the repair message.
2. **Secrets that still reach the row** (`src/cockpit/snapshot.rs` near 870, 906, 938). Each of
   these shows `s3cret`: `curl --api-key s3cret` (a flag and its value separated by a space),
   `curl -H 'x-api-key: s3cret'` (a header: a secret name, `:`, a value, inside or outside
   quotes), `{"api.key":"s3cret"}` (a name with a dot), and ``TOKEN=`printf s3cret` `` and
   `TOKEN=$(printf s3cret)` (a value that is a command substitution, masked only to the first
   space). Mask the whole value in each. Keep `--key-file path` style over-masking in mind and
   say in the report what ordinary flags the rule now hides.
3. **`NaN` and `Infinity`** (`src/cockpit/goal.rs` near 126). `{"event":"ignored","x":NaN}` is a
   good line to the helper (`json.loads` accepts it) and damage to the cockpit (`serde_json`
   does not), and `repair` cannot fix it because the helper sees nothing wrong. Such a line is
   not damage here either: it counts as skipped, or is read, and say which you chose.
4. **A page with one row** (`app.rs` near 1636). At 80x6 with no mascot and one live run on
   another sha, the pane has one interior row, the run needs two, and the row kept for `N more`
   leaves none: the only run cannot be reached. When a whole run cannot fit, show it clipped to
   the room there is. Test heights 3 to 12.
5. **`removed` on a `repair` event** (`goal.rs` near 208). The rule since build 199 is the
   helper's: `removed` counts when it is an int from 1 to 1,000,000,000, else zero, and the
   revision then adds one. `as_i64()` on `9223372036854775808` gives a different answer. Use that
   rule here.
6. **Lines split on `\n` only.** The cockpit already does; since build 199 the helper does too,
   so `{}\rjunk\n` is one bad line in both. Add the test that pins it, and nothing else.

`IMPLEMENTATION.md`: the cockpit passages on the damaged goal, the masking and the policy pane.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass.
