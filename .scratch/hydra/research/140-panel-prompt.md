# Decision panel for map ticket 140

You are one seat on a decision panel. Read only: do not edit, create or delete a file in the
repository, and do not commit. Never open, print or copy the contents of a credential file. Scratch
experiments under `/tmp` are fine. No AWS account, no spend. Your seat name is in your task line.
The human is present and decides on the panel's recommendation, so commit to one option per
question and say why.

## Read

1. `.scratch/hydra/issues/140-two-videos-weight-editing-and-a-sliced-agent-runtime.md`.
2. `.scratch/hydra/research/140-two-videos.md`, the research, read 2026-11-04.
3. `.scratch/hydra/research/138-synthesis.md` and `139-synthesis.md`, the plan this joins, and the
   Answer of `.scratch/hydra/issues/137-the-lab-s-next-arc-cloud-experts-and-post-quantum.md`.
4. `docs/harness-spec.md` section 10 (detached children, `hydra resume`, "Nothing appends to a run
   file except the process that owns it"), `src/watch.rs`, `src/resume.rs` and `src/face.rs`.

## Questions

1. **The short and map 138.** The research confirms the source (MIT CSAIL's weight-space
   programming) and corrects research 138: on networks under 10M parameters the editor read, fixed
   and even wrote working weights with no training, while on Gemma 3 270M it could not improve
   math. Does this change any of map 138's votes (Burn plus a PyTorch reference, frozen layers then
   LoRA, the gate classifier first, frontier weight diffs back to the human)? Is there a cheap,
   licence-safe experiment worth adding, for example an open-weight editor, or a human-written
   edit, on the lab's own nano model? Say where in the 384 to 386 order it goes, or say no.
2. **Infinity: A, B, C or none.** (A) Borrow ideas only: reserved-suffix synthetic tool calls for
   events, wake on message instead of a parent holding its process, a stable id per input. (B) A
   sliced runtime for the lab's own nano agents on the Mac, each slice a linked run. (C) Infinity
   itself in AWS. Pick, and say what it costs in RAM, in dollars and in Hydra's contract.
3. **If A, which idea first**, against Hydra's code as it is: what changes, which spec sentence it
   touches, and whether it needs a map decision before a build. In particular, does "wake on
   message" break the rule that liveness is a live pid?
4. **If B**, when: before or after map 138 produces a nano model that calls tools, and whether to
   use Infinity's crates as a library (`ModelProvider`, the stores) or write the loop. Check one
   claim about `infinity-agent-core` against the repository if you can reach it.
5. **Security.** An injected event is a new channel, and Infinity lists event authenticity as
   open. What map 139's work (credential deny, signed manifest) has to add, if anything.
6. **The tickets**, at most three, smallest first, each with what it measures and how a test
   proves it.

Check at least two claims of the research against the code or its sources and say which.

At most 900 words. End with `## Pending from me`, `## Changed` and `## Found`.
