# 17 - Reuse razzant/ouroboros, or reimplement it in Rust

Type: grilling
Status: claimed
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

## Grilling in progress, 2026-09-02

Four decisions settled with the human. Three questions still open, listed at the bottom. This is a
progress note, not the answer: do not treat the ticket as resolved.

### Settled

**Q1. No code reuse. Take the prompt rules, rewrite the mechanisms from the reading, pin a
reference clone.** The research's own closing line is "I would not reuse the code" and the numbers
carry it: following function-local imports every seed module reaches the same 327 modules and
228,207 lines, so there is no acyclic seam anywhere; nothing in the repo promises an interface; 418
commits landed in one week and the rate is still climbing, 61% authored by the agent itself; the
review panel loads `DEVELOPMENT.md` and `ARCHITECTURE.md` whole, 867 KB of another project's
constitution, and reviewing a foreign repo still reads Ouroboros' own checklist off disk.

Added beyond the ticket's three options: keep a **pinned read-only clone at `e9dfb359`** as a
reference rather than a dependency. Ticket 15 will want to read `review_state.py` while designing,
and "we read it at this sha" is a different commitment from vendoring.

**Q2. Severity has a ceiling per checklist item, set by Hydra, and the model may report under it
but never above.** Ouroboros lets the reviewing model set severity freely, and one reviewer of
three emitting `severity: "critical"` blocks. The only thing between an advisory finding and a
blocked commit is a 27-line prompt constant asking the model to police itself, which is ticket 05's
finding one level up. Fixing severity outright throws away the reviewer's judgement that something
is milder than its category; a ceiling keeps that and removes the escalation path.

**Q3. Ouroboros is Rust, in the same binary as the router.** D8 covers the router core and is silent
on Ouroboros, so this was a real choice. The deciding fact is measured rather than argued:
`preflight_runner.py` is 1,488 lines of hermetic test running, plugin verification and `addopts`
tamper-checking, all standing in for what `cargo build && cargo test` does. Every Rust-shaped cost
is small (about 30 lines of tolerant output parsing) or already paid by other decisions: reviewers
are CLIs under D7 and ticket 11, so their 4,414-line provider layer mostly evaporates. Rung two
rewrites Hydra's source, so reviewer and reviewed sit on one compile gate and no process boundary
has to be kept correct forever.

**Q4. Rung two ships no obligation ledger, and takes none of the prompt rules that depend on one.**
The ledger is 5,009 lines across `review_state.py`, `review_custody.py` and `review_evidence.py`.
It defends against a reviewer being argued down over many rounds, which is a problem you get after
rung two has rounds, and Hydra has run it zero times. Shipping the anti-thrashing rules without the
state underneath is the worse option, because it looks like a defence in the spec and is not one.
Write down instead that rung two blocks on one review with no memory, and that the ledger is the
first thing to build when thrashing shows up in the run records.

### Two facts that should change how ticket 15 is sized

- `config.py:208` ships `OUROBOROS_REVIEW_ENFORCEMENT: "advisory"`. After 1,324 commits the
  reference implementation defaults to review **not blocking**.
- Ticket 06's 63.5% block rate comes from the paper's Table 4, not from the code. It describes one
  deployment rather than a property of the software, so it is not a design constraint.

Both point the same way: blocking review is harder to live with than it looks, and rung two should
be sized down rather than up.

### Still open

- **Q5.** What gets copied and under whose name. Q4 killed `_ANTI_THRASHING_RULE_VERDICT`,
  `_ANTI_THRASHING_RULE_ITEM_NAME` and `_CONVERGENCE_RULE_TEXT`, all of which describe behaviour
  across retries. What survives is the critical-threshold rule (27 lines, repo-agnostic, MIT notice
  if taken verbatim) and the checklist (37 KB tuned to their contracts, their frozen paths, their
  `BIBLE.md`). Proposed: take the rule close to verbatim with attribution, take the checklist's
  shape only and write Hydra's own items, vendor nothing and record the sha.
- **Q6.** What a reviewer's silence means. `empty_array_is_verified_clean` exists because `[]`
  inside refusal prose entered the quorum as an approval, and Q4 makes that worse, since with one
  review and no memory a refusal read as a pass is a self-edit that shipped unreviewed. Proposed:
  three outcomes, pass, block and **reviewer failed**, where a response that does not parse into the
  expected shape is the third and fails closed.
- **Q7.** Whether any of this binds rung one. Proposed: the panel is rung two only, and exactly two
  rules cross over because they are ticket 05's finding rather than this ticket's, that a model
  never sets what blocks and that a model's silence is not consent.
