# 45 - `agy` cuts tool parameters at 513 characters, and the replay judged the cut

Type: build
Status: resolved
Blocked by:

## What

The corpus walk `no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent` went red
on 2026-09-12 with two `GateLeak` findings on run `01a0902c-8d66`, an `agy` 1.2.0 run under
`standard`: the replay refused two `run_command` calls as "a command the floor could not
parse", and both had run, with output in their `DONE` frames.

They ran because the live hook allowed them, correctly. The recorded `CommandLine` is 513
characters long, ends mid-expression, and carries one double quote. Census over every `agy`
record on this machine: no string parameter in a `step_update` frame is longer than 513, sixty
sit at exactly 513, and 59 of the 71 parameters over 300 characters have an odd count of double
quotes. `agy` caps the parameters it streams; the hook saw the whole command, the replay saw the
cap and could not parse it. `src/normalize/agy.rs` says at the tool-result site that "this
provider reports no truncation marker and this normalizer cuts nothing", which was measured on
outputs and is false for arguments.

A replay that judges a different command than the hook saw cannot call the difference a leak.
When a string argument sits at the cap, the call's arguments are truncated: record that, and in
`decide` turn a deny on truncated arguments into a `Limit` that says the replay could not judge
the call the hook saw, rather than a `GateLeak` or a derived decision. A synthetic record in
`tests/normalize.rs` with a 513-character `CommandLine` and a clean result must produce the limit
and no leak; one with a short unparseable command must still produce the leak, because that one
the hook did see. Amend contract 7.6 and 7.7 with the cap, and IMPLEMENTATION.md.

## Outcome

`AGY_PARAMETER_CAP = 513` and `argument_at_cap` in `src/normalize/agy.rs`. In `decide`, a call
the replay denies, with no refusal in the record, a clean result, and any string argument at the
cap gets a `Limit` naming the cap and one `permission_decision` of `allow` by `hydra_policy`
whose reason starts `hydra/capped-argument`, so contract 7.7's one decision per call holds and a
reader can tell the line was not derived from the policy the way its neighbours were. Two tests
in `tests/normalize.rs`: a 513-character `CommandLine` cut inside a quote with a clean result is
a limit and never a leak; a short unparseable command is still judged and still denied.

The corpus walks went green with it: run `01a0902c-8d66` reads as a limit. The first draft
pushed no decision for the capped call and `every_record_on_this_machine_normalizes` caught it,
40 decisions for 42 calls, which is what that walk is for.

What the ticket assumed that was false: nothing. What it did not expect: the same walk then
surfaced build ticket 46 on a `claude` record, which is a different mechanism entirely.
