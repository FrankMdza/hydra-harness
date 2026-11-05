# 200 - The cockpit masks hyphenated names and pages with the dragon up

Type: build
Status: resolved
Blocked by:

## What

A read-only review by `cursor-agent` (grok-4.7-high) on 2026-10-01 of builds 179 (`a34526d`), 180
(`9748119`) and 192 (`0989ec4`) found five defects, two of them secrets that still reach the
screen. It confirmed the rest: the older mascot frames are byte-identical, `index.json` names
every frame, ids that repeat pair in order, and the cut never splits a masked value.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. **A JSON name with a hyphen** (`src/cockpit/snapshot.rs` near 867 and 873 to 896). `is_name`
   stops at `-`, so `{"api-key": "s3cret-value"}`, `{"access-token": "..."}` and `{"x-api-key":
   "..."}` show their values. A name may hold `-`; the value becomes `***`.
2. **An escaped quote in a quoted assignment** (`snapshot.rs` near 905 to 908). The `NAME=value`
   scan stops at the next `"` without reading `\` as an escape, so `API_TOKEN="two \"words\"
   here"` shows `API_TOKEN=***words\" here"`. The whole quoted value becomes `***`, as the JSON
   scan above it already does.
3. **The policy pane with the dragon up** (`src/cockpit/app.rs` near 1599 to 1618, header at 1651
   to 1667). At 80x30 the mascot is on by default, the header takes 12 rows, the pane's inner
   height is 13, and the sha line, the five policy names, the blanks and the help line take 14.
   The page size is 0 at every offset and PageDown draws no run. Same at 100x30 and 80x24. At
   80x33 the room is 2 rows, a run on another sha is 2, and the row kept for `N more` leaves 1, so
   the page is still empty. Build 179's test installs no mascot. Any live run must be reachable:
   when the fixed part leaves no room, give the runs the room (drop the help line, then the
   policy names, or scroll the whole pane) and say which you chose. Test at 80x30, 100x30, 80x24
   and 80x33 with the mascot installed.
4. **Two seats ending in one read** (`src/cockpit/mascot.rs` near 346 to 365). Cards are newest
   first and only `Refused` replaces a flash already chosen. Four live seats, the oldest (head 0)
   and the newest (past the third head) both end `crashed` in one read: the pose is `meme` and
   `hurt_0` never shows. A seat past the third head does not hide a head that failed, and of two
   heads that fail together the pose shows the lowest. Say what a pose with two hurt heads would
   need, and do not add frames in this ticket.
5. **Wide characters at an odd width** (`app.rs` near 2399 to 2401). `wrapped_rows` computes
   `(len - 1) / width`; five `界` (10 columns) at inner width 5 is 3 rows in ratatui's wrap and 2
   here, so the last glyph row is clipped. Count the rows the paragraph draws.

`IMPLEMENTATION.md`: the cockpit passages on the policy pane and the detail's masking, and
nothing else.

## Done when

- Each test failed before the change; fmt, clippy over the workspace, `cargo test --lib` and
  `cargo test --test cockpit` pass.

## Outcome

Resolved 2026-10-01. Run `01a0f5e5` (codex gpt-6.1-sol, `standard`, 2.3M tokens), diff read whole
by the session and applied to master as written.

Built: a secret name may hold `-`, so `"api-key"`, `"access-token"`, `"x-api-key"` and a flag
like `--api-key=value` are masked; the quoted `NAME=value` scan reads `\` as an escape; the policy
pane makes room for one whole run and the `N more` line by dropping the help line first, then the
policy names and directory, then headings; of several seats that fail in one read the pose shows
`refused` first and then the lowest hurt head, and a seat past the third head no longer hides one.

Measured on master: fmt and clippy clean over the workspace, `cargo test --test cockpit` 86
passed, `cargo test --lib` 224 passed. The session reverted `snapshot.rs` alone and saw the two
masking tests fail, then restored it.

What the ticket assumed that is false: item 5. This ratatui draws five `界` at width 5 in two
rows, not three, and `wrapped_rows` already returns two. The run kept the count and added a test
that pins the count to what the paragraph draws, so an upgrade that changes the wrap fails it.

Left open: a pose with two hurt heads needs combined frames or heads drawn one by one. Inside
single quotes a shell does not read `\` as an escape, so `TOKEN='a\' rest` masks past the closing
quote, which errs on the safe side.

