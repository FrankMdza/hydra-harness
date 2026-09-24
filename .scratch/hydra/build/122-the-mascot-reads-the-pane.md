# 122 - The mascot reads the pane, and the pet is the mascot

Type: build
Status: resolved
Blocked by:

## What

Build items 2 and 3 of map ticket 75's panel. The mascot today (`integrations/herdr/bin/hydra-mascot
--pet --demo`) cycles poses on a timer. Make it read real runs. Two parts.

First, two tokens on the bridge. `build_tokens` in `integrations/herdr/bin/hydra-bridge` already
fetches `hydra show <id> --events` for every live pane; add `denials`, the count of
`permission_decision` events with `decision: deny`, and `tool_rate`, the number of `tool_call`
events whose envelope `ts` falls in the last 30 seconds. Test them in
`integrations/herdr/tests/test_mapping.py` beside the existing token tests, from a synthetic events
document, no Herdr and no Hydra.

Second, `hydra-mascot --watch`, a mode that never calls bare `hydra status` and never reads
`~/hydra/runs/`. Once a second it runs `herdr pane list`, keeps the panes carrying a `run_id`
token, and drives the HydraPet renderer (`pet_cells` and the vendored `mascot/hydrapet.py`, whose
`render_compact_dragon` takes three head states and three fire levels): no live pane, all heads
awake and idle; one to three live panes, one head per live run, the others asleep; a pane whose
`denials` rose since the last tick, that head flashes red for a second; a pane whose `tool_rate`
is 6 or more, that head fires; more than three live runs, all three fire and a `+n` prints under
the sprite. Add a `[[panes]]` entry `mascot` to `integrations/herdr/herdr-plugin.toml` running
`hydra-mascot --watch`. Interactive runs (`launch_mode=interactive` token) have a thin record
until they exit, so their head is awake and never fires; say so in the README.

Also make the pet the default: `hydra-mascot` with no mode flag behaves as `--pet`, and
`--save` writes the pet's frame.

## Done when

`integrations/herdr/tests/test_mascot.py` maps synthetic pane lists to head states and fire levels
without Herdr, and the two bridge tokens have tests. One real `--watch` in a Herdr pane beside a
stub run (the stub binary from `integrations/herdr/tests/smoke.sh` or a scratch record) shows a
head wake, and the README of `integrations/herdr/` says how to open the pane. Do not run cargo.

## Outcome

Renumbered from 113 on 2026-09-24 after a collision with the concurrent session. Built by
composer-2.5 on run 01a0d425-68fa (1.29M tokens): `denials` and `tool_rate` tokens on the
bridge, `hydra-mascot --watch` driving the HydraPet heads from `herdr pane list` (one head per
live run up to three, `+n` beyond, a red flash when a pane's denials rise, fire at a tool rate of
six or more, interactive panes awake and never firing), a `mascot` pane in the manifest, and 15
tests. The agent found the ticket's premise false: a normalized event from `hydra show --events`
carries no `ts`, so a rate read off one poll is always zero. The session replaced the rate with
the growth of the tool-call total across the bridge's own polls over 30 seconds, kept per run,
with two tests, and loaded the vendored pet renderer once instead of once per frame. 17 herdr
tests green on master. The agent's live check of `--watch` beside a stub pane showed a head wake;
the pane opens from the plugin list now that the manifest under the linked path carries it.
