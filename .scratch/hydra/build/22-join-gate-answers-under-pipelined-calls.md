# 22 - Join a gate answer to its call when the provider pipelines calls

Type: build
Status: claimed
Blocked by:

## Goal

`tests/normalize.rs::no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent` fails
on this machine since run 01a07efe-97f4, a `claude-sonnet-5` run under `standard` that built
ticket 18. The normalizer reports three `Ungated` findings on it, one `Read` and two `Bash`, and
none of them is real: every one of the run's 60 calls has a `hook_response` from Hydra's gate in
the record. The join is what broke.

## What the record shows

`claude::take_gate_answer` in `src/normalize/claude.rs` joins a gate answer to a call by order
inside a seq window, and its own comment says so: "Ordering is the join key, because the frame
carries no `tool_use_id`." Two things in this record break that key.

**Sonnet 5 issues the next call before the previous result arrives.** Seq 23 is a `Read`, seq 24
its `hook_started`, seq 25 a second `Read`, seq 26 that one's `hook_started`, seq 27 the response
to 24, seq 28 the result of 23, seq 29 the response to 26, seq 30 the result of 25. A window that
expects call, answer, result in that order for one call meets the next call's frames inside it.

**A call gets more than one `PreToolUse` hook pair.** 153 `hook_started` and 153 `hook_response`
for 60 calls. Which frames belong to which call is not answerable by counting.

What the frames do carry: `hook_started` and `hook_response` share a `hook_id`, and every
`hook_started` for `PreToolUse:<Tool>` arrives after the `tool_use` frame it belongs to and
before the next `tool_use` of the same tool. Ordering of `hook_started` frames matches ordering
of calls per tool even under pipelining, because Claude Code starts a call's hooks the moment it
emits the call.

## The slices

**The join.** Pair each `hook_response` to its `hook_started` by `hook_id`. Assign
`hook_started` frames to calls per tool name in seq order: the n-th `PreToolUse:<Tool>` start
group belongs to the n-th `<Tool>` call. A call's group is every start between its `tool_use`
frame and the next `tool_use` of the same tool. The gate answer for the call is the Hydra
response among that group's responses, wherever in the file it lands. `take_external_deny`
follows the same assignment. The seq window stays as a fallback for a record with no
`hook_started` frames, which is what records written before Claude Code emitted them look like.

**The test.** One synthetic record in `tests/normalize.rs` shaped exactly like seq 22 to 30
above, two `Read` calls pipelined with their hooks interleaved, asserting both calls carry an
`allow` decision and no `Ungated` finding. A second with two hook pairs on one call, asserting
one decision and no disagreement.

**The walk.** `no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent` passes again
on this machine with run 01a07efe-97f4 on disk.

## Done when

- Both new tests pass, the walk passes, and the rest of the suite is untouched.
- The doc comment on `take_gate_answer` says what the join key is now and why order alone
  stopped holding, with the run id and the date.
