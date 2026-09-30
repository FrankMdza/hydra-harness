# 172 - The cockpit header counts tokens and windows

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-30. Built by `claude` opus on run 01a0f2ef-719a under `standard`. The session
applied it as written.

The check: only `agy` counts mid-run (each finished `step_update` carries `usage`). `claude`,
`cursor-agent` and `codex` expose nothing until their terminator, and the header says
"<provider> no count until a run ends" rather than 0. Each normalizer's line loop became
`walk(text)`, and `usage_so_far` runs it without `finish`, which would replay the gate every tick.
`src/cockpit/window.rs` ports `sheet.py`'s window share and `models.py`'s `canonical`; `claude`, a
null budget or one of 0 give `-`. Tokens per minute look back 5 minutes across reads. The header
takes one row above the panels only while a run is live. The window share counts live runs only,
so it is a floor; counting ended runs in the window too is left to the human.

Five tests, each failing against stubs first. On master: fmt and clippy clean, `--lib` 221,
`cockpit` 33, `normalize` 87, `outcome` 44, `show` 3, `usage` 3, `harvest` 12, `record_contract`
20, `export` 23, `curate` 22, `fixture_corpus` 3. `hydra doctor --walk` over the 603 records on
this machine prints the same output before and after this change, byte for byte (exit 1 on 11
older findings: 6 gate findings, 5 "owes one decision per call").

