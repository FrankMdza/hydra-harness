# 51 - Cursor ran `ReadLints` past the gate, and its argument is a list

Type: prototype
Status: open
Blocked by:

## Question

On 2026-09-08, build ticket 29's run 01a07ffe-372a on `cursor-agent` (Composer 2.5, `standard`)
called `ReadLints`. The raw frame is `readLintsToolCall` with `args.paths`, a list of three
absolute paths, all inside the worktree. The capability table says the tool's path field is
`file_path` and the normalizer reads `path`, so neither saw a path: the read-time replay derived
`hydra/no-path`, a deny, and the stream shows the tool completing 21 milliseconds after it
started with a real result, `totalFiles: 0`. `tests/normalize.rs`'s corpus walk now reports that
record as a `GateLeak`, which is the finding doing its job.

Two things are unknown and one of them is a decision.

- **Did the hook fire, and did Cursor obey it?** A `Read` outside the worktree in the same run
  was refused with Hydra's reason in the error result, so `preToolUse` was live. Either
  `ReadLints` does not go through `preToolUse`, or it does and Cursor ignores a deny on it, or it
  does and the gate at runtime answered something other than what the replay derives. Nothing in
  the stream says which. The spawn directory is reaped, so the runtime answer is gone.
- **A list-valued path argument has no shape in the table.** `pathed()` takes one field holding
  one path. `paths` is a list, and the gate has to scope every element. Whether the same shape
  exists on other Cursor tools, or on `agy`, is unasked.

The decision: what a leaked tool means for the policy set. If Cursor runs `ReadLints` past a
deny, the tool is `provider_native` on that provider in the sense of 7.5, reachable under
`permissive` only, and the table should say so rather than pretend to scope it. If Cursor obeys
and the field was merely wrong, it is a build ticket like the `grep_search` fix of the same day.

## What to measure

A `cursor-agent` run under `read-only` in a worktree, asked to call `ReadLints` on one file
inside the worktree and one under `/etc`, with the gate's own log kept before reap. Three
outcomes are possible per call: the hook fired and the result is an error carrying Hydra's
reason; the hook fired, said deny, and the result is a success anyway; the hook did not fire.
Harness spec 6.6 says what a hook that fires looks like on this provider.

## Until it closes

`tests/normalize.rs::no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent` skips
record 01a07ffe-372a by id, naming this ticket, so a build run does not spend its last third
bisecting a leak it did not cause. The skip comes out when the ticket does.
