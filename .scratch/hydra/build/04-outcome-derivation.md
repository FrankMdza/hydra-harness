# 04 - Derive the outcome and complete what `hydra run` returns

Type: build
Status: resolved
Blocked by: 03

## Goal

The nine outcomes, `outcome_source`, and the full result JSON. This removes the
`not_yet_derived` field the first slice invented.

## Where the design says it

- Contract 8, the enum and the rung-one column. 8.1 for why `denied` is dropped, 8.2
  for `outcome_source` and why `provider` has no producer, 8.3 for a refused spawn.
- 8.4 for a harness run, which has no producer yet, and 8.6 for who reads one.
- Contract 17 and harness spec 9 for the returned JSON.
- Harness spec 12 for the same enum stated from the launcher's side.

## Done when

- Every outcome derives from the stream and `run_end`, never from the exit status.
- `outcome_source` is `refusal`, `derived` or `unknown`, and `unknown` has a real
  producer through the adapter's `unclassified`.
- `hydra run` returns `run_id`, `outcome`, `outcome_source`, `exit_code`, `error`, the
  final assistant text, the worktree branch, a diff stat, `usage` with its
  `usage_source`, and the run file path. `not_yet_derived` is deleted.
- The diff stat is computed from git at return time and says what it covers, because
  the worktree is seeded with the parent's uncommitted work and a naive `git diff HEAD`
  reports the seed as if the child had written it.

## Watch for

**A rate-limited `claude` run reports `subtype: "success"` with `is_error: true` and
`api_error_status: 429`.** The contract calls this the single nastiest shape in it,
because it is the one that looks correct. A recorder keying off `subtype` files an
exhausted run as a clean one.

**All three providers exit 0 and report success on a run where the gate refused
everything.** The first slice measured that on `claude` before the gate existed:
`terminal_reason: completed`, `is_error: false`, exit 0, with the refusal sitting in
`permission_denials`.

**`outcome` is never stored.** An outcome written into the file would be wrong forever;
contract 6 and 11 both say so, and `run_end.refusal` is the one narrow exception.

## What ticket 03 left you, added 2026-09-05

**The normalizer is the instrument, and it is `hydra::normalize::read_file`.** It takes a
path and a `PolicySet` and returns a `Run`, with `events`, `findings`, `policy` and
`provider`. `Run::usage()`, `Run::messages()`, `Run::tool_calls()`, `Run::run_end()` and
`Run::permission_decisions()` are the accessors an outcome derivation wants. Nothing it
returns derives `Serialize`, on purpose: contract 11's rule is expressed in the type
system, so putting an outcome in a file takes a deliberate struct of your own rather than
a `to_string` on a `Run`.

**The terminator frame is on the `Run` already, as `result`.** It is the raw frame
verbatim rather than a normalized field, which is deliberate: `subtype`, `is_error`,
`api_error_status`, `terminal_reason` and `stop_reason` are all things you have to read
and none of them is one of the ten types. `permission_denials` and `subagent_stats` are on
the same frame, per 8.5. `None` means the run never reached a terminator, which on a
killed run is the abandoned class rather than a failure.

**`usage` is already the shape contract 17 asks for.** `Usage::source` is `provider` or
`unavailable`, never a bare null, and `Counts::raw_input()` and `Counts::total_tokens()`
are 9.1's reconstruction. `estimated` has no producer and `estimator` is always `None`.

**Three findings bear on an outcome and none of them is one.** A `Finding::GateLeak`, a
`Finding::SubagentLeak` and a `Finding::Disagreement` say the run did something its
policy forbade. Contract 8's enum has no outcome for that, and it should not get one
here: a finding is the derivation saying two sources disagree, an outcome is what
happened to the run. If `hydra run` should surface them, that is a decision for the map
rather than a field to invent.

**`unclassified` has a real producer now, and it is not only `agy`.** The `standard`
run's successful `Write` came back with no `is_error` field at all, so its
`tool_result.status` is `unclassified`. That is the shape `outcome_source: unknown` is
supposed to meet, and it arrives on `claude` rather than on the provider the contract
warns about.

**A gated deny still exits 0 with `subtype: success` and `terminal_reason: completed`**,
and the record now proves it per call: the read-only runs each carry one `Write` whose
derived decision is `deny` under `hydra/capability`, beside a `result` frame that reports
success. The normalizer is where an outcome derivation gets that, rather than the exit
status.

## Outcome

Built. `src/outcome.rs` derives contract 8's nine outcomes and 8.2's `outcome_source`
from a normalized run; `hydra::run::result` projects that plus the diff stat into
contract 17's JSON, and `not_yet_derived` is gone. 24 tests in `tests/outcome.rs`, three
of them corpus walks. 148 tests in the suite, all passing.

**The result is read back off the file `hydra run` just wrote.** Every value was still in
scope when the child exited, so assembling it in memory would have been faster and would
also have let the JSON carry a fact the record does not have, which contract 6 says is
the mistake ticket 11 made. That costs a python process per recorded tool call: 0.6ms for
a record with no calls, 94ms for one with two, so about 47ms a call and a 200-call run
spends nine seconds after the child exits. Ticket 05 moves it off the blocking path.

**`gated` is "the gate refused everything" as a predicate**, which is contract 8's own
phrase and the same words in harness spec 12 and tickets 27 and 14. At least one tool
call, no call through, one refusal in Hydra's own voice. Three exclusions, all pinned by
tests: a single refused call beside work that landed is not a `gated` run (8.1's reason
for dropping `denied`), a `GateLeak` is not one because the call went through, and a
`decider: external` refusal is not one because 7.7 spends a section separating the user's
hooks from Hydra's gate.

**The rule's weak spot, stated rather than closed.** One allowed `Read` beside four denied
`Write`s comes out `completed`. That run genuinely failed on too narrow a policy, and
nothing in the record says what it was trying to do. Fixing it means a threshold like
"mostly refused", which is a number no ticket settled, so the predicate stays literal and
`IMPLEMENTATION.md` says so.

**One ticket assumption turned out incomplete.** The ticket pointed at the `standard`
run's unclassified `Write` as `unknown`'s producer, and that is real. Record
`01a07308-a250-7792-9e8b-0d99697c99d9` derives `completed` with `outcome_source: unknown`,
the only one on this machine. What the ticket did not predict is that a pre-gate record
with a real `Write` denial in it derives `completed`, because contract 7.7 gives a run
with no policy no `permission_decision` at all and `external` "presumes a policy exists
to have allowed". `01a07218-8817-7581-9ddb-47565e5d5ba7` is that record. It is the
design's own consequence, it applies to the three pre-gate records and to nothing written
since, and it is written down rather than worked around.

**Three of the nine outcomes still have no producer, and that is now visible in the
enum.** `timed_out` needs ticket 05's watcher; `skill_collision` and `mcp_unavailable` are
launch checks belonging to 07 and 08 and arrive through `run_end.refusal`, which nothing
writes. `record::Refusal` mints the four codes so the write side and the derivation share
one vocabulary, and an unrecognized code still comes out `gated` rather than falling
through to `completed`. `RunEnd::refused` has no caller yet, so the test drives it: it
serializes the writer's own closing line for each variant and derives the outcome from
that, rather than matching the derivation against a second copy of the strings.

**The diff stat needed a tree snapshot, and the trap the ticket named is real.** The base
is `git write-tree` over the freshly seeded checkout, staged through a scratch
`GIT_INDEX_FILE` so the child's own index is untouched. Measured on this repo: a worktree
seeded from nine dirty files with one file written into it afterwards reported one file
and one insertion, where `git diff HEAD` in the same checkout reported nine files.
Committing the seed was the alternative and it hides the work in progress from a child
asked to continue it, which is what 6.3 seeds the worktree for.

**The first `hydra run` after this landed hit the session limit, which measured contract
8's nastiest shape on a live run.** Record `01a073ef-701c-7522-b5bb-56d5ecba0f57`:
`subtype: "success"`, `is_error: true`, `api_error_status: 429`, `terminal_reason:
"api_error"`, **exit code 1**. Derived `rate_limited`, `derived`. The exit code is the
part worth keeping: a reader keying off it would have filed an exhausted quota as an
ordinary failure and rung one would have learned a skill lesson from it.

**One departure from the design found, and it belongs to whoever builds rung two's early
abort.** Contract 8.5 describes the `rate_limit_event` frame as carrying `rateLimitType`,
a `status`, a `resetsAt` epoch and a per-window `utilization`. Measured, they are nested
under `rate_limit_info`, `utilization` sits under `unifiedWindows.<window>`, and
`overageStatus`, `overageDisabledReason` and `isUsingOverage` are beside them unnamed by
8.5. The frame has no reader today, so nothing is broken; `IMPLEMENTATION.md` carries the
measured shape.

**`run_end.error` is read only when there is no terminator, which took two passes to get
right.** The first version read it always and made a lossy UTF-8 byte a `failed` run, which
is contract 8.4's mistake pointed the other way. The second stopped reading it at all, and
the code review caught the consequence: a broken tee then came out `crashed`, asserting a
death nothing witnessed on a child that may have finished perfectly. The split is the
answer. With a terminator the field is a fidelity note; without one it is the only thing
that says which side stopped.

**`provider` and `provider_session_id` are gone from the result.** Contract 17 gives the
field list and then says "Nothing else". Both are in the record and `run_file` is on the
list.

**One thing this ticket did not measure end to end.** A `completed` run's JSON off the
real binary. The measurement run hit the session limit, so the live path was exercised on a
`rate_limited` run instead, filling every field: the derivation, the usage projection,
`result_text`, `branch`, `run_file` and a `diff_stat` with a real `base_tree`. The nine older records
derive `completed` through the same `derive`, and `hydra::run::result` is tested on a
synthetic completed record, so what is untested is only the two in combination.

**A gap in the returned surface, left for ticket 11.** A spawn that never started writes a
record and returns the error, so `hydra run` exits non-zero with nothing on stdout. The
record derives `failed`. Printing the JSON instead would mean exiting 0 on a run that
never launched, which breaks a script reading the status, so it belongs with the MCP
delegate face, where a caller has no exit code to read.

## What the code review changed, added 2026-09-05

Five findings, `/code-review high`. All five are addressed and I checked each fix against
the old code rather than trusting the report, which changed what two of them turned out to
be.

**Two were real bugs and their tests fail against the old logic.**

`final_assistant_text` returned the last text *block* rather than the final turn. Contract
7.3 gives one `message` event per content block, so an assistant frame that interleaves
thinking arrives as `[text, thinking, text]` and `.last()` returned the second half of what
the model said with the first dropped. The blocks of the last frame are now joined in block
order.

A broken tee derived `crashed`. Removing the `error` read earlier in this ticket left no
terminator plus a real exit code as the only signal, and contract 8 defines `crashed` as
"the process died", which a tee failure is not evidence of. `error` is read again, but only
on the no-terminator side, where it is the one thing that says which side stopped.

**One was a real hazard and needed a better test than the reviewer's shape suggested.** A
torn line 0 normalizes to an empty `run_id`, and `Layout::spawn("")` is `~/hydra/spawn/`
itself, so the closing diff stat staged into one file shared by every such record. My first
test passed against the unguarded code, because without a real git repo git refuses before
the index is written and the hazard hides. With a `git init` in the test it fails, so the
guard is pinned rather than assumed.

**One was correctness-neutral and I am not calling it a fix.** The reviewer flagged
`refused_everything` matching gate leaks by `call_id`, which collides when two `tool_use`
blocks carry no id. Working it through: every leak is a call the policy denies that nothing
refused, so it is one of the denies and exactly one finding is pushed per call, which means
one leak already drops the matched count under `calls` and both forms answer "not gated".
The counting version is kept because it needs no identity and takes the collision out of
the reasoning. The test that came with it pins the outcome across that shape and passes
either way, and says so.

**One is a real gap whose trigger I could not test.** `read_file` failing after the record
is whole propagated through `run()`, so a child that exited 0 produced a non-zero command
and no JSON, losing the run id and the path to the file holding the answer. It now degrades
to contract 17's field list with the derivations null and `error` naming the read failure.
Reaching that path needs `read_file` to fail on a file `hydra run` just wrote, and
`hydra run` launches a provider, so only the fallback's field shape is tested.

The review also cleared, worth knowing for the next pass: the hook join's window bound
against the real frame ordering, `splice`'s positional alignment on every path including
its three early returns, `Stat::parse` against `git diff-tree` being plumbing so no rename
lines appear, `api_error_status` being a JSON number on the real 429 record, and that
`tests/normalize.rs` lost only helpers and no coverage.

## What the test suite gained, added 2026-09-05

**`tests/common/mod.rs` is new**, holding the record builder and the sandbox layout that
`normalize.rs` and `outcome.rs` both need. `Record::closed_with` and `Record::unclosed`
are the additions: the outcome derivation reads `exit_code`, `error` and `refusal`, and
one test needs a file with no `run_end` at all.
