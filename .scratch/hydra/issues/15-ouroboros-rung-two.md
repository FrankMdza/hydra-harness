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
