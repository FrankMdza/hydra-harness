# 53 - Two of three judges passed a patch that hid a regression behind an edited test

Type: grilling
Status: resolved
Blocked by:

## Question

Pass `01a084d3-bdd3-7831-af07-339c253faf33` on 2026-09-09 is the first rung-two pass whose
panel reviewed a real patch rather than a red test. The proposer was given a task for the
first time, hit it, and produced 131 lines across four files. The panel blocked it, and the
block was correct.

What the proposer did: its `Layout::resolve_run_id` turned `hydra reap <unknown-id>` from a
benign "nothing to do" into an error. Rather than fix that it edited the baseline test
`reaping_a_run_that_was_never_there_does_nothing_and_says_so` to pass
`11111111-1111-1111-1111-111111111111` instead of `"nobody"`. The resolver opens with
`if prefix.len() == 36 { return Ok(prefix.to_string()) }`, so 36 characters of ones walk
straight through and the old behaviour comes back. The bug and the bypass are the same line.

What the panel did, measured:

| Judge | Provider | Tokens | Verdict | Findings |
|---|---|---|---|---|
| 01a084e2-9df5 | claude, haiku | 25,998 | pass | 0 |
| 01a084e3-dcbd | cursor-agent | 83,516 | pass | 0 |
| 01a084e4-94d8 | agy | 124,648 | block | 1, critical, correct |

Unanimity is what saved it. One judge of three found it, and that judge spent 4.8 times what
the judge that missed it spent. Two judges returned `{"verdict": "pass", "findings": []}` on a
patch carrying an edited baseline test.

1. **Is unanimity the right rule, or is the margin telling us the panel is thinner than it
   reads?** Three judges and one finder is not redundancy, it is a single point of failure with
   two spectators. Does the rule stay, does the panel grow, or does something change about who
   sits on it?
2. **Does a judge that returns zero findings on a patch another judge blocks critically keep
   its seat?** haiku spent 26K tokens to say nothing. Six passes of evidence would answer this
   properly and one does not, so the question may be premature. Ask it anyway, because the cost
   of asking later is an ApprovedPatch built on a pass nobody looked at hard.
3. **The proposer told the truth and it did not help.** Its final message says it "updated one
   test to use a full UUID instead of invalid prefix", which is an accurate description of a
   hidden regression written as a tidy-up. Two judges read that sentence and passed. Is there
   anything the brief or the judge prompt can do about a proposer that is not lying, or is this
   exactly what a panel is for and the panel worked?

## Answer

Resolved 2026-09-21 with the human, after a research run on `codex` (01a0c59c-c1c9) laid the options
out. Four judges and unanimity stay: one veto is what caught the edited test, and the panel is four
since build ticket 61. The seats stay until a human-labeled comparison set exists; one miss is not
evidence. What changes is what a judge is shown: when a patch edits an existing test, the judge
receives the baseline version of that test beside the diff and an instruction to compare the old
assertions against the new behaviour. The research also found the ticket's causal account wrong on
one point: the code sends judges the checklist and the diff, never the proposer's final message,
so "two judges read that sentence and passed" is not what happened. Build ticket 63 carries the
prompt change together with ticket 57's.
