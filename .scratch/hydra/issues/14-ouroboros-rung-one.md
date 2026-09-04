# 14 - Ouroboros rung one: watching failures, evolving skills

Type: grilling
Status: resolved
Blocked by: 05, 06, 09

## Question

D11 ships rung one first: Ouroboros edits skills, prompts and config, never Hydra's own source. This ticket designs it.

Settle:

- What counts as a failure worth learning from, read out of the run records. Repeated tool errors, a human rejecting a diff, a retry loop, a session abandoned
- How many occurrences before it acts, and over what window. The paper's phrase is durable error classes; decide what durable means in numbers here
- What it proposes: a new skill, an edit to one, a prompt change, a config change
- Who reviews a proposal, and whether a human approves before it lands, or only sees it afterwards
- Which guardrails from ticket 05 apply verbatim, and which need changing for Hydra
- Which model does the reviewing, and on which auth plane it runs
- How to tell whether a change helped, which means deciding now what evidence a later comparison would need

Scenario: the same skill gets rewritten three weeks running and each version is worse than the last. What catches that?

## What the research changed

**Ticket 05 supplies the rule list**, fifteen of them, each with the reason behind it. Read the findings file rather than restating them here.

Its headline finding is a missing guard in Hermes, and it reframes this whole ticket. The curator's reviewing model is told in its prompt that archiving moves a directory into `.archive/` and is recoverable. The tool it actually calls runs `shutil.rmtree`. So per-skill recovery does not exist for anything the model consolidates or prunes, and the only safety net is the pre-run snapshot. Dry-run's no-mutation promise rests on prompt text with no tool whitelist.

The rule that follows: **a guarantee written in a prompt is not a guarantee.** Every limit Hydra's curator claims has to be enforced in code, at the tool-dispatch layer, or it does not exist. Design this ticket's guardrails on that basis.

**Ticket 06 supplies the number to design against.** Ouroboros blocks 63.5% of recent self-edits. Two of every three proposals get rejected. Any design that treats a proposal as usually landing is wrong, and the rejection path is the common path.

## What the evidence pass found, 2026-09-01

Provider-against-signal table and method in
[research/14-failure-signals.md](../research/14-failure-signals.md), 607 lines.

- **Three of four providers report failure at the tool level and success at the run level, in the
  same run.** `claude` closed `terminal_reason: "completed", is_error: false` with a real denial
  sitting in `permission_denials`. `agy` returned `status: "SUCCESS"`, empty response, exit 0 on a
  run whose only tool was auto-denied. `cursor-agent` returned `is_error: false` with one tool
  rejected and one failed. Anything keyed off the terminator files a failed run as clean.
- **Two of this ticket's four failure classes are not derivable at all.** A session abandoned has no
  pid, no exit code, no heartbeat and nothing written at exit; a Cursor run killed at 6 seconds
  leaves zero bytes, indistinguishable from one that never started. A human rejecting a diff has no
  producer: `claude` labels a headless auto-deny `user-rejected` exactly like a real one, and ticket
  09's `correction` event has no producer anywhere.
- **Repeated tool errors are derivable but not comparable across providers.** `agy` gives a
  permission denial and a file-not-found the same `state: "ERROR"` and `error.type: "TOOL_ERROR"`,
  and both messages contain the word "permission", so the obvious substring rule is wrong rather
  than merely weak.
- **Ticket 09 rests on an expired fact.** It gave Cursor `usage_source: estimated` because Cursor
  emitted no token counts. The 2026.08.31 build emits `usage` with four token fields, and emits
  `thinking` in print mode where ticket 09 says it never does. Settle that before ticket 16 writes
  the record contract.
- **For ticket 16's seam check:** ticket 11 puts `exit_code`, `status`, `status_source` and `error`
  in the JSON `hydra run` returns, and ticket 09 puts none of them in the run file.

## What ticket 12 settled

**Auth failure is a terminal outcome distinct from task failure, and rung one must never learn from
it.** A skill does not need evolving because a token expired. The run ends carrying a remediation
string that names the provider and the command to re-authenticate.

The signature taxonomy is this ticket's to own. The failure-signals research names truncation, rate
limit and quota classes and no auth class at all, so that slot is empty and needs filling here.

## What ticket 13 added

**Some records arrive late and in a different shape.** Interactive runs cannot stream, because a
vendor CLI keeping its own TUI cannot also speak ACP. They are harvested from the vendor's own
transcript when the run exits, and for `agy` and `cursor-agent` that transcript is a sqlite database
or a chat directory rather than an append-only log. So rung one reads a corpus of mixed provenance,
and a run that looks empty may simply not have been harvested yet.

**Ouroboros cannot call `hydra prune` or `hydra reap` at either rung.** Cleanup stays a human
command, so nothing the loop does can destroy the evidence it learns from.

## What ticket 17 hands you, 2026-09-03

Two rules cross the rung boundary, and they do so because they are ticket 05's finding rather than
ticket 17's. Both apply to the curator, because a curator is a gate.

- **A model never sets what blocks.** Where a curator decision has severity, Hydra sets the ceiling
  and the model reports under it.
- **A model's silence is not consent.** A response that does not parse into the expected shape is
  its own outcome, and it fails closed. In the reference implementation `[]` inside refusal prose
  entered a quorum as an approval, which is the concrete version of this.

Nothing else from ticket 17 belongs here. Worktrees, the drift proof, protected paths and the quorum
are rung two's.

**A watch condition for this ticket to design for.** Rung two ships with no obligation ledger and no
memory across retries, on the grounds that being argued down over many rounds is a problem you only
get once there are rounds. Thrashing appearing in the run records is the signal that the ledger
needs building, so whatever failure signals this ticket settles should be able to see it.

## Answer

**Rung one is a thing you invoke, it writes only to the skill store, and nothing it produces lands
without a human merging it.** Every number the ticket asked for is deferred on purpose, because
Hydra has run zero times and a threshold invented now is a guess wearing a decimal point.

### What it learns from

Five classes, and the fifth is not in the ticket's list.

| Class | Derived from |
|---|---|
| Abandoned run | a dead pid with no `run_end` |
| Human rejection | `user-rejected` in an **interactive** run only |
| Repeated tool errors | the adapter's classification, see below |
| Retry loop | the `parent_run_id` tree |
| **Clean-looking failed run** | terminator says success, tool errors sit inside it |

Two of these were recorded as underivable in the evidence pass above, and ticket 13 closed after
that was written. Both are alive now.

**Abandoned runs.** The research was reading the vendor's transcript, where a Cursor run killed at
six seconds leaves zero bytes. Ticket 13 puts pid and process start time in Hydra's own run
envelope, written at launch, so Hydra always has the envelope even when the child wrote nothing.

**Human rejection, by subtraction.** `claude` labels a headless auto-deny `user-rejected` exactly
like a real one, which is why it looked unproducible. But ticket 11 has Hydra's own policy answering
permissions in recorded runs, so a recorded run has no human in it to reject anything. Interactive
plus `user-rejected` is a person. Recorded plus `user-rejected` is Hydra's policy, and Hydra knows
what it said.

**The fifth class earns its place because it makes the other four unreliable.** Three of four
providers close a run `is_error: false` with a real denial or a failed tool inside it, so anything
keyed off the terminator files a failure as a success. Ticket 26 added a sharper instance: all three
providers exit 0 and report success on a run where the hook gate refused everything.

**Auth failure teaches rung one nothing**, per ticket 12. A skill does not need evolving because a
token expired.

### Classification, and the honesty rule

`agy` gives a permission denial and a file-not-found the same `state: "ERROR"` and the same
`error.type: "TOOL_ERROR"`, and both messages contain the word "permission". So a substring rule is
wrong rather than weak.

**The per-provider adapter classifies, and it may return `unclassified`.** Read-time normalization
sounds right until you notice it needs one rule fitting three providers, which is exactly what does
not exist. An adapter that says it cannot tell a denial from a missing file on this provider gives
the curator something honest to count and keeps the gap visible per provider rather than averaged
away. Guessing here produces a curator that rewrites a skill because a file was missing.

### Where the curator runs

**It is a `hydra run` like any other**: recorded, delegated through ticket 11's contract, with a
policy permitting writes to the skill store and nothing else. That dissolves the ticket's question
about which model and which plane. The provider becomes a config value, its cost is measured the way
every run is, and its own failures land in the corpus.

One filter comes with that and must be written down rather than discovered: **runs whose parent is
the curator are excluded from the corpus the curator reads.** Otherwise it learns from itself.

### When it runs, and what that buys

**Invoked, not scheduled.** You type `hydra curate`. It reads the whole corpus, ranks by frequency,
and proposes. There is no occurrence threshold and no window in v1.

The ticket asked for durable in numbers and the honest answer is that the number does not exist yet.
Q4's decision also removes the reason to have one: if nothing lands unseen, a threshold is noise
control on a list a human reads, not a safety mechanism. The scheduled version goes in the spec as
step two, gated on having enough runs to tune against. Ticket 05's "defer the first-ever run by a
full interval" costs nothing here, because there is no interval to defer.

### What it may write, and what it may never write

**The skill store, and nothing else.**

D11 says rung one edits skills, prompts and config. That was written before anyone knew what Hydra's
config was. It is the delegation policy from ticket 11 and the environment allowlist from ticket 21,
and **a self-improvement loop that can edit its own permissions walks around every gate on this
map**, including the apply-path type ticket 17 spent an answer building. Prompts are struck for a
duller reason: Hydra injects skills and has no separate prompt, so the word points at nothing.

D11 is amended accordingly. This is the hardest line in the answer and it should read as one.

### Nothing lands unseen

Ticket 10 makes a skill with `origin = "ouroboros"` editable in place. **In v1 it is not.** Every
change is a proposal: the curator commits to a branch in the store and a human merges it or does
not. The store's git history is the queue.

D11 ships rung one first so you can find out whether it is any good, and a loop that edits silently
gives you nothing to judge. This is a **graduation condition, not a permanent rule**: the default for
`origin = "ouroboros"` flips once enough proposals have been accepted with none bad. Consistent with
the reasoning above, the count is not picked now. It is picked from the corpus, and the spec says so
explicitly, so the flip is a decision someone makes rather than a default nobody revisits.

### Whether a change helped

**Measure only the signal the change was made for.** A proposal must name the class and the skill it
responds to, and that name becomes the measurement. If it proposed an edit because unclassified tool
errors on skill Y fired seven times, the measure is that count afterwards.

Narrow on purpose. A general before-and-after needs comparable work and Hydra has no notion of task
similarity; A/B across store shas needs repeated tasks, which real work does not provide. This one
needs neither.

**It answers the ticket's scenario.** The same skill rewritten three weeks running, each version
worse, appears as three consecutive proposals against one skill for one signal that never drops.
That is also the thrashing ticket 17 asked rung one to be able to see, so rung two's missing
obligation ledger has a detector.

**Input to ticket 09:** record the resolved skill listing the provider reported, not only what Hydra
linked. Ticket 24 proved `store_sha` plus `linked` does not reproduce what a run's model saw, because
Claude Code truncates its listing to a character budget and silently strips descriptions. `agy` and
`claude` both expose their resolved listing for free.

### Ticket 05's rules

Eleven apply verbatim. Four change:

- **The pin's patch exemption is dropped**, by ticket 17. A guarantee with an exception you can argue
  into is not one.
- **A failed snapshot blocks rather than warns.** With ticket 10's git store, a failed snapshot means
  git failed, which is not a condition to proceed through.
- **Deferring the first run by a full interval is moot**, since there is no interval.
- **Archive versus delete changes character.** Ticket 13 forbids Ouroboros calling prune or reap, and
  a git store makes any deletion a recoverable commit, so Hermes' `shutil.rmtree` bug cannot exist
  here. `absorbed_into` and validating a merge target survive as commit-message discipline rather
  than as safety.

The one to keep loudest, of the eleven: **deterministic transitions run before the model pass and
independently of it.** Ticket 26 found `claude` fails open on a hook response in a dialect it does
not recognise, which is what a gate looks like when it trusts a model's output shape. The curator
being something you invoke will make it tempting to let one model call decide everything.

Ticket 17's two crossover rules hold here unchanged: a model never sets what blocks, and a model's
silence is not consent.

### What this changes elsewhere

- **D11** is amended. Rung one's write surface is the skill store, not skills, prompts and config.
- **Ticket 10** is narrowed. `origin = "ouroboros"` is not editable in place in v1.
- **Ticket 09** gains the resolved provider listing.
- **Ticket 05** has four rules amended, listed above.
- **Ticket 15** is unblocked and inherits a rung one that proposes rather than lands, plus a
  thrashing detector it can rely on.

## Amended by ticket 19, 2026-09-03

**A third protected config.** This ticket named the delegation policy and the environment allowlist
as the config rung one may never write, on the grounds that a loop able to widen its own permissions
walks around every gate on this map. Ticket 19 adds the export allowlist, the file deciding which
repos a run may have touched and still be exportable. It is the only one of the three whose failure
mode is data leaving the laptop rather than a bad action taken on it, so it is the one where a
widening would be hardest to notice.

**The curator exclusion now has a second consumer.** This ticket excluded curator-parented runs from
the corpus the curator reads. Ticket 19 excludes them from the training corpus too, for the same
reason, so the rule is worth stating once in the Ouroboros spec rather than twice.

## Amended by ticket 27, 2026-09-03

**The config rule gains a run-time half.** Barring rung one from config was a restriction on its
write surface, enforced by what the curator is allowed to touch. Ticket 27's gate now enforces it
against every run rather than only the curator's: the hook denies writes to the policy directory and
to the generated gate script under every named policy, `permissive` included. The curator runs under
a policy called `curate` that reads the corpus and writes only inside the skill store.

**A gated run is its own outcome and teaches this ticket nothing.** All three providers exit 0 and
report success when the gate refuses everything, which is this ticket's fifth failure class again.
The `gated` outcome exists so the curator does not rewrite a skill because a tool was denied. Same
shape as the auth rule already here: a run that failed on too narrow a policy is a policy bug.

## Amended by ticket 29, 2026-09-03

**A fifth protected config.** Rung one may never write the delegation policy, the environment
allowlist, the export allowlist, ticket 28's instruction preamble, or ticket 29's canonical MCP
document. The document is the file that decides which MCP servers a run gets, and an MCP server
definition is a command Hydra spawns, so a loop able to edit it runs arbitrary code inside every
future run. That is this ticket's own argument for the first two, unchanged.
