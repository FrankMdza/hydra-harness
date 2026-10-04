# 270 - Grok replaces composer

Type: build
Status: resolved
Blocked by:

## What

Decided by the human on 2026-10-03: composer stops being a driver ("no ha avanzado nada desde
el 2.5"). From now on, on `cursor-agent`, tasks run on Grok 4.6 (`cursor-grok-4.6-high`, which
`cursor-agent --list-models` names "Grok 4.6") and panel seats on Grok 4.7 High
(`grok-4.7-high`).

1. `crates/puppets/src/panel.rs`, `PANEL`: the security seat's model `composer-2.5` becomes
   `grok-4.7-high`.
2. `crates/puppets/src/write.rs`, the default assignment table: every `cursor-agent`,
   `composer-2.5` row becomes `cursor-agent`, `cursor-grok-4.6-high`.
3. `integrations/puppets/aliases.json` and `priors.json`: add the two Grok ids the way the
   composer entries are written, so the sheet ranks their runs; keep the composer entries, since
   old records name it.
4. If the differential tests compare the default table or the panel argv against what
   `puppets.py` recorded, exclude those cases the way `DIVERGED_SINCE_252` does, and say which.
5. `IMPLEMENTATION.md`: one sentence. `README.md` wherever it names composer as a default.

## Done when

- fmt, clippy over the workspace and `cargo test -p hydra-puppets` are clean; a unit test pins
  the panel's three seats and the default table's cursor rows to the new ids.

## Outcome

Resolved 2026-10-03 by run `01a104f8-d22b-71b1` (`codex` gpt-6.1-sol, `standard`, 0.89M tokens),
applied without changes; only its own files were taken, against the tree it was seeded with,
since that tree also held build 269's uncommitted work.

Built: the panel's security seat is `grok-4.7-high`; the four `cursor-agent` rows of the default
table are `cursor-grok-4.6-high`; `aliases.json` and `priors.json` know both ids and the display
name "Grok 4.6", with composer's prior, and keep the composer entries for old records. Two unit
tests pin the seats and the rows; both failed with the old ids. The differential test no longer
compares four `assign` cases and the `review` cases whose stub calls carry the panel's models
(`DIVERGED_SINCE_270_WRITE` and `_LAUNCH`). The session had checked both ids answer through Hydra
before the ticket was written (runs `01a104eb-dbc3` and `01a104eb-95c1`, each `ok`). The skill
was edited by the session the same evening.
