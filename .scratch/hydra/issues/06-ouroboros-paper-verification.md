# 06 - Verify the Ouroboros paper at source

Type: research
Status: resolved

## Question

The originating doc builds its whole self-improvement design on arXiv 2608.08311, and cites a 161-day deployment, 86.74% on Terminal-Bench 2.1 and 90.69% on OSWorld-Verified. It also calls the model "Claude 3 Opus (Opus 5)" and elsewhere refers to Claude 3.5 Sonnet, stale enough to cast doubt on the summary.

Read the paper itself and report:

- The actual architecture of the two evolution loops, in the paper's own terms
- What the diff quorum is: how many judges, chosen how, judging against what criteria, and what happens on disagreement
- How the sandbox and the three-way apply work, and what the parent process is not allowed to do
- Whether the benchmark numbers and the deployment are as reported, and against which models
- The failure modes and limitations the authors admit to

Also check `github.com/razzant/ouroboros`, which the doc names as the reference implementation, and say whether it exists and what state it is in.

If the paper does not exist or does not say what the doc claims, that is the finding, and tickets 14 and 15 get built from scratch instead of from it.

## Answer

The paper is real and the numbers hold. The mechanism in our source document is wrong in ways that would have misled tickets 14 and 15.

arXiv 2608.08311 is "Ouroboros: A Self-Developing Frontier Coding Agent with Reviewed Core Evolution", Razzhigaev et al., cs.SE, v3 revised 31 August 2026. Every figure the source document quotes checks out: 86.74% on Terminal-Bench 2.1 audited (86.97% raw), 90.69% on OSWorld-Verified, both on Opus 5. The 161-day Hope deployment across seven named surfaces is real. Calling the model "Claude 3 Opus" was our document's error, not the paper's.

The diff quorum is not what we assumed. The paper says "diff-review panel" without a judge count; the implementation gives one. Three models by default (`gemini-3.7-flash`, `gpt-5.6-terra`, `claude-opus-5`), owner-configured through `OUROBOROS_REVIEWER_SLOTS`, up to 10. The quorum measures responsiveness, not agreement: `adaptive_quorum` returns `2 if n >= 3 else max(1, n)`, counting reviewers that answered at all. Blocking is a union veto, so one critical FAIL from any single judge stops the commit. "Unanimous approval" is the wrong shape entirely. Criteria come from a 29-item checklist, with a rule that a reviewer must name a live artifact in the diff before its objection can block.

Two more corrections. The three-way apply belongs to subagent patch integration, not to review; the parent stages without committing, so the patch still faces the gate. The sandbox is git worktrees, not repository clones. The Textual hot reload our document describes appears nowhere in the paper.

`github.com/razzant/ouroboros` exists, MIT, Python, v6.113.4, pushed 2026-09-01. All three concepts we wanted are in the code. Hermes appears in the paper as a benchmark baseline at 77.53%.

The number our document omitted is the most useful one: a 63.5% recent review block rate. Two of every three self-edits get rejected. Any design that assumes self-edits usually land is wrong.

Detail: [research/06-ouroboros-paper-verification.md](../research/06-ouroboros-paper-verification.md)

## Amended by ticket 17, 2026-09-03

**The 63.5% block rate is not a property of the software.** Ticket 17's code survey found it is
nowhere in the source and is not computed by it, so it comes from the paper's Table 4 and describes
one deployment. Stop quoting it as a design constraint.

Related, and pointing the same way: `config.py:208` ships
`OUROBOROS_REVIEW_ENFORCEMENT: "advisory"`. After 1,324 commits the reference implementation
defaults to review not blocking at all. The union veto and the responsiveness quorum this ticket
recorded both hold, with file and line. What does not hold is the framing of the gate as something
that routinely blocks.
