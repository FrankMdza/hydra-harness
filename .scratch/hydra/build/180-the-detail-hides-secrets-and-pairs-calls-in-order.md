# 180 - The detail hides secrets and pairs calls in order

Type: build
Status: resolved
Blocked by: 179

## What

A read-only review by `codex` on 2026-09-30 of builds 177 and 178 found three defects in the
detail pane's tool calls (build 178), from in-memory reproductions:

1. **Secrets on screen.** `src/cockpit/snapshot.rs` near 640: the one-line input summary pulls
   `command` (and the like) out of the call, so `API_TOKEN=FAKE_SECRET curl example.test` shows in
   full, where the old 400-character raw cut often hid it; an input with no recognized key shows
   its whole arguments object, env values included.
2. **Results paired by id alone.** Near 586: results go into a map by `call_id`, so two calls with
   the same id (codex reuses them) both get the last result, and a `claude` result that precedes a
   later call with the same id is attached to it. A call with no result after it should show no
   decision.
3. **Cut by characters, not columns.** `src/cockpit/app.rs` near 1141: 40 wide characters (`界`)
   are 66 terminal columns, so the "one row" input wraps.

## Do

Tests first in `tests/cockpit.rs`, each shown failing before the change.

1. Mask before display, in the summary and in the whole-arguments fallback: any token that starts
   with one of `src/record/secrets.rs`'s prefixes (expose them from there, one list), and the value
   of any `NAME=value` or `"NAME": "value"` whose name contains `TOKEN`, `KEY`, `SECRET`,
   `PASSWORD` or `PASS` (case-insensitive), become `***`. A record line whose envelope carries a
   `secret_flag` shows `input hidden: secret_flag` instead of any input. The raw tail (`r`) is out
   of scope; say in the report what it shows today.
2. Pair each result with the latest earlier call of the same id that has no result yet, walking
   events in file order; a result with no such call is dropped.
3. Cut by display width (the `unicode-width` crate ratatui already depends on), never splitting a
   character, with `~` inside the width.
4. `IMPLEMENTATION.md`, the tool calls paragraph.

## Done when

- Each test failed before the change; fmt, clippy, `cargo test --lib`, `--test cockpit` pass.

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f31b-9349 under `standard`. The session
applied it and added one thing. `secrets::prefixes()` exposes the sweep's list; `masked` turns
prefix tokens and the values of `NAME=value` and `"NAME": "value"` with TOKEN, KEY, SECRET,
PASSWORD or PASS in the name into `***` (a `MONKEY` too, on purpose); a call on a line whose
envelope carries `secret_flag` shows `input hidden: secret_flag`. Results answer the latest open
call of their id in file order; an orphan result is dropped. Inputs cut by display width with
`unicode-width` (already in the lock through ratatui). Three tests, each failing first.

The run reported that the raw view behind `r` masked nothing. The session applied the same mask
there and replaces a flagged line with `[line hidden: secret_flag]`; one test, failing without it.
fmt and clippy clean, `--test cockpit` 53, `--lib` 222 on master. Untested: codex reusing call ids
on a real record, and the flag hide on providers whose call is built from a later line.

