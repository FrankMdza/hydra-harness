# 201 - The cockpit rechecks damage at `y` and masks spaced secrets

Type: build
Status: resolved
Blocked by:

## What

A read-only review by `codex` (gpt-6-astra) on 2026-10-01 of builds 198 (`d09d459`) and 200
(`7c6d4e1`) found six defects, four of them here and two in build 202 in the cockpit. It found none for a BOM, CRLF, duplicate keys or
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
   A first run of this ticket (codex, `01a0f5ee`, stopped on item 5 before 199 had landed)
   wrote that test and saw a buffer bounds panic at height 3. No terminal size may panic.
Items about `removed` on a `repair` event and the `\r` test moved to build 202, which waits on
199 because they copy the helper's rule from it.

`IMPLEMENTATION.md`: the cockpit passages on the damaged goal, the masking and the policy pane.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f5f2` (codex gpt-6.1-sol, `standard`, 1.6M tokens), diff read whole
by the session and applied to master as written. An earlier run, `01a0f5ee`, stopped without a
change because the ticket then held an item that cited a rule build 199 had not landed; that item
is build 202.

Built: `approve_step` and `start_review` check the goal again at `y`, close the confirmation and
show the repair message; a secret name may hold `.`, takes `:` as well as `=` before its value,
and as a `--flag` takes a value after a space; a value in backticks or `$(...)` is masked whole,
nested quotes and parentheses included; a line that is JSON but for `NaN`, `Infinity` or
`-Infinity` counts as skipped and is not damage; when no whole run fits the policy pane it shows
one run clipped; the status line draws nothing in an empty area, which was the panic at height 3.

Measured on master: fmt and clippy clean over the workspace, `cargo test --test cockpit` 92
passed, `cargo test --lib` 224 passed. The session reverted `app.rs` and `snapshot.rs` and saw
four of the six tests fail (both confirmations, the secrets, the small pane), then restored them.

Over-masking the run reports, on purpose: the values of `--key-file`, `--keyboard` and
`--pass-through` are hidden, and any `name: value` whose name holds `KEY`, `TOKEN`, `SECRET` or
`PASS`.

Left open: a short flag (`-k value`) has no name to match and is not masked.

