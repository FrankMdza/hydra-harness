# 55 - An SME is a trained model, not a skill, and the corpus to train one does not exist yet

Type: grilling
Status: resolved
Blocked by: 54

## Question

**Rewritten 2026-09-10.** The first draft asked whether an SME is a skill in the store. The
human's answer was no: an SME is a model that gets trained, skills are a separate thing, and
conflating them was the session's error rather than an open question. What follows asks what is
actually undecided.

**The word, first.** Harness spec 18 lists "Training. Out of scope" as a non-goal, and the
reason it gives is that lossless export stays a constraint on the record. That is not a
prohibition on training a specialist. It is a division of labour: Hydra records and exports,
someone trains elsewhere, and D12 keeps the training code out of this repo. So a trained SME
does not reopen the non-goal. It is the thing the non-goal was written to make possible.

**The shape the design already implies.** A trained specialist does not come back as a fourth
routable provider, because D14 and the routing set are about vendor CLIs. It comes back as a
server in `config/mcp.md`, which is the map's own fog entry on a 1B to 3B controller behind an
MCP server with a local state machine that escalates to the frontier model. Its calls already
land in the record as `mcp__` tool calls and are refused under every policy but `permissive`.
That half needs no decision, only a model to point at.

**What is measured and blocking.** Three things, in the order they bite.

1. `hydra export` refuses to run. `config/export-allowlist.json` does not exist, and it is
   protected config #3, so a human writes it. The format is a JSON array of absolute repo
   paths; `src/export.rs` canonicalizes each and refuses a relative one. Confirmed on
   2026-09-10 by running `hydra export --dry-run`, which printed only the refusal.
   `config/README.md` still says "`hydra export` is not built", which stopped being true at
   build ticket 34.
2. The preference corpus is two pairs. Ticket 19 admitted exactly one source: a human
   correcting a tool call in an interactive `claude` run, discriminated by
   `toolDenialKind == "user-rejected"` **and** `entrypoint == "cli"`. Nothing else in 103
   records is a preference.
3. Judge verdicts are the richest preference signal Hydra has produced and are not a source.
   Eight passes have produced findings about real patches, in ticket 52's closed shape, sitting
   on judge run records that nothing reads.

So the questions:

1. **Do judge verdicts become preference pairs, and what is the pair?** A DPO pair needs a
   chosen and a rejected. A blocked patch gives the rejected half; the chosen half does not
   exist, because no pass has ever produced an approved patch. Is the chosen half the human's
   own fix (build ticket 39 is exactly that: a patch a human wrote after the panel blocked two
   attempts at the same feature), or does the corpus wait until rung two approves something?

2. **What does a wrong finding do to the corpus?** Pass `01a08684-67f4` produced two findings
   and both were wrong: one blocked the proposer for obeying a contract its task named, the
   other claimed let-chains fail on stable Rust when they compile on rustc 1.94. Training on
   those teaches a specialist to disobey its task and to avoid a stable language feature. A
   verdict corpus needs an answer for confidently wrong findings, and unanimity is not one,
   because unanimity carried both. Does a human sign off on every pair, and if so what is the
   throughput of that?

3. **What is the specialist for?** The fog entry says a device driver confined to a state
   machine. The corpus is agent trajectories on Rust and shell inside worktrees. Those are not
   obviously the same specialist. Naming the job decides whether 103 records is a starting
   point or nowhere near one.

4. **Where do skills sit once the SME is a model?** They stay the way you steer a vendor model
   you did not train, which is every run Hydra launches today. Ticket 56 measured that they
   only land from sonnet up. That is a fact about the harness now, independent of any training,
   and it should stop being filed under this ticket.

## Answer

Resolved 2026-09-21 with the human, after a research run on `cursor-agent` (01a0c59c-fa57).

1. Judge verdicts become preference pairs through a reader of their own, not through rung one's
   five classes: the rejected half is the blocked proposer patch, the chosen half is the human's
   fix or the panel-approved patch on the pass branch, and a pair is withheld until its chosen
   half exists. The first chosen half exists since 24cade6.
2. A wrong finding does not enter the corpus on its own: every judge-derived pair needs a human
   sign-off in a manifest naming pass, judge run and finding before it leaves through
   `hydra export`. `gated` judges are read, not filtered, because pass seven's correct finding
   came from one.
3. The specialist is the small MCP controller the map's fog describes, and the trajectory corpus
   is SFT material for frontier agents, not training data for that controller. Naming which is
   which stops the corpus being built for the wrong job.
4. Skills stay launch-time steering for vendor models and are ticket 56's; this ticket says
   nothing more about them.

`config/export-allowlist.json` is protected config #3 and the human writes it. Build ticket 65
carries the reader and the manifest.
