# 57 - A directed proposer is judged against a spec the panel cannot see

Type: grilling
Status: open
Blocked by:

## Question

`judge_task` in `src/ouroboros/evolve.rs` is one line: it hands a judge the review checklist
and the diff. The proposer's task is not in it. That cost nothing while the proposer's only
direction was the corpus brief, because the checklist stood in for a spec and both sides were
reading the same one. Commit `5a8aa4f` changed that. A human now names the target, and the
panel grades the patch against a spec it has never seen.

Pass `01a08684-67f4` on 2026-09-09 is the demonstration. Its task named two contracts in
writing:

> `hydra reap` on an id that names no run reports that there was nothing to do and counts no
> failure; it does not error. And do not decide an id is real by measuring its length.

The proposer obeyed both. It put the resolver in `src/resolver.rs`, returned `Ok(None)` for an
unmatched short prefix so the caller decides, had `show` error and `reap` fall back, and wrote
an `is_full_uuid` that checks hyphen positions and hex digits rather than counting to 36.

`agy` blocked it, critical, on `self_consistency`: `reap` falling back where `show` errors is
"inconsistent". The inconsistency was ordered. The judge had no way to know, and nothing in its
prompt could have told it.

**The same judge's second finding is false.** It called the patch's let-chains "an unstable Rust
language feature that fails to compile on stable Rust". They compile on this machine's rustc
1.94.0, verified directly, and the admission gate had already run `cargo build` and `cargo test`
before any judge saw the diff. So pass 8's block is one finding that punishes obedience and one
that is wrong, from the judge that was right on pass 7. `claude` and `cursor-agent` returned
zero findings for the second pass running.

The obvious fix is to put the task in the judge's prompt. Do not take it, because it walks into
the one failure rung two exists to prevent.

1. **Does the judge see the task, and what stops the task from becoming the laundering path?**
   A judge told "the human asked for X" is a judge told what to accept. Anything a proposer
   should not do becomes doable by writing it into the task first. Section 4.2 spends a whole
   type, `ApprovedPatch`, making the apply path uncompilable from anywhere else, and this would
   route around it in prose. What separates "here is the spec you are grading against" from
   "here is your permission slip"?

2. **If the judge stays blind, what happens to a correct-but-unordered block?** Pass 8's patch
   is good and the panel refused it. A human reads the verdict and overrides, which means the
   panel's answer is advisory the moment a task exists. That is a different machine from the one
   spec 4.1 describes.

3. **What does this do to the training signal?** Ticket 55 asks whether judge findings train a
   specialist. Pass 8 produced two findings, one punishing obedience and one factually wrong.
   Training on that teaches a specialist to disobey its task and to fear stable language
   features. Any reader ticket 55 builds needs an answer for a finding that is confidently
   wrong, and "three judges and unanimity" is not one, because unanimity is what let both of
   these through.
