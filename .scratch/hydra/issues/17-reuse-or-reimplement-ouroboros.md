# 17 - Reuse razzant/ouroboros, or reimplement it in Rust

Type: grilling
Status: open
Blocked by: 06

## Question

Ticket 06 surfaced a decision the map did not have. `github.com/razzant/ouroboros` exists, is MIT licensed, is Python, sits at v6.113.4, and was pushed on 2026-09-01. All three concepts we wanted are in the code: worktree sandboxing, the diff-review panel, and baseline patch integration.

D13 rules out depending on Hermes. It says nothing about this, because we did not know it was real and maintained.

Settle:

- Reuse the implementation, port its design to Rust, or take only the review prompts and the 29-item checklist and build the rest
- What reuse costs against D8. The router core is Rust, and Ouroboros in Python means a second language and a process boundary between the supervisor and the thing that rewrites it
- Whether the compiler-as-first-reviewer argument behind D8 survives if Ouroboros is Python. A Python self-edit that only fails at runtime is exactly what D8 was chosen to prevent
- Whether reuse means tracking upstream, vendoring a snapshot, or forking and diverging. A project at v6.113.4 pushed today moves fast, and pinning it is a different commitment from following it
- Which parts are worth taking regardless of the answer. The 29-item checklist and the artifact-naming rule are prompt engineering that took someone real effort

Weigh it against what the ticket 05 findings established: a guarantee written in a prompt is not a guarantee. Reusing code you have not read is the same failure mode one level up.

Whatever this decides, ticket 15 designs against it.

## What the code survey found

Read of the tree at `main`, tip `e9dfb359` dated 2026-08-30, in
[research/17-ouroboros-reuse.md](../research/17-ouroboros-reuse.md).

Two things it corrects. The 63.5% block rate is not in the code and is not computed by it, so it is
a number from the paper's Table 4 about one deployment rather than a property of the software. And
`config.py:208` ships `OUROBOROS_REVIEW_ENFORCEMENT: "advisory"`, so review does not block by
default at all. Ticket 06's union veto and responsiveness quorum both hold, with file and line.

The scale and the cadence are the other half of the decision: 608,552 lines of Python with the
review panel alone at 32,650, and 1,324 commits in eight weeks, accelerating, 61% of them authored
by Ouroboros itself.
