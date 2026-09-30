# 172 - The cockpit header counts tokens and windows

Type: build
Status: claimed
Blocked by: 171

## What

The panel of 2026-09-30 (unanimous, option A) settled "cost" for the cockpit: no dollars, because
the human pays subscriptions (`allow_billed: false`). Show tokens per minute and each provider's
share of its rate-limit window. `integrations/puppets/sheet.py` already computes window share
against `window_budget_tokens` in `integrations/puppets/vendors.json` (build 132 revises that
budget from `rate_limited` records). The cockpit's snapshot reads tokens only for ended runs
(`src/cockpit/snapshot.rs`, `State::Live => {}`).

## Do

Read `src/cockpit/snapshot.rs`, `src/usage.rs`, `integrations/puppets/sheet.py` (the window share
formula) and `integrations/puppets/vendors.json` before code.

**Check first:** do live records carry usage before `run_end`, per provider? Look at the fixtures
under `tests/fixtures/records/` and the normalizers in `src/normalize/`. Say per provider what a
live run exposes. Where a provider exposes nothing until the end, the header says so for it
rather than showing 0.

1. The snapshot sums output tokens seen so far in each live run's record, per provider, using the
   same readers `usage.rs` and `normalize` use; never a new parser of provider frames.
2. The header shows, per provider with a live run: tokens per minute over the last 5 minutes, and
   the window share by `sheet.py`'s formula against `vendors.json`'s budget, or `-` when the
   budget is null. Port the formula to Rust and name `sheet.py` as its source in a comment; do not
   call Python.
3. Nothing is written. No price table, no dollars.
4. `IMPLEMENTATION.md`, the cockpit passage.

Tests first in `tests/cockpit.rs` on synthetic records with usage frames and a scratch
`vendors.json`, each shown failing before the change.

## Done when

- The check is in the report; each test failed before the change; fmt, clippy, `cargo test --lib`
  and `cargo test --test cockpit` pass.
