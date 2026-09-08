# 51 - Cursor ran `ReadLints` past the gate, and its argument is a list

Type: prototype
Status: resolved
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

## Measured

Run 01a0800f-1fa4, Composer 2.5 under `read-only`, 2026-09-08, asked for three `ReadLints`
calls: `src/main.rs` inside the worktree, `/etc/hosts`, and no paths. All three ran and all
three returned `{"fileDiagnostics":[],"totalFiles":0,"totalDiagnostics":0}`, the model read
each as "No linter errors found", and nothing was refused. The raw argument is
`readLintsToolCall.args.paths`, a list, relative or absolute, empty allowed. The spawn
directory's `cursor-data` holds no `hydra gate:` line, which every other deny writes there
through the hook's stderr, and a `Read` outside the worktree in the same build is refused
with Hydra's reason in the result. So `preToolUse` does not fire for `ReadLints` on
2026.09.02-c22c1a3: the provider runs it without asking. The replay derives `hydra/no-path`
and reports each call as a `GateLeak`, three on this record, one on each of build tickets 29
and 31's Composer runs. In print mode there is no language server behind the tool, which is
why every call returns zero files: what it would read with one running, and whether a
diagnostic can carry file content, is not measured.

What is left is the decision. The tool reads nothing today and cannot be refused on any
policy. The capability table can say `file_read` on `paths` and keep reporting a leak it
cannot close, or say the provider runs the tool unhooked, so the replay records that as a
decision by the provider rather than a leak by the gate, and the risk statement moves to the
Cursor section of `IMPLEMENTATION.md`. Contract 7.7 has `hydra_policy`, `user` and `provider`
as deciders; the second reading spends one of them. `tests/normalize.rs` passes over that
one tool's leaks until this is answered.

## Next measurement

The human read the measurement on 2026-09-08 and asked for more before deciding: an
interactive `cursor-agent` run with a real language server behind the tool, to see what
`ReadLints` returns with one running and whether a diagnostic can carry file content. That is
a Herdr session rather than a print-mode run, and the interactive mode is built on `claude`
only, so it waits on either an interactive launch for Cursor or a hand-driven session whose
transcript is read by hand.

## Two more of the same, 2026-09-08

Build ticket 32's Composer run 01a0801d-7382 called `getMcpToolsToolCall` with `server:
hydra`, listing the delegate face's tools, and `awaitToolCall` with a `taskId`, waiting on a
background shell task it had started; ticket 34's run called the second too. Neither name is
in the table, the replay derives `hydra/unknown-tool`, and both completed with results. So the
question is not one tool but a class: Cursor's own bookkeeping tools, listing, waiting and
linting, run without the hook. The two corpus walks pass over all three by name.

## Measured again, 2026-09-08, with a file that cannot parse

Run 01a08042-7721, Composer 2.5 under `standard`, wrote `broken.rs` holding `fn main( {` into
its worktree and called `ReadLints` on it and on `src/main.rs`. Both came back
`{"fileDiagnostics":[],"totalFiles":0,"totalDiagnostics":0}`, and the model reported "No linter
errors found" for a file no compiler accepts. So in print mode there is no analyzer behind the
tool at all, not merely none for this repo, and a `ReadLints` result cannot carry file content
because it carries nothing. The interactive session with a real language server is measured
below.

## The interactive session, 2026-09-08

`cursor-agent` 2026.09.02-c22c1a3 on Composer 2.5, interactive in a Herdr pane, in a scratch
Cargo repo holding the same `fn main( {` file, outside Hydra because the interactive mode is
built on `claude` only. Asked for `ReadLints` on `broken.rs` and on `src/main.rs`, it answered
"No linter errors found" for both and reported no line of file content in either. So the CLI
has no language server behind the tool in its interactive mode either; the diagnostics come
from the editor, and the CLI is not the editor. The measurement the human asked for is done
and the decision is the one the first measurement posed.

## Answer

Decided with the human on 2026-09-08, after three measurements. The capability table gains a
third kind of row beside `tool` and `pathed`: **unhooked**, a tool the provider runs without
asking the hook. On `cursor-agent` 2026.09.02-c22c1a3 the three are `ReadLints`,
`getMcpToolsToolCall` and `awaitToolCall`. The replay records a call to one as a
`permission_decision` with `decider: provider` and a reason naming the row, never as a
`GateLeak`, and the two corpus walks stop passing over those names. The risk statement per
tool lives in the Cursor section of `IMPLEMENTATION.md`, and today it is zero for all three:
`ReadLints` has no analyzer behind it in either mode and returns nothing, the MCP listing names
tools the run's own document admitted, and the wait blocks on a task the gate already saw
start. A fourth unhooked tool arriving shows up as `unknown_tool` plus a completed result, which
is how these three did. Build ticket 35.

The alternatives, a known leak the walks skip forever and an upstream request, were rejected
for the noise they leave on every Composer record and for waiting on a vendor.
