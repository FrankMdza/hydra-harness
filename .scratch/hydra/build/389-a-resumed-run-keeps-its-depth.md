# 389 - A resumed run keeps its depth and its parent's bound

Type: build
Status: resolved
Blocked by:

## Objective

Map 140's panel found it, and the session checked it. `hydra resume` builds its request with
`parent: None` (`src/resume.rs:121`), and `Lineage::of` gives a run with no parent depth 0
(`src/run/lineage.rs:42-47`). So a depth-2 child that is resumed comes back at depth 0 and can
delegate two more levels, past `MAX_DEPTH = 2` (`src/live.rs:52`). Its policy is also no longer
checked against the parent that bounded the original run. That is harmless while a human types
every resume, and it stops being harmless once map 141 lets anything resume a run on its own.

## Scope

1. A resumed run's depth is the depth of the run it continues, and the policy narrowing check runs
   against that run's parent, as if the parent had spawned it. Read both off the source run's
   line 0 (`depth`, `parent_run_id`), following `resumed_from` if the source was itself resumed.
2. No record field changes meaning. `parent_run_id` stays the edge a parent writes at spawn
   (contract 4). If the cleanest fix needs a field to change meaning, stop and say so: that is a
   map decision.
3. Read harness spec 6.1 and section 10 on `hydra resume`, and contract 4, before writing.

## Done when

- A test in `tests/delegation.rs` or `tests/resume.rs` builds by hand a depth-2 child record and
  resumes it, launching nothing a test cannot stub. The resumed run's own `delegate` is refused at
  the cap.
- A second test: a resumed child of a `readonly` parent asking for `permissive` is refused, the way
  a spawn would be.
- Verified by reverting: with the fix reverted, both tests fail. Record that under Outcome.
- `IMPLEMENTATION.md` amended in the same change. `cargo nextest run --release --workspace` and
  `cargo test --release --workspace --doc` green, `cargo clippy --workspace --all-targets -- -D
  warnings` clean.

## Outcome

Built on 2026-11-04. `Lineage::for_request` in `src/run/lineage.rs` is now what the launch
(`reserve_on_layout`) and `record_unlaunched` call. For a request with `resumed_from` and no
`--parent` it reads the source's line 0. If the envelope has a `parent_run_id`, it runs
`Lineage::of` on that parent, which gives the parent's depth plus one, the cap check, and the
parent's policy as the narrowing bound. If the source has no edge and was itself resumed, it
follows `resumed_from` back, refusing a loop. A chain that ends at a run a human started takes
that run's depth and no bound. Any other request goes to `Lineage::of` as before.

No record field changes meaning. `Lineage` gained a private `spawned` flag, false on a resume, and
`parent_run_id()` and `parent_policy()` return `None` when it is false. So the resumed run's own
record carries no `parent_run_id` (contract 4, the parent writes it at spawn) and no
`parent_policy` (contract 5.1, absent on a run with no parent), and its `depth` is the source's.
The narrowing remediation names "the parent of the run this resumes".

Two tests in `tests/resume.rs`:
`a_resumed_depth_two_child_keeps_its_depth_and_cannot_delegate` builds root, depth 1 and depth 2
records by hand, resumes the depth 2 one, writes the resumed run's line 0 through
`record_unlaunched` (depth 2 on disk, `resumed_from` set, no edge), and checks that
`Lineage::of` with the resumed run as parent is refused at the cap, and that a resume of the
resume is also depth 2.
`a_resumed_child_of_a_read_only_parent_asking_for_permissive_is_refused` resumes a `permissive`
child of a `read-only` parent and gets the narrowing remediation naming both policies and the
parent, and nothing when the resume asks for `read-only`.

Verified by reverting. Before the fix, with `for_request` as a stub over `Lineage::of`, both failed
at depth 0. With the resume branch disabled after the fix, the first failed at `left: 0, right: 2`
on the depth and the second at its `expect`, because the narrowing returned no refusal. Both pass
with the branch restored.

What the ticket assumed that is not quite so: neither test drives `hydra::resume::resume_in`.
That function goes on to launch a provider once lineage passes, and in a session Hydra launched
`HYDRA_RUN_ID` is set, so the containment refusal would fire before the narrowing one. The tests
drive `Lineage::for_request` with the request `resume_in` builds, which is the function the
launch calls. A resume whose original parent's record has been deleted is now refused, as a spawn
naming that parent would be. A resume of a depth 2 child still launches, because the cap bounds
its children rather than the resume.
