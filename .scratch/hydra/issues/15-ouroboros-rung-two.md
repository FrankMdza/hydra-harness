# 15 - Ouroboros rung two: sandbox, test gate, diff quorum

Type: grilling
Status: open
Blocked by: 06, 12, 14, 17

## Question

D11 keeps rung two fully specified even though it ships later, so that it is not a rewrite when the time comes. Rung two lets Ouroboros edit Hydra's own Rust.

Settle:

- The authority boundary: what the supervisor process may do that a proposing agent may not, and how that is enforced rather than merely intended
- The sandbox: worktree, clone or container, and what it may reach. Network, credentials, the real run records
- The test gate: what has to pass before a patch is even shown to reviewers. D8 puts the compiler first
- The diff quorum: how many judges, which models, on which auth plane, what they are asked, and what happens when they split
- What the judges are told to look for beyond correctness. Ticket 06 should supply the paper's alignment criteria
- The apply path back into the live tree, and whether a hot reload is worth the risk against just restarting
- The stop button: how a human halts a loop already running, and how a bad commit gets reverted
- What Ouroboros may never touch, whatever the quorum says. The quorum code itself is the obvious candidate

Scenario: Ouroboros proposes a patch that removes a check in the quorum, the tests pass, and two of three judges approve.

## What the research changed

**Ticket 06 replaced our assumed quorum design with the real one.** Do not design from the source document.

- Three judge models by default: `gemini-3.7-flash`, `gpt-5.6-terra`, `claude-opus-5`. Owner-configured through `OUROBOROS_REVIEWER_SLOTS`, up to 10 slots.
- The quorum counts responsiveness, not agreement. `adaptive_quorum` returns `2 if n >= 3 else max(1, n)`, counting reviewers that answered at all.
- Blocking is a **union veto**. One critical FAIL from any single judge stops the commit. "Unanimous approval" was our misreading.
- Criteria come from a 29-item checklist, with a rule that a reviewer must name a live artifact in the diff before its objection can block. That rule exists to stop judges vetoing on vibes, and it is worth copying.
- The sandbox is git worktrees, not repository clones.
- The three-way apply belongs to subagent patch integration, not to review. The parent stages without committing, so the patch still faces the gate.
- The hot reload our source document describes appears nowhere in the paper.
- 63.5% of recent self-edits get blocked.

**A new question this ticket must answer, from ticket 04.** Three judge models per review need somewhere to run. Nothing carries a Claude subscription through LiteLLM, and that is a terms limit Anthropic enforces, so judges on the raw-model plane mean per-token billing on every review. The alternative is running judges as headless calls through the agent CLIs on existing subscriptions. Decide which, and price it against a 63.5% block rate, because rejected proposals still pay for their reviews.

## What ticket 17 settled for you, 2026-09-03

Design against these rather than reopening them.

- **Reuse nothing from `razzant/ouroboros`.** A pinned read-only clone at `e9dfb359` is the
  reference. `review_state.py` is worth reading while you design; it is not worth depending on.
- **Rust, in the same binary as the router.** D8 now covers Ouroboros too.
- **No obligation ledger, no retries, one review.** The reference spends 5,009 lines on the ledger,
  and it defends against a problem you only get after many rounds. Write down that the ledger is the
  first thing to build when thrashing shows up in the run records.
- **Severity has a ceiling per checklist item, set by Hydra.** The model reports under it, never
  above.
- **Three outcomes: pass, block, reviewer failed.** A response that does not parse into the expected
  shape is the third, and it fails closed.
- **The apply path takes an `ApprovedPatch` that no code outside the review module can construct**,
  so a self-edit that skipped review does not compile. The apply also records the verdict it
  consumed into the run record, because unreachable and auditable are different properties.

**The work this hands you, which nobody has scoped:** Hydra's own checklist items. Ticket 17 takes
the 27-line critical-threshold rule close to verbatim with the MIT notice and deliberately does not
take the 29 items, because they name another repo's contracts and frozen paths, and the rule they
sit next to demands the reviewer name an artifact in *this* repository. Writing them is yours.

The mechanical pieces are small and the research file has them with line counts: the worktree
registry, the 45-line drift proof, protected-path matching, `adaptive_quorum`, the union veto. Read
[research/17-ouroboros-reuse.md](../research/17-ouroboros-reuse.md) section 6 before budgeting any
of it, and read its list of what is genuinely hard before assuming the rest is easy.
