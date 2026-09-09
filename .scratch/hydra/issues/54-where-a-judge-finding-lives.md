# 54 - Where a judge finding lives today, and what would have to read it

Type: research
Status: resolved
Blocked by:

## Question

The map's fog carried this since ticket 19: rung one's merged-or-dropped proposals and rung
two's panel verdicts carry preference signal in a different shape from the one admitted corpus
source, a diff plus a verdict rather than a trajectory plus a turn, and whether they become a
second corpus "waits until rung two has produced verdicts worth looking at."

Pass `01a084d3-bdd3` produced one on 2026-09-09: a critical, specific, correct block on a real
patch. The condition is met. Before anything can be decided about training, this ticket asks
the smaller question: where does a judge finding physically live today, what shape is it in,
and what part of the machine would have to change for something to learn from it?

## Answer

Measured on 2026-09-09 against the corpus and the running code.

**The training channel is built and empty.** The proposer of pass `01a084d3-bdd3` ran under
`standard` with `skills.linked: []` and `skills.resolved` naming eighteen of the human's own
skills that `claude` found for itself. `linked` is empty because `~/hydra/skills/` holds an
empty git repo: five commits, all of them ticket probes and Hydra's own pin commits, and not
one skill. Every wire is in place. Nothing has been put through it.

**A judge finding is already in a closed shape, and it is already on a record.** Ticket 52
settled that a reviewer answers one JSON shape and any deviation blocks, so every judge's
`result_text` is `{"verdict": ..., "findings": [{"item", "severity", "note"}]}`. The shape the
fog entry asked about does not need designing; it was designed by the parser. `agy`'s finding
on pass 7 reads back off its run record intact.

**It lives in two places and neither is readable by anything.** The judge's own run record
carries it as `result_text`, a JSON string inside a JSONL line. The verdict commit on
`ouroboros/blocked/<pass-id>` carries a prose rendering of the same thing. The commit message
is for a human. The record is for a reader that does not exist.

**Rung one cannot see it, by construction rather than by oversight.** Section 3.2 admits five
failure classes, all derived from the shape of a run: an abandoned pid, a `correction` event in
an interactive run, repeated tool errors, a retry loop in the `parent_run_id` tree, and an
outcome that disagrees with its own events. A judge finding is none of those. It is a second
run's opinion about a first run's output, which is a class of evidence rung one was never given.

**Nothing bars it.** The exclusion rule is that runs whose parent is the curator are out of the
corpus, so it does not learn from itself. A judge's parent is the evolve pass, not the curator,
so judge runs are already in the corpus rung one reads. It reads their outcome and their tool
errors today and ignores the one field that carries their judgement.

**What would have to change is one reader, not the format.** That makes the decision in ticket
55 cheaper than the fog entry assumed: the question is not what shape to invent, it is whether
rung one gains a sixth class or a separate pass owns verdicts.

**One caveat that costs something.** Two of the three judges on pass 7 came out `gated` and
still wrote real reviews, because the diff rides the prompt. Their findings are as good as the
`completed` one's, and the one correct finding of the pass came from a `gated` run. Any reader
that filters the corpus on outcome throws away the best evidence rung two has produced.
