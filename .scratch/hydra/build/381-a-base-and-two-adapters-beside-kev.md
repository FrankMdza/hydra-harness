# 381 - A dense base and two adapters beside Kev, measured on this Mac

Type: build
Status: open
Blocked by:

## Objective

Map 137, ticket 2. Question 2's answer A (one dense base on the Mac, one adapter per task) is a
plausible fit, not a measured one. Measure it before anything is trained for it.

## Scope

1. Thresholds written into this ticket before the run: peak footprint, free swap, and time to
   swap an adapter, with one agent live and Kev's server up.
2. A dense base under 1.5B (Ministral 3B is over; a 0.5B to 1.5B Apache-2.0 base) served on
   loopback with two LoRA adapters swapped in turn. Round one's adapter is one candidate.
3. Footprint by `vmmap --summary`, not `ps`, as build 294 found.

## Done when

The table is under Outcome and says whether the thresholds held. If they did not, map 137's
question 2 goes back to the human.

## Outcome
