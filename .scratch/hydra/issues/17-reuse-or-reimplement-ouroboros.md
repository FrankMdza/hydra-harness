# 17 - Reuse razzant/ouroboros, or reimplement it in Rust

Type: grilling
Status: resolved
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

## Grilling notes, 2026-09-02

Written mid-grilling and kept as the record of how the answer below was reached. The three questions
it lists as open were settled in a later round and are answered below.

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

## Answer

**Reuse no code. Take one prompt rule, rewrite the mechanisms from the reading, build it in Rust,
and ship rung two smaller than the reference implementation.** D13's judgement about Hermes applies
here for different reasons: worth reading, not worth depending on.

### Why not the code

The research settled this with numbers rather than taste. Following function-local imports, every
seed module reaches the same 327 modules and 228,207 lines, so there is no acyclic seam anywhere in
the package. Nothing in the repo promises an interface, and `grep` for a stability statement returns
nothing. 418 commits landed in one week, the rate is still climbing, and 61% of them are authored by
the agent itself. The review panel loads `DEVELOPMENT.md` and `ARCHITECTURE.md` whole into its
prompt, 867 KB of another project's constitution, and reviewing a foreign repo still reads
Ouroboros' own checklist off disk.

Tracking upstream is a full-time merge job. Vendoring a snapshot of a package that grew 38% new
modules in two months is a fork on day one whatever you call it. So the three options in the ticket
collapse to one real one.

What replaces them: **a pinned read-only clone at `e9dfb359`, recorded in the spec as a sha and a
clone command, vendored nowhere.** Ticket 15 will want to read `review_state.py` while designing.
Reading it at a sha is a different commitment from carrying 102 MB in a repo D2 says holds the
harness and Ouroboros.

### What gets copied

One thing, close to verbatim, with the MIT notice: the **critical-threshold rule**. 27 lines, says
nothing about Python or their repo, and it is the highest-value-per-line text in the repository.
Paraphrasing it to dodge an attribution line would be a bad trade.

The **29-item checklist is not copied.** Its shape is worth keeping and its items are not: they name
their contracts, their frozen paths, their `BIBLE.md`. A checklist that names artifacts in someone
else's repo cannot satisfy the rule it sits next to, which demands the reviewer name the exact file
or symbol in **this** repository. Hydra writes its own items. That is real work and ticket 15
inherits it.

Three prompt constants that looked worth taking are dropped, because the decision below removed
what they stand on. `_ANTI_THRASHING_RULE_VERDICT`, `_ANTI_THRASHING_RULE_ITEM_NAME` and
`_CONVERGENCE_RULE_TEXT` all describe behaviour across retries, and rung two has no retries.

### Rust, and what that is actually worth

Ouroboros is Rust, in the same binary as the router. D8 covers the router core and was silent on
Ouroboros, so this was a real choice rather than an inherited one.

The deciding fact is measured, not argued. `preflight_runner.py` is **1,488 lines** of hermetic test
running, plugin verification and `addopts` tamper-checking, all standing in for what
`cargo build && cargo test` does. That is the reference implementation's own code showing the bill
for not having a compiler, and it is stronger evidence for D8 than any argument about D8.

Every Rust-shaped cost is either small or already paid. Tolerant parsing of model output is worse in
Rust and is about 30 lines. The 4,414-line provider layer mostly evaporates because reviewers are
CLIs under D7 and ticket 11. Prompts stay data on disk, so tuning one needs no recompile. The one
thing Rust does not buy: nothing in a type system makes a reviewer honest.

The structural argument matters more than the sum. Rung two rewrites Hydra's source, so reviewer and
reviewed sit on one compile gate, and no process boundary between the supervisor and the thing
rewriting it has to be kept correct forever. Their repo carries `supervisor/` at 27,191 lines partly
for want of that.

### Rung two ships smaller than the reference

**No obligation ledger.** 5,009 lines across `review_state.py`, `review_custody.py` and
`review_evidence.py`. It defends against a reviewer being argued down over many rounds, which is a
problem you get once rung two has rounds, and Hydra has run it zero times. Shipping the
anti-thrashing rules without the state underneath is the worst of the three options, because it
looks like a defence in the spec and is not one. The spec says instead that rung two blocks on one
review with no memory, and that the ledger is the first thing to build when thrashing shows up in
the run records. Ticket 14 gets something concrete to watch for.

Two facts should keep this sized down rather than up. `config.py:208` ships
`OUROBOROS_REVIEW_ENFORCEMENT: "advisory"`, so after 1,324 commits the reference implementation
defaults to review **not blocking**. And ticket 06's 63.5% block rate comes from the paper's Table 4
rather than from the code, so it describes one deployment and is not a design constraint. Both say
blocking review is harder to live with than it looks.

### Three rules about not trusting the reviewer

These are ticket 05's finding applied rather than restated, and they are the part of this ticket
most likely to outlive it.

**Severity has a ceiling, set by Hydra per checklist item.** The model may report under it, never
above. Ouroboros lets the reviewing model set severity freely, one reviewer of three emitting
`severity: "critical"` blocks, and the only thing in between is a prompt asking the model to police
itself. Fixing severity outright throws away the reviewer's judgement that something is milder than
its category; a ceiling keeps that and removes the escalation path.

**Silence is not consent.** Three outcomes, not two: pass, block, and **reviewer failed**, where a
response that does not parse into the expected shape becomes the third rather than being forced into
one of the first two. Reviewer-failed fails closed. `empty_array_is_verified_clean` exists in their
code because `[]` sitting inside refusal prose entered the quorum as an approval, and with one review
and no memory that is a self-edit shipping unreviewed while nothing downstream notices. A sentinel
string would also work and is what they landed on, but a sentinel is something the model must
remember and a type is something it cannot get wrong.

**The apply path is unreachable, not merely gated.** The function that lands a self-edit takes an
`ApprovedPatch` that no code outside the review module can construct, so a self-edit that skipped
review does not compile. Not a runtime check, not a flag, not a policy, and no call to forget. On
top of it, the apply records the verdict it consumed into the run record, so a later reader can tell
an approved edit from an edit whose approval nobody can find. The type makes it unreachable and the
record makes it auditable, and those are different failure modes.

What the type does not cover, said plainly so it does not imply more safety than it has: it proves a
review happened, never that the review was any good, and it stops at Hydra's boundary. A child
provider writing a file inside its own worktree never touches this path. Ticket 26 exists for that.

### What binds rung one

The panel is rung two only. Worktrees, the drift proof, protected paths and the quorum stay here and
should not leak into ticket 14's design.

Exactly two rules cross over, and both do so because they are ticket 05's finding rather than this
ticket's: **a model never sets what blocks**, and **a model's silence is not consent**. Neither is
about diff review. Both are about not trusting model output at a gate, and the curator is a gate.

### What this changes elsewhere

- **D8 is confirmed rather than amended**, and gains evidence from the reference implementation's
  own source. It also gains scope: Ouroboros is Rust too, which D8 did not previously say.
- **Ticket 15** designs against a rung two with no ledger, no retries, one review, and an
  `ApprovedPatch` type. It also inherits writing Hydra's checklist items, which is the largest piece
  of unscoped work this answer creates.
- **Ticket 14** inherits two rules and a watch condition: thrashing in the run records is the signal
  that the ledger needs building.
- **Ticket 26** was opened out of this ticket's last round. The apply-path type stops at Hydra's own
  code, and a child provider's actions need a different mechanism.
- **Ticket 06's 63.5%** should stop being quoted as a property of the software.

## Amended by ticket 16, 2026-09-04, writing `docs/ouroboros-spec.md`

**"The apply records the verdict it consumed into the run record" cannot be built as written.** Rung
two's panel runs after the proposer's process has exited, and `run_end` is written when `wait()`
returns, so there is no open run file to append a verdict to. In v1 nothing lands anyway: the apply
path produces a branch rather than a commit on `main`.

The requirement behind the sentence survives intact, and it is the one this ticket actually argues
for, that unreachable and auditable are different properties. The verdict block goes in the branch's
commit message, naming the pass run id, the proposer run id and the three judge run ids. A commit
message is also the one place a later patch cannot edit, which the tree is not. "Which review
approved the commit that broke this" is answered by walking the merge commit's parent to
`ouroboros/<run-id>` and reading three run records.

**The critical-threshold rule is taken as this ticket recommended**, close to verbatim with the MIT
notice and the clone sha, in `docs/ouroboros-spec.md` section 5.1. The 29 items are not taken. Hydra
writes 23 of its own, and with the 7 carried the checklist is 30 rather than 29.

## Amended by ticket 36, 2026-09-04

The verdict's home is settled: the branch commit message, naming the pass run id, the proposer run id
and the three judge run ids. Ticket 36 adds a record for the pass itself, so the audit walk is git
plus five run records rather than git plus four.
