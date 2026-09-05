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
