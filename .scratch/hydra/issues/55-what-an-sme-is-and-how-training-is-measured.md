# 55 - What an SME is, where its memory lives, and how you tell training from noise

Type: grilling
Status: open
Blocked by: 54, 57

## Question

**First, the word.** Harness spec 18 lists "Training. Out of scope" as a non-goal, and the
build plan repeats it. That entry is about model weights, and the reason it gives says so:
lossless export stays a constraint on the record so someone can train elsewhere. What this
ticket calls training is a skill in the store getting better because a judge found something,
which is rung one's stated job, "the curator that evolves skills". Nothing here reopens the
non-goal. Anyone who reads the two together should come away with weights out and skills in.


On 2026-09-09 the human asked for the first test of an SME: specialists launched one at a time,
one of them making decisions while the judges' recommendations are collected as its training.
Ticket 54 measured the ground. The skill store is an empty git repo, the proposer already gets
`skills.linked` from it, judge findings already exist in a closed JSON shape on a record, and
nothing reads them. So the wires exist and four decisions do not.

**One iteration has already happened by hand, which is what makes this specifiable.** Pass
`01a084d3-bdd3` was blocked by `agy` for editing a baseline test to hide a regression. A human
then added one clause to the proposer's brief forbidding exactly that, named the two contracts
the patch broke in the task, and launched pass `01a08684-67f4` on the same target. That is
training: a judge's finding changed what the next specialist was told. It was applied by a
person editing Rust, it lives in an uncommitted diff, and it is not reproducible. Every question
below is about turning that into something the machine does.

1. **Is an SME a skill, or a new noun?** A skill in the store plus its `hydra.toml` sidecar
   already gives a name, a version, a per-provider opt-out, a snapshot pinned by sha on every
   record, and a proposal branch a human merges. A new noun costs a second store, a second
   snapshot rule, and a new gate scope. The cheap answer is that an SME is a skill and the word
   is a role rather than a type. What does a new noun buy that this does not?

   Ticket 56 measured the catch on 2026-09-09. A skill reaches the model's menu on every model
   and gets opened only by the stronger one: Sonnet's first tool call was
   `Skill { skill: "hydra-self-edit" }`, haiku never reached for it on the same task and named
   the repo's `CLAUDE.md` instead. So "an SME is a skill" carries a model requirement, and the
   proposer has run on haiku for all eight passes. Does the answer name a floor for the model an
   SME runs on, and what happens to a specialist whose training sits in a store its model
   ignores?

2. **Rung two is memoryless and training is memory.** Section 4.1 says a pass is "No loop, no
   retries, no memory," and that line is load-bearing: it is why there is no supervisor process.
   A specialist that gets better across passes has memory somewhere. The reading that keeps 4.1
   literally true is that the pass stays memoryless and the store carries the memory, since the
   store is outside the pass and already pinned per run. Does that reading hold, or does it make
   4.1 a sentence that is true and no longer means anything?

3. **Who applies the training?** Ticket 54 found the change is one reader, not a format. Either
   rung one gains a sixth failure class whose evidence is a judge finding, or a separate pass
   owns verdicts. Rung one is the natural home because it already writes skills and only skills,
   and the argument against is that its five classes are all derived from one run's own shape
   while a verdict is a second run's opinion. That is a real difference in kind, not a detail.

4. **How do you tell training from noise?** Section 3.8 refuses a general before-and-after and
   measures only the signal a change was made for, on purpose, because Hydra has no notion of
   task similarity and real work does not repeat. Training a specialist needs the same task
   twice, which is the thing 3.8 declined to build. Passes 7 and 8 ran the same target and are
   the beginning of a fixed task set by accident. Does a benchmark of repeatable tasks get built,
   and if not, what does "this SME improved" mean that is not a story told after the fact?

   Sharpened by ticket 57 on the same day. Pass 8 produced two findings: one blocking the
   proposer for obeying a contract its task named, and one claiming let-chains do not compile
   on stable when they compile on this machine's rustc 1.94. A reader that trains on judge
   findings would have taught a specialist to disobey its task and to avoid a stable language
   feature. Whatever answers question 3 needs an answer for a finding that is confidently wrong,
   and unanimity is not it, because unanimity is what carried both.

5. **Nothing lands unseen, at one merge per iteration.** Section 3.7 makes every skill change a
   proposal on a branch a human merges, and calls that a graduation condition rather than a
   permanent rule, with the count "picked from the corpus" by someone later. A training loop
   run one specialist at a time is precisely the thing that wants the flip, and it is also the
   thing that would do the most damage if the flip came early. Is a human merge per iteration
   the intended pace of training, or does this ticket pick the count?
