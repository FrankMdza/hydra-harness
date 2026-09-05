# 36 - What shape is a run record with no provider?

Type: grilling
Status: resolved
Blocked by:

## Question

`docs/ouroboros-spec.md` section 4.10 needs a run record for a rung-two pass, and the record
contract has no shape for one.

A pass is `hydra evolve`: a proposer child, a protected-path check, the admission gate, three judge
children, one branch. The proposer and the judges are ordinary `hydra run` records. The pass itself
is not: it has no provider, no vendor frames and no body.

It needs a record anyway, and the Ouroboros spec gives three reasons. The protected-path refusal,
the admission gate output and the panel's aggregate verdict are all decided after the proposer's run
file was closed, and under D10 anything not in a run record did not happen. The contract's own
sentence "who reviewed a patch is a tree of three judge runs under one parent" needs that parent to
exist. And the two alternatives are worse: parenting the judges to the proposer claims the proposer
spawned its own reviewers, and a pass file of its own gives the harness a second source of truth.

Settle:

- Whether a provider-less run record is a run record, or a second kind of thing the contract names
  separately
- What `launch_mode` says on one. It is neither `recorded` nor `interactive`
- What `provider`, `provider_version` and `model` say. Null, or a Hydra sentinel
- Whether `pid` is the Hydra process. The contract reads a null `pid` as a run that never had a
  process, and this one has a real one that is not a child
- Which outcomes a pass can end on, and whether `run_end.refusal` stretches to cover a
  protected-path refusal, which is a fifth Hydra-side check beside the four it names
- Whether `hydra curate` needs the same thing. Rung one is a single provider run today, so it does
  not, unless a curator pass grows a step the supervisor decides

The refused-spawn record in contract section 8.3 is the precedent to argue from: a record with
`run_start`, `run_end` and nothing between, written so an attempt is not invisible. Decide whether
that is a special case or the first instance of a general shape.

If the answer is that the contract should not carry these, the fallback is on the Ouroboros side and
this ticket says what it is.

## Why it blocks ticket 16

The seam check is the wrong place to discover that two documents disagree about what a run is. Both
documents are written; this is the one place they do not meet.

## Answer

**A provider-less pass is a run record, and the word "run" widens to hold it.** A run is a unit of
recorded work Hydra owns, and there are two kinds: a **provider run**, which is one provider process
Hydra launched or refused to launch, and a **harness run**, which is work Hydra did itself. Today
there is exactly one producer of the second kind, a `hydra evolve` pass. `provider: null` is the
discriminator.

The contract's own definition already had two disjuncts and the second one, the refused spawn, has no
process behind it. So this is a third disjunct on a definition that was never "a provider process",
rather than a new concept smuggled into one.

### Why it is a record at all

**Because its children need a parent that exists.** D10 is the frame and Ctrl-C is the argument. A
pass spawns a proposer and three judges; without a pass record they are four unrelated runs at depth
0, and a human who interrupts `hydra evolve` leaves detached children that nothing can group. The
contract's "who reviewed a patch is a tree of three judge runs under one parent" also needs that
parent to be something.

Both alternatives are worse. Parenting the judges to the proposer claims the proposer spawned its own
reviewers, which is a lie in the field the depth cap and the retry-loop derivation both read. A pass
file outside `runs/` gives the harness a second source of truth, which is the thing D10 exists to
prevent.

### The narrow test

**A harness run exists for something that has children.** `hydra export`, `hydra reap` and
`hydra prune` all decide things and none of them writes a record, because none of them spawns a run
that would otherwise be orphaned. A second population inside `runs/` is a filter every corpus query
has to learn, so it is bought once, for something unobtainable another way.

That test also answers rung one without a special case: **`hydra curate` gets no harness run**,
because a curate pass spawns nothing. The curator's only post-run supervisor action is committing
work the run already did, and a commit is self-describing, where a panel verdict is a new fact about
a run that has ended.

### The shape

Seven fields of twenty-six are filled: `command`, `cwd`, `depth`, `pid`, `process_start_time`,
`schema_version`, `hydra_version`. **That number goes in the contract rather than being left for a
reader to discover**, because it is the strongest argument anyone will ever make for reversing this,
and it should be in front of whoever makes it.

- **Null the scalars, drop the groups.** `provider`, `provider_version`, `model`,
  `provider_session_id`, `launch_mode`, `task`, `policy`, `policy_sha`, `worktree`, `subscription`
  and `run_end.body_source` are null. `skills`, `mcp`, `auth` and `env_allowlist` are absent. A null
  `skills.store_sha` invites a reader to ask which sha the pass pinned; an absent `skills` object
  says the question does not apply.
- **No sentinel.** `provider: "hydra"` would put a fake provider in the field the corpus groups by,
  and every runs-per-provider query would grow a fourth bar that is not a provider.
- **`pid` is the Hydra process the human invoked.** This is the one field a harness run stretches
  rather than empties, and it is the stretch the record was bought for: liveness then answers whether
  the pass is running. A pass killed with Ctrl-C is a dead pid with no `run_end`, and `hydra status`
  is its reader.
- **No policy, because nothing gates a harness run.** The gate applies to runs Hydra launches and a
  pass is Hydra. Its children carry their own policies and narrow against the human at depth 0.
- **The body is two lines**, `run_start` and `run_end`. The contract's "exactly two Hydra lines"
  survives without an exception, and no third line type is minted.

### A new field: `command`

**Nothing in the record said which command created a run**, and on a harness run that is the whole
identity. `command` goes on `run_start` for every run, with four values today: `run`, `evolve`,
`resume`, `curate`.

The alternative was reusing `task`, which would make one field a prompt on one row and a command name
on another, in the field export reads as a prompt. Deriving the identity from the children was the
third option and it is a fingerprint rather than a fact: three judge children stops being true the
moment a pass dies before spawning them.

A curator run reads `command: curate` even though a curate pass gets no record, so the asymmetry
above is visible in the data rather than only in a document.

### The outcome describes the pass, not the patch

**A pass that refused a patch did its job, so it is `completed`.** So is a pass whose panel blocked
and a pass whose admission gate caught a build failure. `failed` means the pass itself broke, git or
the worktree. `auth_failed` and `rate_limited` carry over unchanged, since a judge that cannot run
aborts the review. `gated` never applies. No new outcome, and `refusal` keeps meaning that Hydra
would not start something rather than stretching to a fifth check.

**So ticket 15's "the run ends `failed` carrying the compiler output" does not survive.** Section 8's
enum is long because providers report success on runs that did no work; filing a pass that worked
perfectly as a failure is that same mistake pointed the other way, and rung one would eventually read
it as one.

**The compiler output moves to the branch commit message, beside the verdict.** Both are results
about the patch, and splitting them by which stage produced them would be arbitrary. That leaves the
pass record holding no verdict at all, which is the point: one copy, on the artifact the pass
produced.

### Who reads a harness run

**Rung one skips the kind**, keyed on `provider: null` rather than on an outcome. A pass tried
nothing, so it teaches nothing, and leaving it in would turn every interrupted evolve into an
abandoned-run signal about nothing. Its children stay in the corpus as ordinary provider runs, which
is where the proposer's trajectory is worth something.

**Export skips the kind too**, as a fourth exclusion beside the three in the contract's section 16.2,
keyed on kind rather than outcome, because a trajectory with no messages exports as an empty line.

**Judge runs export like any other run**, with no rule of their own. The diff in a judge's prompt is
Hydra's own source, so the export allowlist already decides whether it may leave. A rule excluding
judges by role would be the first time export judged a run by what it was for rather than by where it
touched.

### The cost, stated

**The proposer and the judges move from depth 0 to depth 1**, so the cap at 2 leaves the proposer one
level of delegation rather than two. Accepted: a proposer needing two levels to write a patch is
doing something worth noticing, and the cap is a configurable number someone raises with a reason.
Exempting harness runs from the depth count was the alternative, and it would make depth mean two
things.

### What this changes elsewhere

- **`docs/run-record-contract.md`**: section 2 widens the definition and states the has-children
  test, section 4 admits a null `provider`, section 5.1 gains `command`, **section 5.5 is new** and
  holds the harness run, section 6 nulls `body_source`, **section 8.4 is new** on the outcome and the
  old 8.4 becomes 8.5, **section 8.6 is new** on who reads one, section 10 names the pass as the
  parent and states the depth cost, section 14 records that a verdict is deliberately not in the
  record, and section 16.2 gains the kind exclusion.
- **`docs/ouroboros-spec.md`**: sections 4.5, 4.9 and 4.10 rewritten, section 6's tables amended, and
  the open question this ticket was filed from is deleted rather than answered in place.
- **Ticket 15** loses "the run ends `failed` carrying the compiler output".
- **Ticket 17's** apply-records-the-verdict requirement is met by the commit message.
- **Ticket 11** gains the depth consequence for rung two's children.
- **Ticket 09** gains `command`.
- **Ticket 16** is unblocked and the seam check can run.
