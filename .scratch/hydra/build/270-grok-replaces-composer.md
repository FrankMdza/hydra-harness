# 270 - Grok replaces composer

Type: build
Status: claimed
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
