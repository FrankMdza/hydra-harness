# Hydra Ouroboros spec

Both rungs of self-improvement: the curator that evolves skills, and the reviewed loop that edits
Hydra's own source.

This is one of three documents. `docs/harness-spec.md` covers how Hydra launches, feeds and records
the three agent CLIs. `docs/run-record-contract.md` fixes the format Hydra writes and this document
reads. The three are meant to be read together, and nothing crosses between the harness and
Ouroboros except the run record and the skill store (D5).

Provider versions this document was written against, read 2026-09-04 at 17:33:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.261 | 2026-09-04 17:33 |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-04 17:33 |
| `agy` | 1.1.26 | 2026-09-04 17:33 |

**`claude` moved from 2.1.257 to 2.1.261 between the two sibling documents and this one**, on the
same day, which is the fourth time a provider has moved under this design. The two sibling documents
carry their measurements against 2.1.257 and those measurements stand as measurements. Re-read all
three versions before trusting any provider-specific claim, and re-measure rather than inherit.

## 1. What Ouroboros is

Two loops that read the run records and propose changes. Neither lands anything.

**Rung one, the curator.** Reads the corpus for failures, proposes changes to the skill store,
commits them to a branch. Ships in v1.

**Rung two, reviewed core evolution.** Proposes a patch to Hydra's own Rust, runs it past a
deterministic gate and a panel of three provider CLIs, commits it to a branch. Fully specified here
and shipped later, so that shipping it is not a rewrite (D11).

**Both rungs stop at a branch a human merges.** That is the single most important sentence in this
document and it was not the original design. D11 had rung one editing skills, prompts and config in
place. Two things moved it: a loop that can edit its own permissions walks around every gate on this
map, and the reference implementation ships `OUROBOROS_REVIEW_ENFORCEMENT: "advisory"` after 1,324
commits, which is what a blocking gate becomes once it is the only thing in someone's way. A human
merge is a gate nobody turns off with an environment variable.

Ouroboros is Rust, in the same binary as the router (D8, extended by ticket 17). Reviewer and
reviewed sit on one compile gate, and no process boundary between the supervisor and the thing
rewriting it has to be kept correct forever. The evidence is the reference implementation's own
source: `preflight_runner.py` spends 1,488 lines of hermetic test running, plugin verification and
`addopts` tamper-checking, standing in for what `cargo build && cargo test` does.

**Every number this document declines to pick is declined on purpose.** Hydra has run zero times. A
threshold invented now is a guess with a decimal point on it. Where a number will eventually be
wanted, this document names the signal it should be picked from and says who picks it.

## 2. What both rungs share

### 2.1 Three rules about not trusting a model

These come from ticket 05 by way of ticket 17, and they are the part of this design most likely to
outlive it. The first two bind both rungs, because a curator is a gate as much as a panel is.

**A model never sets what blocks.** Severity has a ceiling per checklist item, set by Hydra. The
model may report at or under it, never above. The reference implementation lets the reviewing model
set severity freely, one reviewer of three emitting `severity: "critical"` blocks the commit, and
the only thing in between is a 27-line prompt constant asking the model to police itself. Fixing
severity outright would throw away the reviewer's judgement that something is milder than its
category. A ceiling keeps that and removes the escalation path.

**A model's silence is not consent.** Three outcomes, not two: pass, block, and reviewer failed. A
response that does not parse into the expected shape is the third and it fails closed.
`empty_array_is_verified_clean` exists in the reference implementation because `[]` sitting inside
refusal prose ("I cannot review this diff... []") entered the quorum as an approval. With one review
and no memory, a refusal read as a pass is a self-edit that shipped unreviewed while nothing
downstream noticed.

**The apply path is unreachable, not merely gated.** The function that lands a self-edit takes an
`ApprovedPatch` that no code outside the review module can construct, so a self-edit that skipped
review does not compile. Not a runtime check, not a flag, not a policy, and no call to forget.

Said plainly so the type does not imply more than it has: it proves a review happened, never that
the review was any good, and it stops at Hydra's boundary. A child provider writing a file inside its
own worktree never touches this path. The gate in `docs/harness-spec.md` section 6.6 is what covers
that, and it is a different mechanism.

**A guarantee written in a prompt is not a guarantee.** Every limit either rung claims is enforced
in code, at the tool-dispatch layer or by the gate, or it does not exist. Hermes told its reviewing
model that archiving moves a directory into `.archive/` and is recoverable, while the tool it called
ran `shutil.rmtree`. That is the failure this whole document is named after.

### 2.2 What neither rung may ever do

| Never | Why | Enforced by |
|---|---|---|
| Call `hydra prune` or `hydra reap` | cleanup is a command a human types, so nothing the loop does can destroy the evidence it learns from | `evidence_is_read_only`, section 5.3 |
| Call `hydra export` | export is the one path by which anything leaves this machine (harness spec section 14) | `evidence_is_read_only`, section 5.3 |
| Write anywhere under `~/hydra/runs/` | a loop that learns from evidence must not be able to edit the evidence | the gate, every policy (harness spec 7.5) |
| Write any of the five protected configs | harness spec section 15.1 | the gate, every policy, and admission refusal |
| Widen its own policy | the delegation policy is the first protected config, and monotonic narrowing (harness spec section 7.3) bounds it at run time as well | the spawn check, plus the two above |

**The five rows do not share a mechanism, and the third column is there because assuming they did
was wrong.** An earlier draft said the gate enforced the first three on every run under every policy.
It does not, and the seam check caught it.

**Rows three, four and five are path blocks, and the gate delivers them outright.** A hook sees the
path and refuses the write before it reaches disk, measured on all three providers, so these hold for
any run rather than only for Ouroboros'. Harness spec section 7.5 lists the denied paths as one set, and since ticket 40 the store, its snapshots and the pruned records are on it beside the corpus.

**Rows one and two are not a gate's job at all.** `hydra prune`, `hydra reap` and `hydra export` are a
binary reached through `shell`, so a gate rule against them is an argv match, with the failure mode
the harness spec's pattern floor already admits to: silent permission on the spelling nobody wrote
down. `./target/debug/hydra export` walks past it. What actually holds these two is that no Ouroboros
code path calls those commands, which is a property of Hydra's own source, and
`evidence_is_read_only` is the checklist item that keeps it true under a self-edit. That item is now
load-bearing rather than belt-and-braces, which is worth knowing before anyone lowers its ceiling.

Stating the weaker mechanism honestly is the point. A floor described as a guarantee is section 2.1's
last rule with the roles swapped.

### 2.3 The reference implementation

`github.com/razzant/ouroboros`, MIT, Python, read at tip `e9dfb359963789516eed4a3960c2cfaea95044b0`
dated 2026-08-30.

**Read at a sha, vendored nowhere, depended on never.** Following function-local imports, every seed
module reaches the same 327 modules and 228,207 lines, so there is no acyclic seam anywhere in the
package. Nothing in the repo promises an interface. 418 commits landed in one week with the rate
still climbing, and 61% of them are authored by the agent itself. The review panel loads
`DEVELOPMENT.md` and `ARCHITECTURE.md` whole into its prompt, 867 KB of another project's
constitution, and reviewing a foreign repo still reads Ouroboros' own checklist off disk.

Tracking upstream is a full-time merge job. Vendoring a snapshot of a package that grew 38% new
modules in two months is a fork on day one whatever it is called. So the reading is the thing worth
having:

```
git clone https://github.com/razzant/ouroboros
git -C ouroboros checkout e9dfb359963789516eed4a3960c2cfaea95044b0
```

**One thing is copied**, the critical-threshold rule in section 5.1, close to verbatim with the MIT
notice. **The 29-item checklist is not copied**, because its items name their contracts, their
frozen paths and their `BIBLE.md`, and the rule beside them demands the reviewer name a live
artifact in *this* repository. Hydra's own items are in section 5.3.

**Two numbers about that repository should stop being quoted as design constraints.** The paper's
63.5% block rate is nowhere in the code and is not computed by it, so it describes one deployment
rather than a property of the software. And `config.py:208` ships
`OUROBOROS_REVIEW_ENFORCEMENT: "advisory"`, so after 1,324 commits review does not block by default
at all. Both point the same way, which is that blocking review is harder to live with than it looks,
and rung two should be sized down rather than up.

The paper itself holds up. arXiv 2608.08311, Razzhigaev et al., v3 revised 31 August 2026: 86.74%
on Terminal-Bench 2.1 audited, 90.69% on OSWorld-Verified, both on Opus 5, and a real 161-day
deployment. The mechanism is what our originating document got wrong.

## 3. Rung one: the curator

### 3.1 Invoked, not scheduled

You type `hydra curate`. It reads the whole corpus, ranks by frequency, and proposes.

**No occurrence threshold and no window in v1.** The ticket that designed this asked for "durable
error classes" in numbers, and the honest answer is that the number does not exist yet. Nothing
lands unseen, so a threshold would be noise control on a list a human reads rather than a safety
mechanism. The scheduled version is step two, gated on having enough runs to tune against.

Ticket 05's rule to defer a first-ever run by a full interval costs nothing here, because there is no
interval to defer.

### 3.2 What it learns from

Five classes. The fifth is not one anybody asked for and it is the one that makes the other four
trustworthy.

| Class | Derived from | Record |
|---|---|---|
| Abandoned run | a dead pid with no `run_end` | contract sections 5.2 and 6 |
| Human rejection | a `correction` event in an **interactive** run | contract section 7.9 |
| Repeated tool errors | the tool-call adapter's classification | contract section 7.6, harness spec section 13 |
| Retry loop | the shape of the `parent_run_id` tree | contract section 10 |
| Clean-looking failed run | the outcome derivation disagreeing with the tool results inside it | contract section 8 |

**The fifth class earns its place because it makes the other four unreliable.** Three of three
providers close a run reporting success with a real denial or a failed tool inside it. `claude`
closed `terminal_reason: completed, is_error: false` with a denial sitting in `permission_denials`.
`agy` returned `status: SUCCESS`, an empty response and exit 0 on a run whose only tool was
auto-denied. `cursor-agent` returned `is_error: false` with one tool rejected and one failed. All
three exit 0 and report success on a run where the gate refused everything. Anything keyed off the
terminator files a failure as a success, which is why the contract derives every outcome from the
event stream.

**Two of these were recorded as underivable and came back alive.** An abandoned run looked
unproducible while the evidence pass was reading vendor transcripts, where a Cursor run killed at six
seconds leaves zero bytes. Hydra's own envelope carries the pid and the process start time, written
before the child starts, so Hydra has the envelope even when the child wrote nothing. And a human
rejection looked unproducible because `claude` labels a headless auto-deny the same way it labels a
real one; the recorded-against-interactive split answers it, and section 3.3 gives the exact
mechanism that was measured after the fact.

### 3.3 Human rejection, by mode and by mechanism

**The rule is that a recorded run has no human in it.** Hydra's own policy answers permissions in a
recorded run, so `user-rejected` there is Hydra's policy talking and Hydra knows what it said.
Interactive plus a rejection is a person.

Training on a policy denial would teach a model to obey a config that changes, and a pair recorded
today would be wrong the day the policy widens. So a policy denial is never a correction, and that is
written down rather than left to judgement.

**The producer is one of three and the discriminator is a conjunction.** Measured on `claude` 2.1.261
on 2026-09-04: `toolDenialKind == "user-rejected"` **and** `entrypoint == "cli"`. Both conditions.
`user-rejected` alone means the ask path answered, and under `sdk-cli` the answerer is code. The
contract's section 7.9 carries the harvest rule, the fields, and the one thing this shape cannot do,
which is tell a redirection from a change of subject. Rung one inherits that caveat rather than
solving it: a human who rejects a call and then changes the subject produces a record
indistinguishable from a correction.

### 3.4 Classification, and the honesty rule

`agy` gives a permission denial and a file-not-found the same `state: "ERROR"` and the same
`error.type: "TOOL_ERROR"`, and both messages contain the word "permission". So a substring rule is
wrong rather than merely weak.

**The per-provider adapter classifies, and it may return `unclassified`.** Read-time normalization
sounds right until you notice it needs one rule fitting three providers, which is exactly what does
not exist. An adapter that says it cannot tell a denial from a missing file on this provider gives
the curator something honest to count, and keeps the gap visible per provider rather than averaged
away.

**Guessing here produces a curator that rewrites a skill because a file was missing.** That is the
whole argument. An `unclassified` count is a number the curator may rank on and may not diagnose
from.

### 3.5 Where the curator runs

**It is a `hydra run` like any other**: recorded, delegated through the delegation contract, under
the `curate` policy, which reads the corpus and writes only inside the skill store.

That dissolves three questions the design started with. Which model reviews is a config value. Which
auth plane it runs on is the agent-CLI plane, like everything else in v1. What it costs is measured
the way every run is measured, and its own failures land in the corpus.

**Runs whose parent is the curator are excluded from the corpus the curator reads.** Otherwise it
learns from itself. The same exclusion applies to the export corpus, for the same reason, and the
rule is stated once here rather than twice: `parent_run_id` makes it computable and this document
owns it.

**`curate` starts at depth 0, from a human.** The five policies are a partial order rather than a
chain, and `curate` and `standard` are incomparable: one writes to the store and cannot touch a
worktree, the other is the reverse. An incomparable pair is refused rather than resolved, so a
`standard` parent cannot delegate a `curate` child. That reads like the right answer on its own,
because rung one's write surface is what a laundered instruction would most want.

### 3.6 What it may write, and what it may never write

**The skill store, and nothing else.**

D11 said skills, prompts and config. It was written before anyone knew what Hydra's config was.
Config is struck because Hydra's config is the delegation policy, the environment allowlist, the
export allowlist, the instruction preamble and the canonical MCP document, and a self-improvement
loop that can edit its own permissions walks around every gate on this map, including the
`ApprovedPatch` type that section 2.1 spends a rule building. Prompts are struck for a duller reason:
Hydra injects skills and has no separate prompt, so the word points at nothing.

This is the hardest line in rung one's design and it should read as one.

**The curator writes into its own git worktree of the store, never into `~/hydra/skills/` itself.**
A run pins the store at a sha and links into an immutable snapshot, so the live working tree is only
ever read at launch, where a dirty store is committed before pinning. A curator writing into that
same working tree would collide with the next launch's commit. Its worktree is created per run at
`~/hydra/skill-worktrees/<run-id>/` and its commits land on `ouroboros/curate/<run-id>` in the
store's own repo. Ticket 40 settled this on 2026-09-06 and harness spec 7.2 was the paragraph that
moved. The worktree is checked out at the sha the run pins, and the pass does the pinning itself
before the checkout, so a dirty store is committed by `hydra curate` rather than by the launch that
follows it and the curator never reads a store its own launch is about to move past. Ticket 41.

**Every limit above lives at the tool-dispatch layer and the gate, and none of it lives in a
prompt.**

### 3.7 Nothing lands unseen

The skill store's sidecar makes a skill with `origin = "ouroboros"` editable in place. **In v1 it is
not.** Every change is a proposal: the curator commits to `ouroboros/curate/<run-id>` in the store's
repo and a human merges it or does not. The store's git history is the queue.

D11 ships rung one first so you can find out whether it is any good, and a loop that edits silently
gives you nothing to judge.

**This is a graduation condition, not a permanent rule.** The default for `origin = "ouroboros"`
flips once enough proposals have been accepted with none bad. The count is not picked now, it is
picked from the corpus, and it is a decision someone makes rather than a default nobody revisits.

### 3.8 Whether a change helped

**Measure only the signal the change was made for.** A proposal names the failure class and the skill
it responds to, and that name becomes the measurement. If it proposed an edit because unclassified
tool errors on skill Y fired seven times, the measure is that count afterwards.

Narrow on purpose. A general before-and-after needs comparable work and Hydra has no notion of task
similarity. A/B across store shas needs repeated tasks, which real work does not provide. This one
needs neither.

**It answers the scenario rung one was designed against.** The same skill rewritten three weeks
running, each version worse, appears as three consecutive proposals against one skill for one signal
that never drops. That is also the thrashing signal rung two depends on, per section 4.12.

### 3.9 The curator's rules

Fifteen rules, extracted from Hermes, each with the reason behind it, because rules without reasons
are cargo cult. Eleven apply verbatim. Four change, and the fourth column says how.

| # | Rule | Reason | In Hydra |
|---|---|---|---|
| 1 | Scope edits to skills the loop itself created, not to "not bundled and not hub" | foreground skill creation is user-directed and those skills belong to the user | verbatim, as `origin` on the sidecar |
| 2 | Never touch a pinned skill | the pin exists to stop irrecoverable loss | **amended**: the patch exemption is dropped |
| 3 | Anchor staleness on activity, falling back to creation only when there has never been any | without the fallback an untouched skill anchors to nothing and is immortal | verbatim |
| 4 | Never let usage counters gate an overlap judgement | counters are sparse, so zero use is absence of evidence rather than evidence of irrelevance | verbatim |
| 5 | Require the intent declaration at the moment of the action, not from a summary afterwards | the direct fix for a regression where the model's free text did not match what it did | verbatim, as commit-message discipline |
| 6 | Validate that a claimed merge target exists, and re-check it after the run | guards a hallucinated umbrella that was never created | verbatim |
| 7 | Snapshot before every mutating pass, and make rollback itself reversible | a rollback that cannot be undone turns one mistake into two | verbatim, and free: the store is a git repo |
| 8 | Treat a failed snapshot as a warning rather than a blocker | a transient disk error should not silently disable the curator forever | **amended**: it blocks. A failed snapshot means git failed, which is not a condition to proceed through |
| 9 | Exclude externally managed content from the snapshot and any rollback | overwriting it desyncs it from whatever owns it | verbatim, as the read-only index of the user's own skills |
| 10 | Rewrite other subsystems' references to a skill when it is consolidated or renamed | a lookup by name that skips silently on a miss is a duty that quietly stops firing | verbatim |
| 11 | Defer the first-ever run by a full interval | it fixed a real incident of a curator hitting a freshly migrated library | **moot**: invoked rather than scheduled, so there is no interval |
| 12 | Run deterministic transitions before the model pass and independently of it, and let reactivation reverse a stale mark | state that only moves one way punishes a skill for a temporary lull | verbatim, and this is the one to keep loudest |
| 13 | Install a non-interactive approval callback on anything shell-like before it runs | an approval prompt on a background thread with no TUI attached deadlocks | verbatim, and the gate is what implements it |
| 14 | Make archive and delete the same real operation everywhere, or do not call the destructive one archive | the rule Hermes' own code violates | **changed in character**: a git store makes any deletion a recoverable commit, and Ouroboros cannot call prune or reap, so `absorbed_into` survives as commit-message discipline rather than as safety |
| 15 | Enforce a dry run's non-mutation guarantee at the tool-dispatch layer rather than in prompt text | a "please don't" to a model is not the same class of guarantee as a runtime deny | verbatim, and it is section 2.1's third rule under another name |

**Rule 12 is the one to keep loudest.** `claude` fails open on a hook response in a dialect it does
not recognise, which is what a gate looks like when it trusts a model's output shape. A curator you
invoke by typing a command will make it tempting to let one model call decide everything.

### 3.10 What rung one never learns from

The outcomes in the contract's section 8 marked "never": `gated`, `auth_failed`, `skill_collision`,
`mcp_unavailable`, `rate_limited`.

**A skill does not need evolving because a token expired.** The same argument covers the other four.
A run that failed on too narrow a policy is a policy bug rather than a skill bug, a collision is a
naming bug, an MCP shortfall is a config bug, and a rate limit is a quota.

**Recorded and learned-from are different questions.** Every one of those runs is on disk in full,
verbatim. A reader who collapses "rung one never learns from this" into "throw it away" has the
record backwards.

Nothing in v1 reads the signal those runs carry. That breaks no seam, because D5 constrains what
Ouroboros reads and says nothing about fields the harness writes with no reader, and it is on the map
as fog rather than as a ticket.

### 3.11 A sixth class, named and not built

**The stalled run.** The five classes above all derive from something the record holds. A run that is
alive and emitting nothing holds nothing, and liveness is a pid plus a process start time, so a
wedged run and a busy run look identical.

Message deltas are the cheap heartbeat and the harness turns them off for reasons that hold
independently of this. `agy` keeps one by accident, emitting `step_update` with `state: ACTIVE` and
shipping no flag to stop it, so the signal exists on exactly one provider of three today.

It is left unbuilt on rung one's own principle: a timeout invented before the system has run once is
a guess.

## 4. Rung two: reviewed core evolution

### 4.1 The shape of one pass

`hydra evolve` is a single pass. One proposer run, the protected-path check, the admission gate, one
panel of three, one branch, exit. No loop, no retries, no memory.

```
human types hydra evolve
  -> proposer run           a hydra run child, its own worktree, produces a diff
  -> protected-path check   refuses the whole patch before any review
  -> admission gate         cargo fmt, clippy, build, test, in the worktree
  -> panel                  three judge runs, one per routable provider
  -> branch                 ouroboros/<run-id> or ouroboros/blocked/<run-id>
  -> exit
```

**Nothing in that list is a retry.** Iterating against the compiler inside the proposer's own run is
expected and free, and it is not a review round. Without this sentence someone reads "no retries" as
forbidding the proposer from fixing its own build.

### 4.2 The authority boundary

**No supervisor process.** The boundary is the parent and child boundary Hydra already has. A process
boundary between a supervisor and the thing rewriting it is something you have to keep correct
forever, and the reference implementation carries `supervisor/` at 27,191 lines partly for want of a
compiler.

The proposer is a `hydra run` child: its own worktree, no credential of any kind, an environment
built from the allowlist, a hook config Hydra wrote at spawn. The supervisor is the Hydra process the
human invoked. It alone reads verdicts, constructs `ApprovedPatch`, and runs git against the real
repo.

Three mechanisms enforce that and all three already exist:

- the child has no path to the real tree
- the gate refuses writes outside its worktree
- `ApprovedPatch` makes the apply path uncompilable from anywhere else

**The child runs under the same binary version it is editing, and that is harmless.** It edits files.
The running process is already loaded.

### 4.3 The sandbox, and the hole in it

**A worktree, the gate and the environment allowlist. Not a container.**

A container has to reproduce the provider's own login or be handed credentials, and D6 forbids the
second. The first was measured and it costs more than it buys: a wrong `HOME` silently logs three of
four providers out. So the proposer runs with the real `HOME`, and the gate is what keeps it out of
`~/.claude`, `~/.cursor`, `~/.agents` and `~/hydra/runs`. Read blocking is confirmed on all three
providers, `beforeReadFile` on Cursor and `PreToolUse` on the other two.

**Network cannot be cut**, because the CLI needs it to reach its own model. That is a hole and this
document calls it one rather than dressing it up as a control.

**Run records reach the proposer as a read-only brief the supervisor assembles, never as a directory
it can open.** Rung one gets the directory and rung two gets a brief, and the asymmetry is
deliberate rather than an inconsistency: rung one's entire job is the corpus, while rung two's job is
source code and it has no reason to shop for evidence. The gate denies writes under `~/hydra/runs/`
for both.

### 4.4 Protected paths: refused before review

**A patch touching a protected path is refused before review, not blocked by it.** A protected path
checked by the review is a path the review can be talked out of. **The whole patch is refused, never
the offending hunks**, because a partial apply is a patch nobody reviewed.

The list is twelve entries and it lives in `docs/harness-spec.md` section 15.2, which reconciles it
with the run-time denials the gate applies. This document does not restate it. Two things about it
belong here, because they are rung two's:

**Ten of the twelve are what admission refuses.** Every ticket that stated a number said nine, the
items those tickets named came to eleven, and the seam check added the run records directory as a
twelfth. The arithmetic correction and the new row are different kinds of change and section 15.2
keeps them apart. What matters here is the smaller number: two entries carry a run-time denial and no
admission refusal, the generated gate script and the run records directory, because a patch cannot
propose changing a file that is generated per run or a corpus that is not source. **Rung two's
refusal is over the other ten.**

**Matched case-insensitively**, because `write_file(path="bible.md")` on macOS writes `BIBLE.md` and
an exact-case membership test misses it.

**A protected path changes when a human edits it by hand**, like any other file in any other repo.
That is what the list is for, not a gap in it.

**The opening scenario is answered here rather than by the panel.** A patch that removes a check in
the quorum touches the review module, which is entry 1. It is refused at admission. Two of three
judges approving it is not a state this design can reach.

### 4.5 The admission gate

`cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, `cargo test`, in the worktree, all
four before a single judge is dispatched. Deterministic, and they run whether or not any model agrees
with them.

**Admission failure is not a review outcome.** The pass ends carrying the compiler output on its
branch, and spends no review budget. It ends `completed`, because the gate did its job; section 4.10
says why the pass's outcome is about the pass rather than about the patch. This is the work that
costs 1,488 lines of Python in the reference implementation and nothing here, which remains the
strongest single piece of evidence on this map for D8.

Two refinements the reference implementation earned the hard way:

**Protected test files come from the baseline sha, not from the patched worktree.** A self-edit that
adds its own passing test proves nothing.

**The order is load-bearing and it is a checklist item.** A judge dispatched before the four commands
pass, or an admission failure that still spends review budget, is `admission_gate_order` in section
5.3.

### 4.6 The panel

**Three judges, one per routable provider, each a `hydra run` on the agent-CLI plane under the
`judge` policy.**

The alternative was judges on the raw-model plane, and it is closed by subtraction: there is no
Anthropic API key and no credit, LiteLLM cannot carry a Claude subscription and that is Anthropic's
terms rather than a bug, and the raw-model plane does not ship in v1. Judges on the raw plane would
bill one Google AI Studio key behind `--allow-billed`, per review, in a system where most proposals
are rejected and rejected proposals still pay for their reviews.

**The panel takes its independence from provider diversity.** That is a different and stronger claim
than one about model vendors, and the difference matters: `agy --model claude-opus-4-6-thinking`
serves Claude Opus 4.6 on the Google subscription, so one subscription does buy a second model
vendor. What it does not buy is a second binary, a second login, a second quota and a second
system-prompt harness.

**The panel never substitutes a judge**, and that fact is why. Routing a rate-limited `claude` judge
onto `agy` with a Claude model id looks safe, because `judge` is the one policy where the harness gap
cannot bite: it allows nothing at all, the diff arrives in the prompt and one JSON answer comes back.
It fails on the panel rather than on the model. Substituting produces `agy` running Claude,
`cursor-agent`, and `agy` running Gemini, which is two judges on one binary and one quota, each
paying `agy`'s 16,338-token floor. If the Google subscription is the limited one, two judges die
together and the panel sits at one responder.

**A judge run needs no tools.** The diff goes in the prompt and one JSON answer comes back, so
`judge` denies everything. Judges are the cheapest thing on this map to make safe, and `judge` is the
narrowest policy the gate will ever be asked to express, which is what shaped the policy set into
five names rather than an enumeration of everything to forbid.

**All three configured judges must return a parseable verdict, or there is no review.** No shrinking
panel. The reference implementation tolerates one silent reviewer: `adaptive_quorum` returns 2 when 3
are configured. A panel of two when you configured three is a quorum nobody agreed to, and it
degrades quietly, which is the one property a gate must not have.

**Judge identity is not pinnable and the record carries it.** A judge is a provider version plus
whatever model that subscription served that day, and `agy` moved through four versions in four
days, 1.0.10 to 1.1.23 to 1.1.24 to 1.1.25 to 1.1.26. The
contract's `run_start` puts `provider_version` and `model` on every run, so the requirement is met by
the ordinary record rather than by anything review-specific. Without it, "which model approved the
commit that broke this" has no answer a month later.

**Auth failure is not a review outcome either.** The review aborts with the remediation string naming
the provider and the command that fixes it, and rung two waits. One expired login stops rung two
until you log back in. That is the right cost, because nothing was landing unattended anyway.

### 4.7 The early abort on rate limits

Before starting a review, rung two may read the last `claude` run's `rate_limit_event` status from
the run records and abort rather than spending the other two judges' quota on a review that cannot
reach quorum.

**It is an optimisation rather than a decision**, and it fires on the vendor's own `allowed`,
`allowed_warning` and `rejected` enum, never on a threshold Hydra sets. Nothing keys on
`utilization`, which was caught moving backwards, 0.24 to 0.05 on `seven_day` against an unchanged
`resetsAt`. A reading whose `resetsAt` has passed is stale and says nothing.

A judge that is rate-limited mid-review aborts the review in the same shape as an auth failure.

### 4.8 What a critical finding means

**Union veto on critical.** One critical FAIL from any judge and the patch is not offerable. Nobody
votes. The reference implementation built its veto for an unattended loop; Hydra has a human at the
end, which is an appeal path they do not have, so the conservative rule is cheap here.

Severity keeps its ceiling per checklist item. Under this design `critical` has an exact meaning
rather than a mood: **the patch cannot be offered for merge.**

**An unrecognised item name is reviewer-failed.** In the reference implementation the reviewer's
`item` field is free text and is never checked against the checklist, so a model returning an
invented name with `severity: "critical"` blocks a commit the checklist says that item cannot block.
Under a severity ceiling an unknown item has no ceiling to sit under, so there is nothing to clamp it
to. Fail closed.

### 4.9 Branches, and where the verdict lives

**Every pass ends with exactly one branch.** Rejection is the common path, and a discarded rejection
teaches nobody anything.

| Pass outcome | Branch | Carries |
|---|---|---|
| Offerable | `ouroboros/<run-id>` | the verdict block; only an `ApprovedPatch` produces this branch |
| Blocked by the panel | `ouroboros/blocked/<run-id>` | the verdict block, including the critical findings |
| Refused at admission | `ouroboros/blocked/<run-id>` | the compiler output, and no verdict |
| Refused on a protected path | `ouroboros/blocked/<run-id>` | the path that matched, and no verdict |

**Everything a pass decided about the patch lives in the commit message**, and it names the pass run
id, the proposer run id and the three judge run ids. The verdict, the critical findings, the compiler
output from a failed admission gate and the path that matched a protected-path refusal are all the
same kind of thing, a result about the patch, so splitting them across two artifacts by which stage
produced them would be arbitrary.

That is a deviation from ticket 17's "the apply records the verdict it consumed into the run record",
and the reason is a timing fact nobody had when that was written: the panel runs after the proposer's
process has exited, and `run_end` is written when `wait()` returns, so there is no open run file to
append to. Ticket 17's actual requirement survives, since what it argues for is that unreachable and
auditable are different properties. A commit message is also the one place a later patch cannot edit,
which the tree is not.

**The audit path is git plus the run records.** A merge commit's parent is `ouroboros/<run-id>`,
whose message names three judge run ids, whose records hold each judge's verdict as its final
assistant text and each judge's identity as `provider_version` plus `model`. Nothing review-specific
had to be added to the record for that to work.

### 4.10 The pass record

A pass leaves five run records: the pass itself, the proposer, and three judges.

**The pass is a harness run**, which is the contract's second kind of run: no provider, a body of
`run_start` and `run_end` with nothing between, `pid` belonging to the Hydra process the human
invoked, and `command: evolve`. The contract's section 5.5 owns the shape and section 2 owns the
definition.

**It exists because its children need a parent that exists.** Without it the proposer and the three
judges are four unrelated runs at depth 0, and a human who hits Ctrl-C on `hydra evolve` leaves
detached children that nothing can group. The two alternatives are worse: parenting the judges to the
proposer claims the proposer spawned its own reviewers, and a pass file outside `runs/` gives the
harness a second source of truth, which is what D10 exists to prevent.

**A pass's outcome describes the pass, not the patch.** A pass that refused a patch on a protected
path did its job, so it is `completed`. So is a pass whose panel blocked and a pass whose admission
gate caught a build failure. `failed` means the pass itself broke, git or the worktree. A blocked
patch is not a failed run, and filing one as a failure would put a gate that worked into the corpus
as a lesson.

**Rung one skips harness runs**, so an interrupted pass never reads as an abandoned run worth
learning from. `hydra status` is its reader. The children stay in the corpus as ordinary provider
runs, which is where the proposer's trajectory is actually worth something.

### 4.11 No hot reload, and how a bad edit is undone

**No hot reload.** It appears nowhere in the paper, and D8 makes Hydra a compiled binary. A self-edit
takes effect on the next build a human runs.

The consequence is worth naming because it costs nothing to have: **the binary running the review is
always the pre-patch binary**, so a patch cannot weaken the gate reviewing it, whatever it changes.

**The stop button is Ctrl-C on the command you typed**, because `hydra evolve` is one pass rather
than a loop. The proposer is a detached child and survives that, so `hydra status` reconciles it and
`hydra reap` commits its worktree to its own branch before removing it. Reaping never destroys work.

**Nothing has landed, so undoing a rejected proposal is `git branch -D`.**

**For a self-edit you merged three weeks ago that turns out bad, the answer is `git revert` and no
Ouroboros machinery at all.** That sentence is here so nobody builds a rollback subsystem.

### 4.12 No obligation ledger, and the signal that says to build one

Rung two blocks on one review, with no memory across attempts and no obligation ledger.

The reference implementation spends 5,009 lines on the ledger across `review_state.py`,
`review_custody.py` and `review_evidence.py`. Findings carry an `obligation_id`, blocked attempts
write structured obligations the next round must address by name, and repeated blockers synthesise
commit-readiness debt that keeps the blocking path shut until cleared. It defends against a reviewer
being argued down over many rounds, which is a problem you get once rung two has rounds.

**Shipping the anti-thrashing prompt rules without the state underneath is the worst of the three
options**, because it looks like a defence in the spec and is not one. So three prompt constants that
looked worth copying are dropped with the ledger: the two anti-thrashing rules and the convergence
rule all describe behaviour across retries.

**The ledger is the first thing to build when thrashing shows up in the run records**, and rung one's
measurement in section 3.8 is the detector: consecutive proposals against one artifact for one signal
that never drops.

## 5. The review checklist

**Thirty items, 24 critical and 6 advisory.** Every stated count in the tickets this comes from is
wrong by one or more, in the same direction, three times over. The numbers here are counted from the
rows below.

| Group | Items | Critical | Advisory |
|---|---|---|---|
| Carried over (section 5.2) | 7 | 4 | 3 |
| Hydra's own (section 5.3) | 23 | 20 | 3 |
| Total | 30 | 24 | 6 |

**Ceiling is the highest severity a judge may report on that item.** An `advisory` item can never
block, whatever a model says about it.

### 5.1 The critical-threshold rule

Adapted from `docs/CHECKLISTS.md` in `github.com/razzant/ouroboros` at `e9dfb359`, MIT licensed,
used with attribution. It is the highest-value-per-line text in that repository and paraphrasing it
to dodge an attribution line would be a bad trade.

> Before marking any item CRITICAL you must be able to answer yes to all of:
>
> 1. I can name the exact file, symbol, function, test, or config path **in this repository** that
>    makes this problem live right now.
> 2. That artifact actually appears in the diff or the touched-file context I have been given, not
>    just in a hypothetical future scenario or an external environment.
> 3. The fix requires a change to **this diff**, not a follow-up task or a speculative guard.
>
> If you cannot satisfy all three, use advisory, not critical.
>
> One root cause is one FAIL entry. Do not split one underlying problem into several FAIL items that
> all require the same change. Do not reformulate a fixed concrete issue into a broader future-risk
> variant: if the named artifact is fixed, mark PASS, and raise a new advisory if a broader concern
> remains.
>
> Coverage is semantic, not numerical. Zero or one FAIL is a valid review, and reviewers must never
> invent findings to reach a count.

**This rule is why the 29 items were not copied.** A checklist that names artifacts in someone else's
repository cannot satisfy the rule it sits beside.

### 5.2 Carried over, 7 items

Project-independent in the reference implementation's own checklist, unchanged in intent.

| Item | Ceiling | What the reviewer must find in the diff |
|---|---|---|
| `secrets_check` | critical | A credential, token, key, or a literal path to one, entering source, tests or fixtures |
| `security_issues` | critical | A named unsafe construct: unchecked path join, shell interpolation of untrusted text, a widened `unsafe` block |
| `tests_affected` | critical | A behaviour change with no test touched, naming the changed function and the test file that should have moved |
| `self_consistency` | critical | The diff doing something its own commit message does not claim |
| `code_quality` | advisory | Duplication, dead code, a panic path on a recoverable error |
| `cross_platform` | advisory | Case sensitivity, path separators, an assumption that `HOME` looks like this machine's |
| `changelog_accuracy` | advisory | A user-visible change missing from the changelog entry |

### 5.3 Hydra's own, 23 items

Each names an artifact this repository will contain, and each is a mechanism some earlier decision
paid for. That is the point of writing them from this design rather than from theirs.

**Gates and authority, 7 items, all critical:**

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `approved_patch_unreachable` | critical | A new constructor, a widened visibility, or a test helper that builds an `ApprovedPatch` outside the review module |
| `severity_ceiling_table` | critical | An item's ceiling raised, an item losing its ceiling, or a path carrying a model-supplied severity through unclamped |
| `reviewer_failure_fails_closed` | critical | A parse failure, timeout or refusal resolving to anything other than reviewer-failed. `[]` inside refusal prose is the case that produced this item |
| `panel_size_unshrunk` | critical | Any code that yields a verdict with fewer than the configured judges answering |
| `protected_path_set` | critical | An entry removed or narrowed, matching made case-sensitive, or the refusal moved to after the review |
| `admission_gate_order` | critical | A judge dispatched before fmt, clippy, build and test have passed, or an admission failure that still spends review budget |
| `baseline_test_provenance` | critical | A protected test read from the patched worktree rather than from the baseline sha |

**The harness's own mechanisms, 8 items, all critical:**

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `env_allowlist_intact` | critical | A variable added to the allowlist without a named reason. `NODE_OPTIONS`, `PYTHONPATH` or `GEMINI_API_KEY` reaching a child at all |
| `hook_config_written_fresh` | critical | A hook config linked rather than copied, written once rather than per spawn, in the wrong per-provider location, or an `agy` spawn missing `--add-dir` |
| `hook_dialect_coverage` | critical | A gate script that answers in fewer than the two dialects it must, or that infers the dialect rather than taking it as an argument. `claude` fails open on a shape it does not recognise, so a missing branch is a silent hole rather than an error |
| `billed_spend_gate` | critical | A per-run human flag becoming inheritable, settable by the delegation policy, readable from the environment, or appearing on the MCP delegate face |
| `delegation_depth_cap` | critical | The depth cap removed or made unreachable, or the MCP face gaining `--shared-tree`, a timeout override, depth or `model` |
| `curator_write_surface` | critical | Rung one gaining a write path outside the skill store, above all to config, the policy or the allowlist |
| `evidence_is_read_only` | critical | Ouroboros gaining a call to `prune`, `reap` or `export`, or any write under `~/hydra/runs/`. For the three commands this item is the only mechanism, per section 2.2, so a reviewer may not defer to the gate here |
| `worktree_containment` | critical | A proposer path that reads or writes outside its own worktree, or a gate whose path matching can be escaped by a relative path or a symlink |

**The record contract, 5 items, 3 critical:**

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `record_append_only` | critical | A second concurrent writer to a run file, an in-place rewrite, an ownership transfer that skips the liveness test, or a run id that is not a UUIDv7 |
| `record_source_fields` | critical | `usage_source`, `outcome_source` or `permission_decision.decider` gaining an undocumented value or losing one |
| `raw_frame_preserved` | critical | A derived field overwriting the verbatim provider frame, or normalization dropping a frame it could not classify |
| `export_losslessness` | advisory | A record change that cannot survive the SFT and DPO export in D12, or redaction moving out of export |
| `provider_claim_dated` | advisory | A provider-specific assertion added to code or docs without the version and date it was observed |

**Scope and shape, 3 items, 2 critical:**

| Item | Ceiling | What the reviewer must find |
|---|---|---|
| `no_new_crossing` | critical | Anything crossing between the harness and Ouroboros other than the run record and the skill store: a shared module, a database, a socket (D5) |
| `no_daemon` | critical | A scheduler, background service, timer, or anything that must be running for the harness to work (D10) |
| `spec_matches_code` | advisory | A behaviour change that leaves `docs/harness-spec.md`, `docs/ouroboros-spec.md` or `docs/run-record-contract.md` promising the old behaviour |

### 5.4 Five items amended against the sibling documents

The items were written before the harness spec and the record contract existed, and five of them
name something those documents then settled differently. The amendments are above; the reasons are
here, so a reader comparing against the originating ticket does not read them as drift.

This heading said three and listed four, which is the same off-by-one section 5 opens by complaining
about. Counted from the paragraphs below.

**`record_source_fields` said `status_source`.** The contract names the field `outcome_source` and
drops its `provider` value, because no routable provider produces a run-level status worth reading.

**`record_append_only` said "a second writer to a run file".** Taken literally that forbids the
harvest that completes an abandoned interactive run. The contract's rule is at most one writer at a
time, with ownership transferring only after the liveness test says the recorded pid is gone.

**`hook_dialect_coverage` said three dialects.** Cursor accepts Claude's response dialect on
`preToolUse`, so the script answers in two shapes rather than three. The failure mode it guards is
unchanged and it is the reason the item is critical.

**`billed_spend_gate` was widened from `--allow-billed` to the pattern.** There are two per-run human
flags now, and the harness spec names the shape once so a third costs a table row. The item follows
it.

**`evidence_is_read_only` gained a sentence saying a reviewer may not defer to the gate.** The seam
check found section 2.2 claiming the gate enforced the three `hydra` commands on every run. It does
not, because they are reached through `shell` and an argv match is a floor rather than a gate, so
this item is the only thing holding them. A critical item that is the sole mechanism should say so,
or a reviewer reads a redundant check and marks it advisory in their head.

## 6. The seam: what Ouroboros reads

**Every field either rung reads is one the harness writes**, which is D5's first requirement. The
contract's producer column is what makes that checkable, and this table is the Ouroboros side of the
lookup.

**Rung one:**

| What it reads | Fields | Contract |
|---|---|---|
| Whether a run was abandoned | `pid`, `process_start_time`, the absence of `run_end` | 5.2, 6 |
| Whether a human corrected a tool call | `correction`, with `launch_mode` | 7.9, 5.1 |
| What the tools did | `tool_call`, `tool_result`, the adapter's classification | 7.6 |
| Whether a run retried | `parent_run_id` | 10 |
| Whether a clean-looking run failed | the derived outcome against the tool results | 8 |
| Which runs to skip | the derived outcome against section 3.10, and `provider: null` for the whole harness-run kind | 8, 8.6 |
| Which skills were in the run | `skills.store_sha`, `skills.linked`, `skills.resolved`, the last optional on `cursor-agent` and the group absent on an interactive run | 5.3, 5.6 |
| Which runs are its own | `parent_run_id` | 10 |
| Whether the gate leaked | `permission_decision`, and `subagent_stats.spawned` in `raw`. **Recorded runs only** | 7.7, 8.5, 5.6 |
| Whether it is thrashing | the store's git history joined to the outcomes above | 10, plus the skill store |

**Rung two:**

| What it reads | Fields | Contract |
|---|---|---|
| Who judged a patch | `provider_version` and `model` on each judge run | 5.1, 10 |
| What each judge said | the judge run's final assistant text | 17 |
| Whether to abort early | `rate_limit_event.status` and `resetsAt` in `raw` | 8.5 |
| Whether a judge could run at all | the derived outcome and `run_end.remediation` | 6, 8 |
| Which runs belong to one pass | `parent_run_id` against the pass's own harness run | 5.5, 10 |
| The corpus brief for the proposer | assembled by the supervisor from the rung-one fields above | |

**The gate-leak check is free and worth running.** Every tool call is evaluated against the run's own
policy at read time, so a call the provider allowed and the policy denies is a gate that leaked.
`claude`'s `subagent_stats.spawned` above zero under a policy denying in-process subagents is the
same signal directly. Neither reopens the decision to deny subagents, because a count is not a
trajectory. Both make the denial checkable, which it otherwise would not be.

**One field is read from `raw` on purpose in three rows above.** A normalized field that exists on one
provider is a shape that changes with the provider, which is what normalization exists to prevent, so
the contract names the paths and leaves them where they are.

**Rung one reads two populations and the seam check found it treating them as one.** A recorded run
carries a policy, so it carries permission decisions and a gate-leak check. An interactive run
carries neither and is the only source of a human correction (contract 5.6 and 7.2.1). So the failure
class that finds a leaked gate and the failure class that finds a human rejection can never fire on
the same run, and neither can be looked for in the other's population. Nothing in section 3.2's five
classes broke, because each of them already keyed on what it needed. What was missing was the reason
they never overlap, which a reader would otherwise discover by finding half the corpus silently
missing a field.

**The corpus is two shapes and the curator ranks across both.** That is fine for counting and wrong
for diagnosing: a signal absent from every interactive run is absent because the mode has no producer
rather than because nothing happened, which is section 3.4's honesty rule applied to launch mode
instead of to a provider.

## 7. What Ouroboros writes

Three things, and nothing else.

| Writes | Where | Merged by |
|---|---|---|
| Rung one's proposals | a branch in the skill store's own repo, `ouroboros/curate/<run-id>` | a human |
| Rung two's proposals | a branch in `~/hydra`, `ouroboros/<run-id>` or `ouroboros/blocked/<run-id>` | a human |
| Its own run records | `~/hydra/runs/`, written by the harness like any other run | nobody; they are the record |

The third row is the harness writing, not Ouroboros. Ouroboros' runs are runs, so they land in the
corpus, and the curator's own failures are learnable by the next curator pass, subject to the
exclusion in section 3.5.

**Nothing else crosses.** No shared module, no database, no socket, no scheduler. That is D5 and D10
together, and `no_new_crossing` and `no_daemon` are the checklist items that keep it true under a
self-edit.

## 8. Graduation conditions

Both rungs ship deliberately smaller than their designs. These are the conditions under which each
grows, written down so each is a decision someone makes rather than a default nobody revisits.

| Condition | Currently | Graduates when | Judged from |
|---|---|---|---|
| Rung one edits `origin = "ouroboros"` skills in place | proposes to a branch | enough proposals have been accepted with none bad | the store's git history and the run records |
| Rung one runs on a schedule | invoked | there are enough runs to tune a threshold against | the corpus |
| Rung two auto-lands an approved patch | proposes to a branch | enough proposals have been merged unchanged | the run records |
| Rung two gains an obligation ledger | one review, no memory | thrashing appears in the run records | section 3.8's measurement |
| Rung one gains a stall class | five classes | a stall signal exists on more than one provider | section 3.11 |

**No count is picked here and that is the point.** Hydra has run zero times, so every number in this
table would be invented rather than measured. Each row names the evidence its number comes from.

## 9. Open questions this spec does not close

Each of these is a ticket rather than a paragraph.

### 9.1 The curator's store worktree, closed

Section 3.6 gave the curator its own git worktree of the skill store rather than letting it write
into `~/hydra/skills/`, and no closed ticket said so. Ticket 40 closed it on 2026-09-06 and 3.6 won:
harness spec 7.2 had been claiming the opposite for as long as both documents existed.

The open detail was where that worktree lives, and it is `~/hydra/skill-worktrees/<run-id>/`, a
sibling of `worktrees/` rather than a place inside the spawn directory. The spawn directory is the
one `hydra reap` removes whole, and an uncommitted proposal is work. `hydra reap` now sweeps a
leftover checkout too, committing to the branch first, so a proposal survives on
`ouroboros/curate/<run-id>` rather than being thrown away.

The store's working tree gained a stronger guarantee on the way. It is one of the paths harness spec
7.5 denies under every policy, so no gated run writes into it by any route, the way no gated run
writes into `~/hydra/runs/`. The record and the store are the two things that cross D5's seam and
now both are protected.

### 9.2 The proposer's policy has no name

Rung two's proposer needs reads and writes inside its own worktree and a shell for `cargo`, which is
`standard` as the five policies stand. Nothing says whether it should be `standard` or a sixth name.
A sixth policy should arrive with a reason, and "it is the proposer" is not one until something
`standard` allows turns out to matter here.

The read-only brief in section 4.3 is the part that would justify a name, because assembling a brief
is not a policy at all under the current shape: it is the supervisor choosing what to put in a prompt.
Whether that stays a convention or becomes a mechanism is unsettled.

Closed by map ticket 52 on 2026-09-08: the proposer runs as `standard`, and the brief stays a
convention the supervisor keeps. A sixth policy arrives with the first reason.

### 9.3 Nobody has fitted a review prompt

The reference implementation injects whole governance documents and budgets 920K tokens per reviewer.
Hydra's answer is probably the diff plus the checklist, and the machinery for deciding what to drop
when a diff is large is real work that nothing here has scoped: the fit note, the path manifest
fallback, the representation of binary files.

Related, and worth reading before it bites: their reviewer context-window resolution carries two
production failures in its own docstring. A hardcoded `context_window=1_000_000` sized a 200K route
as 1M and drew a provider 400, losing that reviewer's entire review. And a process-lifetime memo that
outlived a 24-hour evidence TTL made a healthy install read its own scope reviewer as expired
forever, blocking every commit for the rest of that process's life. The design answer they landed on
transfers whatever Hydra builds: sizing may be optimistic, blocking authority may not.

Closed by map ticket 52 on 2026-09-08: nothing is fitted. The proposer's patch has a line cap,
a patch over it is recorded as rejected for size before any reviewer runs, and a reviewer sees
the whole diff or none. The fit note, the manifest and the binary representation are not built.

### 9.4 Tolerant parsing is thirty lines nobody has written

Telling a refusal from a clean pass is the one genuinely hard piece of rung two that Rust does not
make easier. The reference implementation's fix is a whole-response match with one optional code
fence and an optional sentinel. Section 2.1 chooses a type over a sentinel, because a sentinel is
something the model must remember and a type is something it cannot get wrong, but the parse in front
of that type is still the thirty lines you do not think to write until it bites you. When it bites,
the failure is a self-edit that was never reviewed being recorded as reviewed.

Closed by map ticket 52 on 2026-09-08: the reviewer answers one closed JSON shape, parsed whole,
and a response that does not parse is a block carrying the parse error. No sentinel.
