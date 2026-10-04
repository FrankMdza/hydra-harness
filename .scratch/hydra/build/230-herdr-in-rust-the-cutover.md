# 230 - herdr in Rust: the cutover

Type: build
Status: resolved
Blocked by: 226, 227, 228, 229, and the human's answer on the mascot (map 95, question A)

## What

Map ticket 95, "H8" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`. Deletes `bin/hydra-graph`, `bin/hydra-bridge`, `bin/hydra-board`
and their tests once `hydra-herdr` has replaced them. What happens to `bin/hydra-mascot`,
`mascot/hydrapet.py` and numpy is map 95's question A, which the human has to answer first.

## Do

Close what the reviews of builds 225 to 229 found; `herdr-plugin.toml`, the README and
`smoke.sh`'s `python3` snippets name the binary; delete the Python the binary replaced. The
session runs `smoke.sh` against a live Herdr before master takes the diff.

## Done when

- Each test failed before the change, or the report says why it could not; fmt and clippy clean over
  the workspace; the tests of every crate the ticket touches pass; no dependency added without a
  reason in the report.

## Outcome

Dropped 2026-10-03 by map ticket 104 (D1, B): Herdr is frozen and the cockpit is the one interface, so the plugin is not ported to Rust. The run graph moves into the cockpit as build 271.
