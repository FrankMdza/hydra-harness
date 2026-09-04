# 15 - Ouroboros rung two: sandbox, test gate, diff quorum

Type: grilling
Status: resolved
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

**How much checklist work, with a number.** Ticket 17's survey counted roughly 7 of the 29 items as
project-independent: `secrets_check`, `code_quality`, `security_issues`, `tests_affected`,
`cross_platform`, `self_consistency`, `changelog_accuracy`. The other 22 name Ouroboros files,
Ouroboros invariants or Ouroboros subsystems. So the shape and the severity machinery carry over and
**22 items have to be written from scratch**.

One trap to fix rather than copy, found in their gate. The reviewer's `item` field is free text and
is never validated against the checklist, so a model returning an invented item name with
`severity: "critical"` blocks the commit even where the checklist says that item cannot block. Under
ticket 17's severity ceiling an unrecognised item has no ceiling to sit under, so decide what an
unknown item name does. Rejecting the response as a reviewer failure is the option consistent with
this ticket's other answers.

## Answer

**Rung two proposes, a panel of three provider CLIs decides whether the proposal may be offered, and
a human merges.** Nothing lands unattended in v1. The scenario the ticket opens with is answered
before the panel ever sees it: a patch touching the quorum is refused at admission, so two of three
judges approving is not a state this design can reach.

The panel is not the last line of defence, which is why it can be cheap. Two facts pushed that
decision. Any patch that weakens the review is reviewed by the thing it weakens, and no arrangement
of judges fixes that. And the reference implementation ships `OUROBOROS_REVIEW_ENFORCEMENT:
"advisory"` after 1,324 commits, which says a blocking gate is what people turn off once it is the
only thing in their way. A human merge is a gate nobody flips with an environment variable.

The graduation condition, written down the way ticket 14 wrote its own so it is a decision someone
makes rather than a default nobody revisits: auto-landing becomes arguable once enough proposals
have been merged unchanged, judged from the run records. The count is not picked now. Hydra has run
rung two zero times.

### The shape of one run

`hydra evolve` is a single pass. One proposer run, the admission gate, one panel, one branch, exit.
No loop, no retries, no memory, per ticket 17. The word "loop" in this ticket's stop-button question
does not survive the design.

### The authority boundary

**No supervisor process.** The boundary is the parent and child boundary Hydra already has, and
ticket 17's argument holds: a process boundary between the supervisor and the thing rewriting it is
something you have to keep correct forever, and their `supervisor/` is 27,191 lines partly for want
of a compiler.

The proposer is a `hydra run` child under ticket 11. Its own worktree, no credential of any kind, an
environment built from ticket 21's allowlist, a hook config Hydra writes at spawn. The supervisor is
the Hydra process the human invoked. It alone reads verdicts, constructs `ApprovedPatch` and runs
git against the real repo.

Three mechanisms enforce it and all three already exist. The child has no path to the real tree. The
hook gate refuses writes outside its worktree. `ApprovedPatch` makes the apply path uncompilable
from anywhere else.

The child runs under the same binary version it is editing. That is harmless: it edits files, and
the running process is already loaded.

### The sandbox, and the hole in it

**A worktree, the hook gate and the environment allowlist. Not a container.**

A container has to reproduce the provider's own login or be handed credentials, and D6 forbids the
second. Ticket 21 measured what the first costs: a wrong `HOME` silently logs out three of four
providers. So the proposer runs with the real `HOME`, and the gate is what keeps it out of
`~/.claude`, `~/.cursor`, `~/.agents` and `~/hydra/runs`. Ticket 26 confirmed read blocking on all
three, `beforeReadFile` on Cursor and `PreToolUse` on the other two.

**Network cannot be cut**, because the CLI needs it to reach its own model. Write that in the spec
as a hole rather than dressing it up as a control.

Run records reach the proposer as a read-only brief the supervisor assembles, never as a directory
it can open. A loop that learns from evidence must not be able to edit the evidence. Ticket 13
already forbids it calling `prune` or `reap`; this closes the same door at the file level.

### The admission gate

`cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, `cargo test`, in the worktree, all
four before a single judge is dispatched. Deterministic, and they run whether or not any model
agrees with them.

**Admission failure is not a review outcome.** The run ends `failed` carrying the compiler output
and spends no review budget. This is `preflight_runner.py`'s job, which costs 1,488 lines of Python
and nothing here, and that remains the strongest evidence on this map for D8.

Two refinements the reference implementation earned the hard way.

**Protected test files come from the baseline sha, not from the patched worktree.** A self-edit that
adds its own passing test proves nothing. Ouroboros freezes `tests/test_contracts.py` for exactly
this reason.

**Iterating against the compiler inside the proposer's own run is expected and free.** It is not a
retry. Ticket 17's "no retries, one review" governs review rounds, and without this sentence someone
reads it as forbidding the proposer from fixing its own build.

### The panel

**Three judges, one per routable provider, each a `hydra run` on the agent-CLI plane.** Ticket 04's
open question is closed by subtraction: there is no Anthropic API key and no credit (ticket 08),
LiteLLM cannot carry a Claude subscription and that is Anthropic's terms rather than a bug (ticket
04), and the raw-model plane does not ship in v1 (ticket 12). Judges on the raw plane would bill the
one Google AI Studio key behind `--allow-billed`, per review, where most proposals are rejected and
rejected proposals still pay for their reviews.

Model diversity comes from provider diversity, which is the only kind three subscriptions buy.

**A judge run needs no tools.** The diff goes in the prompt and one JSON answer comes back, so the
hook config for a judge denies every tool. Judges are the cheapest thing on this map to make safe.

**Judge identity is not pinnable and the record must carry it.** A judge is a provider version plus
whatever model that subscription served that day. `agy` moved 1.0.10 to 1.1.25 in the life of this
map, four bumps in the last three days alone. Record the provider version and the model the provider
reported, or "which model reviewed this" is unrecoverable a month later.

**All three configured judges must return a parseable verdict, or there is no review.** No shrinking
panel. Ouroboros tolerates one silent reviewer: `adaptive_quorum` returns 2 when 3 are configured,
and `quorum_met` fails only when responders drop below that. A panel of two when you configured
three is a quorum nobody agreed to, and it degrades quietly, which is the one property a gate must
not have.

Auth failure is not a review outcome either. The review aborts with ticket 12's remediation string
naming the provider and the command that fixes it. The cost is real and it is the right cost: one
expired login stops rung two until you log back in. Nothing was landing unattended anyway.

### What a critical finding means

**Union veto on critical.** One critical FAIL from any judge and the patch is not offerable. Nobody
votes. Ouroboros built its veto for an unattended loop; Hydra has a human at the end, which is an
appeal path they do not have, so the conservative rule is cheap here.

Severity keeps ticket 17's ceiling per checklist item. Hydra sets the ceiling, the model reports at
or under it, and under this answer `critical` has an exact meaning rather than a mood: the patch
cannot be offered for merge.

**A blocked patch still gets a branch.** Rejection is the common path, and a discarded rejection
teaches nobody anything. Approved proposals land on `ouroboros/<run-id>`, blocked ones on
`ouroboros/blocked/<run-id>` with the verdict in the commit message. Only an `ApprovedPatch` produces
the first.

**An unrecognised item name is reviewer-failed.** Their `item` field is free text and is never
checked against the checklist, so a model returning an invented name with `severity: "critical"`
blocks a commit the checklist says that item cannot block. Under a severity ceiling an unknown item
has no ceiling to sit under, so there is nothing to clamp it to. Fail closed.

### What Ouroboros may never touch

A patch touching a protected path is **refused before review, not blocked by it**. A protected path
checked by the review is a path the review can be talked out of.

Seven entries, matched case-insensitively because `write_file(path="bible.md")` on macOS writes
`BIBLE.md` and an exact-case membership test misses it:

- the review module: quorum arithmetic, the severity table, the checklist loader, `ApprovedPatch`
- the protected-path list itself
- the environment allowlist from ticket 21
- the hook-config writer from ticket 26
- the delegation policy from ticket 11
- the run-record writer from ticket 09
- the admission gate

**The whole patch is refused, never the offending hunks.** A partial apply is a patch nobody
reviewed.

A protected path changes when a human edits it by hand, like any other file in any other repo. That
is what the list is for, not a gap in it.

**No hot reload.** Ticket 06 found it nowhere in the paper, and D8 makes Hydra a compiled binary. A
self-edit takes effect on the next build a human runs. The consequence is worth naming because it
costs nothing to have: the binary running the review is always the pre-patch binary, so a patch
cannot weaken the gate reviewing it whatever it changes.

### The checklist: 29 items

Ticket 17 takes the critical-threshold rule close to verbatim with the MIT notice, and deliberately
does not take the 29 items, because they name another repo's contracts and the artifact-naming rule
beside them demands the reviewer name a live artifact in **this** repository. Seven of theirs are
project-independent and carry over. The other 22 are written here.

Ceiling is the highest severity a judge may report on that item. `advisory` items can never block.

**Carried over from ticket 17's survey, unchanged in intent:**

| Item | Ceiling | What the reviewer must find in the diff |
|---|---|---|
| `secrets_check` | critical | A credential, token, key, or a literal path to one, entering source, tests or fixtures |
| `security_issues` | critical | A named unsafe construct: unchecked path join, shell interpolation of untrusted text, a widened `unsafe` block |
| `tests_affected` | critical | A behaviour change with no test touched, naming the changed function and the test file that should have moved |
| `self_consistency` | critical | The diff doing something its own commit message does not claim |
| `code_quality` | advisory | Duplication, dead code, a panic path on a recoverable error |
| `cross_platform` | advisory | Case-sensitivity, path separators, an assumption that `HOME` looks like this machine's |
| `changelog_accuracy` | advisory | A user-visible change missing from the changelog entry |

**Hydra's own, 22 items. Each names an artifact this repo will contain.**

Gates and authority:

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `approved_patch_unreachable` | critical | A new constructor, a widened visibility, or a test helper that builds an `ApprovedPatch` outside the review module |
| `severity_ceiling_table` | critical | An item's ceiling raised, an item losing its ceiling, or a path carrying a model-supplied severity through unclamped |
| `reviewer_failure_fails_closed` | critical | A parse failure, timeout or refusal resolving to anything other than reviewer-failed. `[]` inside refusal prose is the case that produced this item |
| `panel_size_unshrunk` | critical | Any code that yields a verdict with fewer than the configured judges answering |
| `protected_path_set` | critical | An entry removed or narrowed, matching made case-sensitive, or the refusal moved to after the review |
| `admission_gate_order` | critical | A judge dispatched before fmt, clippy, build and test have passed, or an admission failure that still spends review budget |
| `baseline_test_provenance` | critical | A protected test read from the patched worktree rather than from the baseline sha |

The harness's own mechanisms:

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `env_allowlist_intact` | critical | A variable added to the allowlist without a named reason. `NODE_OPTIONS`, `PYTHONPATH` and `GEMINI_API_KEY` reaching a child at all |
| `hook_config_written_fresh` | critical | A hook config linked rather than copied, written once rather than per spawn, in the wrong per-provider location, or an `agy` spawn missing `--add-dir` |
| `hook_dialect_coverage` | critical | A change to the gate script that answers in fewer than three dialects. `claude` fails open on a shape it does not recognise, so a missing branch is a silent hole rather than an error |
| `billed_spend_gate` | critical | `--allow-billed` becoming inheritable, settable by the delegation policy, or readable from the environment |
| `delegation_depth_cap` | critical | The depth cap removed or made unreachable, or the MCP face gaining `--shared-tree`, a timeout override or depth |
| `curator_write_surface` | critical | Rung one gaining a write path outside the skill store, above all to config, the policy or the allowlist |
| `evidence_is_read_only` | critical | Ouroboros gaining a call to `prune` or `reap`, or any write under `~/hydra/runs/` |
| `worktree_containment` | critical | A proposer path that reads or writes outside its own worktree, or a gate whose path matching can be escaped by a relative path or a symlink |

The record contract:

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `record_append_only` | critical | A second writer to a run file, an in-place rewrite, or a run id that is not a UUIDv7 |
| `record_source_fields` | critical | `usage_source`, `status_source` or `permission_decision.decider` gaining an undocumented value or losing one |
| `raw_frame_preserved` | critical | A derived field overwriting the verbatim provider frame, or normalization dropping a frame it could not classify |
| `export_losslessness` | advisory | A record change that cannot survive the SFT and DPO export in D12, or redaction moving out of export |
| `provider_claim_dated` | advisory | A provider-specific assertion added to code or docs without the version and date it was observed |

Scope and shape:

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `no_new_crossing` | critical | Anything crossing between harness and Ouroboros other than the run record and the skill store. A shared module, a database, a socket (D5) |
| `no_daemon` | critical | A scheduler, background service, timer or anything that must be running for the harness to work (D10) |
| `spec_matches_code` | advisory | A behaviour change that leaves `docs/harness-spec.md`, `docs/ouroboros-spec.md` or `docs/run-record-contract.md` promising the old behaviour |

Fifteen of the 22 are critical, and every one of them is a mechanism some earlier ticket paid for.
That is the point of writing them from this map rather than from theirs.

### The stop button, and undoing a bad edit

`hydra evolve` is one pass, so the stop button is Ctrl-C on the command you typed. The proposer is
a detached child under ticket 11 and survives that, so `hydra status` reconciles it and `hydra reap`
commits its worktree to its own branch before removing it. Ticket 13 already guarantees reaping
never destroys work.

Nothing has landed, so undoing a rejected proposal is `git branch -D`.

For a self-edit you merged three weeks ago that turns out bad, the answer is `git revert` and no
Ouroboros machinery at all. The spec says that in one line, so nobody builds a rollback subsystem.
Ticket 17's apply records the verdict it consumed, so "which review approved the commit that broke
this" stays answerable from the run records.

### What this changes elsewhere

- **D11 is amended.** Rung two proposes rather than lands in v1, with a graduation condition
  recorded in the spec. Both rungs now stop at a branch a human merges.
- **Ticket 09 gains two fields**: per judge, the provider version and the model the provider
  reported. Judge identity drifts with the subscription and is otherwise unrecoverable.
- **Ticket 27 gains a case**: a judge run is a `hydra run` with every tool denied, which is the
  narrowest policy the gate will ever be asked to express. If the policy shape cannot say "no tools
  at all" cleanly, it is the wrong shape.
- **Ticket 04's open question is closed.** Judges run on the agent-CLI plane. Nothing on this map
  now needs the raw-model plane in v1.
- **Ticket 16 inherits the 29 items** as content for `docs/ouroboros-spec.md`, already written.
- **Ticket 28 was opened during this grilling.** The proposer is one gated child, but every run
  Hydra launches reads text that can be phrased as an instruction, and that is a different question
  from this one.

## Amended by ticket 19, 2026-09-03

**The export allowlist is a protected path.** A rung-two patch touching it is refused before review
rather than blocked by it, which is the treatment this ticket already gives protected paths. The file
decides which repos a run may have touched and still leave the machine, so a patch widening it is an
exfiltration path dressed as a config change, and no panel verdict should be able to approve one.

**The panel's verdicts are not preference-pair data in v1.** Ticket 19 admitted exactly one
preference source, the human correction inside an interactive run. Approved and refused patches for
one problem carry the same kind of signal in a different shape, a diff plus a verdict rather than a
trajectory plus a turn, so they are recorded and left out of the export format rather than bent into
it. Whether they become a second corpus sits in the map's fog.

## Amended by ticket 27, 2026-09-03

**Two more protected paths.** The delegation policy directory and the generated per-run gate script
join the export allowlist from ticket 19. A rung-two patch touching either is refused before review
rather than blocked by it, which is the treatment this ticket already gives protected paths.

**A judge run has a name now.** `judge` is one of the five named policies, and it allows nothing at
all. This ticket's case, that a panel member needs no tools because the diff arrives in the prompt
and one JSON answer comes back, turned out to be the test that shaped the policy set: the narrowest
policy the gate will ever express is one word rather than an enumeration of everything to forbid.

## Amended by ticket 28, 2026-09-03

**An eighth protected path**, the instruction preamble's config file. Ticket 28 puts the text of
Hydra's instructions-and-data rule in a human-authored config rather than in an injected skill, which
makes it Hydra's fourth protected config beside the delegation policy set, the environment allowlist
and the export allowlist. Same case-insensitive matching as the other seven, for the reason this list
already carries.

**A mismatch worth fixing in one place.** Ticket 27 counts three protected configs. This list holds
seven paths and ticket 19's export allowlist is not among them, and both tickets closed on the same
day. Ticket 16 reconciles the two rather than each document restating its own.

## Amended by ticket 29, 2026-09-03

**A ninth protected path**, the canonical MCP document. A rung-two patch touching it is refused
before review rather than blocked by it, which is the treatment this list already gives protected
paths. An MCP server definition is a command Hydra spawns, so a loop that can edit that document can
run arbitrary code inside every future run, which is D11's argument for striking config from rung
one's write surface applied to a file D11 did not know about. It is also Hydra's fifth protected
config, beside the delegation policy set, the environment allowlist, the export allowlist and ticket
28's instruction preamble.

**The mismatch ticket 16 has to reconcile moves to five and nine.** This list held eight paths and
ticket 27 counted three configs; ticket 28 took them to eight and four, and this ticket takes them to
nine and five. The generated per-run gate script, already on this list, now has a path:
`~/hydra/spawn/<run-id>/`.

## Amended by ticket 20, 2026-09-04

**"Model diversity comes from provider diversity, which is the only kind three subscriptions buy" is
false as written.** `agy --model claude-opus-4-6-thinking` serves Claude Opus 4.6 on the Google
subscription, so one subscription does buy a second model vendor. The sentence should say that the
panel takes its independence from provider diversity, which is a different and stronger claim than
the one about model vendors.

**The panel does not substitute a judge, and this ticket's own reasoning is why.** Ticket 20 looked
hard at routing a limited `claude` judge onto `agy` with a Claude id, because `judge` is the one
policy where the harness gap cannot bite: it allows nothing at all, the diff arrives in the prompt
and one JSON answer comes back. It fails on the panel rather than on the model. Substituting produces
`agy` running Claude 4.6, `cursor-agent`, and `agy` running Gemini, which is two judges on one
binary, one login, one quota and one system-prompt harness, each paying `agy`'s 16,338-token floor.
If the Google subscription is the limited one, two judges die together and the panel sits at one
responder. That is a gate degrading quietly, which this ticket names as the one property a gate must
not have.

**A rate-limited judge aborts the review**, in the shape this ticket already gives auth failure: a
remediation string naming the provider and what fixes it, and rung two waits. The reasoning
transfers unchanged, since nothing was landing unattended anyway.

**One early abort, which is an optimisation rather than a decision.** Before starting a review, rung
two may read the last `claude` run's `rate_limit_event` status from the run records and abort rather
than spending the other two judges' quota on a review that cannot reach quorum. It fires on the
vendor's own `allowed` / `allowed_warning` / `rejected` enum, never on a threshold Hydra sets, since
ticket 14 ruled those out and ticket 20's research caught `utilization` moving backwards. A reading
whose `resetsAt` has passed is stale and says nothing.
