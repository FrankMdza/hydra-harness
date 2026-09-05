# 04 - Derive the outcome and complete what `hydra run` returns

Type: build
Status: open
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
