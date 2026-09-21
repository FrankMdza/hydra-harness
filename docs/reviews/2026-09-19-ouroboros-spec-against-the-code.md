# The Ouroboros spec against the code

Written by `claude` on Haiku 4.5, run 01a0bcdb-c757 under `standard`, 1.0M tokens.
The one finding is fixed in `src/ouroboros/evolve.rs` in the commit that publishes this
report: the frame-detected abort ends the pass `completed` on a blocked branch, the way the
outcome-detected one already did.

## Method

Read `docs/rungs-spec.md` in full, then read every file under `src/ouroboros/` and tests in `tests/curate.rs` and `tests/evolve.rs`. For each numbered rule or sentence in the spec that says what rung one or rung two does, search for where the code does it or verify that it does not. Check `IMPLEMENTATION.md` for allowed departures. Rate findings as wrong (false statement a reader would act on), stale (was true, no longer), or wording (ambiguous but defensible).

## Findings

| File | Line | Finding | Rating |
|------|------|---------|--------|
| `src/ouroboros/evolve.rs` | 110-113 | Proposer rate-limit exit code mismatch | wrong |

## Detail

### Proposer rate-limit exit code mismatch

The spec's claim. Section 4.7 states at lines 163 to 168 of the comment in `src/ouroboros/evolve.rs` that "the pass ends `completed`, the way an admission refusal does, and its branch carries the reason and no `cargo` command runs."

Code path one, the early abort. Lines 110 to 113 call `bail!` when `rate_limit_abort` detects a rate-limited proposer. The `write_harness_end` call at line 111 writes with `exit_code: None` and `error: Some(abort_reason)`. 

Code path two, the outcome check. Lines 169 to 188 return `Done::Finished` when proposer outcome is "rate_limited". The `write_harness_end` call at line 176 writes with `exit_code: Some(0)` and `error: None`.

The contradiction. IMPLEMENTATION.md line 2777 states "a harness run ... derives `completed` from its own `run_end`, `failed` when that carries an error." Path 1 writes harness_end with an error, producing `failed`. Path 2 writes harness_end without error, producing `completed`. The spec requires `completed` in both cases.

Why it rates wrong. A proposer that is rate-limited via frame detection (path 1) produces a different pass outcome than one detected via outcome field (path 2). The spec section 4.7 does not distinguish between these detection methods. The spec unambiguously requires the pass to "end `completed`" in this case, but path 1 produces `failed` instead.

What `IMPLEMENTATION.md` says. IMPLEMENTATION.md documents the outcome check (path 2) at lines 2811 to 2815 but does not document path 1's bail behavior. The deviation is not named.

## Count by rating

- Wrong: 1
- Stale: 0
- Wording: 0

## The finding it is most sure of

1. Proposer rate-limit exit code mismatch, lines 110 to 113 against 169 to 188. Spec requires pass to end `completed` when proposer is rate-limited. Early abort bail path produces `failed` outcome. Outcome check path produces `completed` outcome. IMPLEMENTATION.md does not document the bail.
