# 16 - Assemble the three documents and check the seam

Type: task
Status: resolved
Blocked by: 09, 10, 11, 12, 13, 14, 15, 19, 20, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36

## Question

The destination. Write the three documents from what the closed tickets decided.

1. `docs/harness-spec.md`
2. `docs/ouroboros-spec.md`
3. `docs/run-record-contract.md`

Then check the seam, which is the part that only shows up once all three exist side by side:

- Every field Ouroboros reads is one the harness actually writes
- Every guarantee the harness claims is one Ouroboros relies on, and no more
- Nothing crosses the line except the run record and the skill store, per D5
- The record survives the export test in D12 without loss

Done when someone who was in none of these sessions can build from these three files. If a gap shows up here, it is a new ticket, not a paragraph of hand-waving.

## Content already written, 2026-09-03

**The 29 review checklist items are done**, in ticket 15's answer: 7 carried from ticket 17's survey
of the reference implementation, 22 written against this map. Each carries a severity ceiling and
names an artifact this repo will contain. Lift them into `docs/ouroboros-spec.md` rather than
rewriting them, and check the seam the usual way: every item that names a harness mechanism should
name one the harness spec actually describes.

## The export contract, 2026-09-03

Ticket 19 wrote it. This ticket never listed 19 as a blocker and should have, since the seam check
"the record survives the export test in D12 without loss" has no meaning until the export exists.
Read ticket 19's answer before writing `docs/run-record-contract.md`.

Three of its decisions land in the harness spec rather than the contract: `hydra export` is a command
only a human types, the export allowlist is Hydra's third protected config beside the delegation
policy and the environment allowlist, and eligibility is computed over a run's whole subtree. One
seam to check specifically: ticket 19's eligibility check reads paths through the tool-call adapter
that ticket 09 left open, so whatever the harness spec says that adapter does has to be enough to
answer "every path this run touched", and it has to be allowed to say it cannot.

## Wiring fixed, 2026-09-03

Tickets 19, 20, 29 and 30 were missing from `Blocked by:` and are now listed. Ticket 19 wrote the
export contract this ticket has to check D12 against. Ticket 20 decides how a run says which
subscription paid for it, which is a record field. Ticket 29 decides where Claude's MCP config goes,
which the harness spec has to state. Ticket 30 can invert ticket 27's composition decision, which
would change the harness spec rather than a paragraph of it.

## What tickets 28 and 30 hand this ticket, 2026-09-03

**A paragraph for `~/hydra/CLAUDE.md`.** Ticket 28 found three populations rather than two: a
recorded run Hydra launches, an interactive run it launches, and an agent working on this repo. Only
the first is gated. The third's home is `~/hydra/CLAUDE.md`, which is not one of the three documents,
so nothing else places it. The wording must not let a reader assume the third population is gated on
the strength of sharing the first's phrasing.

**Two protected lists to reconcile.** Ticket 27 counts three protected configs, ticket 15's
protected-path list holds seven paths, and ticket 19's export allowlist is in the first and not the
second. Ticket 28's preamble config takes them to four and eight. Fix it once, here, rather than in
two documents.

**One seam to check specifically.** Ticket 28 promises that a run's actions are bounded by its own
policy, and that promise rests on ticket 30's measurement that a hook cannot grant on any provider.
Ticket 31 settles which of the user's settings run at all on `claude`. The harness spec states the promise once and
the command line that makes it true.

## What ticket 31 hands this ticket, 2026-09-03

**The floor's initial patterns.** Ticket 31 puts argument conditions on the `shell` capability
covering the destructive git forms, `aws configure` and `sso`, and Slack webhooks. The exact patterns
are spec work here. Write them knowing the floor is documented as catchable by spelling, so the
harness spec must not imply coverage it does not have.

**The exact `claude` command line.** Hydra composes with the user's settings, so `--restricted`
appears for `judge` alone and `--setting-sources project` appears nowhere. Say why the second was
measured and rejected, or a later reader finds it in ticket 30 and assumes it was an oversight.

**One pattern to name once.** A flag typed per run, never inheritable, unsettable by the delegation
policy. D16's `--allow-billed` and ticket 31's floor escape are both instances, so the spec describes
the shape and then refers to it.

## Amended by ticket 29, 2026-09-03

**The two lists to reconcile are now five configs and nine paths.** Ticket 28 took them to four and
eight, ticket 29's canonical MCP document takes them to five and nine, and ticket 15's list still
does not carry ticket 19's export allowlist. Fix it once here, as this ticket already says.

**The generated per-run gate script has a path.** Ticket 27 wrote it outside the worktree it guards
and named no directory. Ticket 29 gives it one, `~/hydra/spawn/<run-id>/`, shared with the
`--mcp-config` file for `claude` and the `CURSOR_DATA_DIR` root for Cursor, and reaped with the
worktree by ticket 11's cleanup. The harness spec needs one table of what Hydra writes per run and
where, because the answer is now three places: the worktree, the skills snapshot, and the spawn
directory.

**One asymmetry to state on purpose.** Claude's MCP config goes on the command line while its skills
are injected into the worktree, so a reader who expects the two to match will go looking for a bug.
The spec says why, which is that `--strict-mcp-config` makes a run's server set an argument Hydra
wrote rather than the state of five directories at spawn time.

**A second seam to check.** Ticket 29's launch check compares the servers a run's policy admits
against the servers the launch can deliver. The policy set lives in the harness spec and the
admission field lives in the MCP document, so the promise that a run never comes up short silently is
made in one document and kept in another.

## Progress: document one written, 2026-09-04

**`docs/harness-spec.md` exists, 1,192 lines.** One of the three. Written in a session that took this
ticket one document at a time rather than whole, because three documents at the fidelity this map has
been working at do not fit one session and the seam check should not be run by the session that wrote
the last document.

Order for the remaining sessions: `docs/run-record-contract.md`, then `docs/ouroboros-spec.md`, then
the seam check alone.

### What the harness spec covers

Sections 1 to 18 are the settled material: the routable set and the version drift rule, the two
launch modes, ticket 28's three populations, the per-run write table, the launch sequence with the
environment allowlist and the Hydra-set variables as separate categories, skills, MCP, the gate, the
exact command lines, the policy engine with the pattern floor's initial patterns spelled out,
instructions and data, `hydra run` and the delegate face, lifecycle and harvest, auth, the outcome
table, the tool-call adapter, `hydra export`, the reconciled protected lists, the per-run human flag
pattern, the command surface, and the non-goals.

Section 19 holds what it could not close.

### One gap, filed as ticket 32

**The recorded transport is undecided.** Ticket 01's "drive ACP, not print mode" rested on a
four-provider set: `gemini` and `hermes` were two of the four and both left the routable set,
ticket 18 retired ACP for Google, and `cursor-agent` never had it. ACP now covers `claude` alone and
no ticket says whether Hydra builds a client for one provider. Section 6.7 carries a `<transport>`
placeholder in the `claude` command lines until ticket 32 closes. This ticket now lists 32 as a
blocker.

### Three things decided here rather than ticketed

**The protected paths are eleven, not nine.** Every ticket that states a number says nine. The items
those tickets name come to eleven: the running count was carried from seven through ticket 28's
preamble and ticket 29's MCP document without ever folding in ticket 19's export allowlist or either
of ticket 27's two additions. The correction is arithmetic rather than a decision. Ticket 15 is
amended and `docs/harness-spec.md` section 15.2 carries the reconciled table. The configs are five,
which was right.

**"Protected" was carrying two mechanisms under one word.** Ticket 15's is refusal at admission for a
rung-two patch. Ticket 27's is a run-time hook deny on every run. The policy directory and the
generated gate script sit on both under the same word, so section 15.2's table has a column for each.
The generated gate script is listed under protest: it is written per run into
`~/hydra/spawn/<run-id>/` and reaped with the worktree, so no rung-two patch can reach it, and what a
patch can reach is the code that generates it, which is a separate entry. Its run-time denial is real
and is why it stays listed.

**Three spawn-directory artifacts had no filenames.** No closed ticket names any of them. The spec
uses `settings.json`, `mcp.json` and `gate`, and puts Cursor's data root in a `cursor-data/`
subdirectory rather than at the top of the spawn directory. That subdirectory is a refinement of
ticket 29 and is flagged in section 19.2: under ticket 29's literal reading the vendor writes
`projects/<slug>/` into the same directory that holds the gate script, which weakens "the script
lives where nothing else writes" even though the gate's own rule still denies writes to the script
path.

### Two things for the record-contract session

**`denied` against `gated`.** Ticket 11's status enum carries `denied`; ticket 27 later minted a
`gated` run outcome for the same situation. The harness spec's outcome table lists `gated` and drops
`denied`, which is the reading that makes the taxonomy non-overlapping, but the enum belongs to the
record contract and should be settled there rather than inherited from a sibling document.

**Read ticket 09's amendments rather than its answer.** Ticket 25 falsified two of ticket 09's Cursor
facts in the answer text itself: Cursor is `usage_source: provider` rather than `estimated`, decided
per run rather than per provider, and the `thinking` sentence is wrong in both directions. Ticket 09
says this itself.

### One thing for the Ouroboros session

**Ticket 15's checklist counts do not match what ticket 15 lists.** Its heading says 29 items and the
tables hold 30. It says "Hydra's own, 22 items" and the four tables hold 23 rows. It says "Fifteen of
the 22 are critical" and 20 of the 23 carry `critical`, with only three advisory
(`export_losslessness`, `provider_claim_dated`, `spec_matches_code`). This ticket's own note above
says "The 29 review checklist items are done", so the wrong number walks into
`docs/ouroboros-spec.md` unless someone recounts. Ticket 15 is amended with the corrected counts.

## What ticket 32 hands this ticket, 2026-09-04

**The transport hole is closed and section 6.7 is written.** Every recorded run is print mode with
`--output-format stream-json`, because no routable provider speaks ACP. The `<transport>` placeholder
is gone, `--verbose` turned out to be mandatory, and four `claude` flags no ticket had considered are
decided: `--include-hook-events` on, `--forward-subagent-text` on, `--include-partial-messages` off,
`--input-format stream-json` specced and unbuilt.

**The producer column is this ticket's main instrument.** Ticket 32 puts a producer column on every
event type in the record contract, so the first seam check, "every field Ouroboros reads is one the
harness actually writes", becomes a table lookup rather than a re-read of thirty tickets. Build that
column while writing `docs/run-record-contract.md` rather than deriving it during the seam pass.

**The event list is ten, not eleven.** `permission_request` is dropped and `message_delta` is dark in
v1, which the producer column has to say out loud.

**Two open items were added to the harness spec rather than ticketed**, sections 19.4 and 19.5:
nothing reads the policy signal that gated runs carry, and there is no stall detector. Both are on
the map as fog. Neither breaks a seam, because D5 constrains what Ouroboros reads and says nothing
about fields the harness writes with no reader.

## Progress: document two written, 2026-09-04

**`docs/run-record-contract.md` exists, 976 lines.** Two of three. Versions re-read at the top of
the session and unchanged from the harness spec's: `claude` 2.1.257, `cursor-agent`
2026.09.02-c22c1a3, `agy` 1.1.26.

Remaining order: `docs/ouroboros-spec.md`, then the seam check alone.

**Tickets 33 and 34 are now blockers and the Ouroboros document does not wait on either.** They are
both about the record contract and both feed the seam check, which is what this ticket's `Blocked
by` list is really guarding at this point. A session that wants to write document three can take
this ticket past them; a session running the seam check cannot.

### What the contract covers

Sections 1 to 17: the file and the append discipline, the envelope, `run_start` with everything
Hydra knows at launch, `run_end`, the ten event types with the producer column, the outcome enum,
usage, lineage, stored against derived, streamed against harvested, never-discard, what is
deliberately not recorded, versioning, the D12 export test as a field-by-field mapping, and what
`hydra run` returns. Section 18 holds what it could not close.

### Two probes, and one of them contradicts a closed ticket

Two small paid `agy` runs on 1.1.26, 14,304 and 6,577 input tokens.

**Ticket 25's two-usage-conventions finding is wrong.** It recorded `agy` reporting full input with
cache read beside it and `cursor-agent` subtracting, and ticket 09 carried that into the schema as a
convention field the normalized `usage` event had to hold. A resumed `agy` conversation reports
input 6,577 with cache read 8,128 and `total_tokens` 6,770, which is input plus output and excludes
cache read entirely. **`agy` subtracts too, and so does `claude`**, whose observed `input_tokens: 2`
beside `cache_creation_input_tokens: 10558` settles it. All three share one convention, so the
`usage_convention` field is struck and the reconstruction rule is stated instead. Ticket 25's claim
was made on a sample where `agy` reported `cache_read_tokens: 0`, which cannot distinguish the two
readings.

**`agy`'s `result` usage is per conversation, not per run.** The resumed run's `result` frame carried
input 20,881, output 384 and thinking 336, which are the sums across both runs. Hydra mints a fresh
run id per resume, so taking usage from `result` would double-count the first run in any corpus that
sums. Per-run usage comes from the `step_update` frames. No closed ticket saw this.

**Ticket 32's `message_delta` call stands, on better evidence.** `agy`'s `step_update` carries a
field named `text_delta`, which looks like a producer for a type ticket 32 declared dark. It is not:
a prompt asking for ten numbers one per line returned the whole ten-line body in one `ACTIVE`
frame's `text_delta`, followed by a `DONE` frame carrying the trailing newline. Buffered flush
wearing a streaming name, the same shape `cursor-agent` has without `--stream-partial-output`. The
contract states it, because the field name invites the opposite reading.

### Four things decided here rather than ticketed

**`denied` is dropped and `gated` stands**, closing section 19.2 of the harness spec. Two labels for
one state is a bug with a schema, and `denied` also read as a per-tool-call fact, which is
`permission_decision`'s job.

**`outcome_source` loses `provider` and gains `refusal`.** No routable provider produces a run-level
status worth reading: `cursor-agent`'s `result.subtype` is only ever `success` in shipped code, `agy`
reports `SUCCESS` on a fully denied run, and `claude` reports `success` on a 429. So `provider` has
no producer, and ticket 32's rule against types nobody writes applies to enum values too. `refusal`
replaces it for the four outcomes Hydra decides itself rather than reads.

**`permission_decision`, `correction` and the run's outcome are all derived, not stored.** The draft
had the first two as stored Hydra lines while also deriving them, which contradicts ticket 09's
read-time rule. Only `run_start` and `run_end` carry facts no frame holds. The sharpest case is the
outcome: it is the field a reader most wants stored, and storing it would freeze today's
classification into a corpus meant to be read for years. Section 6's `refusal` field is the narrow
exception, because Hydra refusing a launch is a fact rather than a reading.

**Ownership of a run file transfers on proof of death.** Ticket 13 says nothing appends except the
owning process and also says `hydra status` completes an abandoned run's harvest. Read literally the
first forbids the second. The rule is at most one writer at a time, and a process may take the file
only after the liveness test says the recorded pid is gone. The child never touches it under any
reading.

### Two seams this session could not check and did not fake

Both belong to the fourth session, and both now have the instrument they need.

**The producer column is built** (section 7.2), so "every field Ouroboros reads is one the harness
writes" is a table lookup rather than a re-read of thirty tickets.

**The D12 mapping is written field by field** (section 16.1), including the four things that
deliberately do not cross. It fails today in exactly one place, section 16.3: `preference.jsonl` has
one source and ticket 34 is whether that source exists.

### What the Ouroboros session inherits

**The outcome table's rung-one column is in the contract** (section 8) and the Ouroboros spec should
reference it rather than restate it. The column says what rung one learns from and never what Hydra
keeps.

**The gate-leak check is free and belongs in the Ouroboros spec's list of things worth reading**
(section 7.7). Every tool call is evaluated against the run's own policy at read time, so a call the
provider allowed and the policy denies is a gate that leaked, and `claude`'s
`subagent_stats.spawned` above zero under a policy denying subagents is the same signal directly.

**Ticket 15's checklist counts are still wrong** and this ticket's earlier note stands: the heading
says 29, the tables hold 30, and 20 of 23 carry `critical` against a stated 15 of 22. Recount before
lifting them.

## Section 7.9 needs rewriting before the seam check (from tickets 34 and 35, 2026-09-04)

`correction` was the last type whose producer was unestablished. It now has one, measured rather
than assumed, and section 7.9 as written predates all of it. Fix 7.9 before the seam check runs,
or the check validates a paragraph we already know is wrong.

Four changes, in order of how wrong the current text is.

**The auto-deny claim is too pessimistic and should be cut.** 7.9 says `claude` "labels a headless
auto-deny `user-rejected` exactly as it labels a real one". It does not. Ticket 34 read the mapping
out of the binary and ticket 35 measured it: a rule denial writes `permission-rule`, an auto-mode
denial writes `automode-blocked`, and a run with `--permission-prompts none` writes `permission-rule`
at `entrypoint: sdk-cli`. The paragraph's conclusion still holds, since policy denials stay out of
preference export, but its stated reason is false and the real separator is better than a mode check.

**Name the separator.** `toolDenialKind == "user-rejected"` **and** `entrypoint == "cli"`. Both
conditions, not either. `user-rejected` alone means the ask path answered, and under `sdk-cli` the
answerer is code: ticket 34's corpus holds three such records, all written by ticket 30's own
harness. 7.9's "interactive plus `user-rejected` is a person" is the right idea with the mechanism
left unnamed.

**Carry the harvest rule.** Rejected content comes from following `sourceToolAssistantUUID` to the
assistant record and matching the `tool_use` block whose `id` equals the denial's
`tool_result.tool_use_id`, which held on 35 of 35 auto-denials and 3 of 3 human ones. Chosen content
comes from walking children by `parentUuid` to the first assistant record holding a `tool_use`, and a
pair is emitted only when the tool name matches. A rejected `Edit` needs no special handling, since
the diff is `old_string` and `new_string` in the ordinary input.

**The human's words are the weak part, and 7.9 should say so.** They are not in a field.
`userFeedback` exists, is gated on the ask path, and was empty on all three human rejections. The
text lands as an ordinary user turn further down the same chain, so it is read by walking to the
first `user` record whose `message.content` is a plain string rather than a list of blocks. Nothing
marks that turn as a redirection rather than an unrelated new instruction, so a human who rejects a
call and then changes the subject produces a record indistinguishable from a correction. That is a
caveat the contract carries, not a problem it solves.

One thing for section 13's sweep rather than 7.9: a denial also emits a `user` record holding
`[Request interrupted by user for tool use]`. A denial contains an interrupt marker, so anything
counting interrupts by that string double-counts every rejection.

Detail in [research/34-correction-producer.md](../research/34-correction-producer.md) and
[research/35-human-rejection-shape.md](../research/35-human-rejection-shape.md). Two real pairs exist
on disk under `~/.claude/projects/-private-tmp-hydra-t35/`, and `.scratch/hydra/verify35.py` prints
the five checks against any transcript from the last N minutes.

Ticket 33 also lands on section 7.8 and section 9, and its answer says what.

## Progress: document three written, 2026-09-04

**`docs/ouroboros-spec.md` exists, 868 lines.** Three of three. Versions re-read at the top of the
session: `claude` **2.1.261**, up from the 2.1.257 both sibling documents were written against and
measured on, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.26. All three documents now say which
build their claims were measured on.

Remaining: the seam check, on its own, as this ticket has planned since the harness spec.

### What the Ouroboros document covers

Sections 1 to 8 are the settled material: what both rungs share, the three rules about not trusting a
model, what neither rung may ever do, the reference implementation read at a sha; rung one's five
failure classes, the classification honesty rule, the write surface, the fifteen curator rules with
their four amendments, and the outcomes it never learns from; rung two's pass shape, authority
boundary, sandbox, protected-path refusal, admission gate, panel, veto, branches, the missing
obligation ledger; the 30 checklist items; the seam table of what Ouroboros reads; what it writes;
and the five graduation conditions. Section 9 holds what it could not close.

### One gap, filed as ticket 36

**A rung-two pass has facts that no run record can hold.** The protected-path refusal, the admission
gate output and the panel verdict are all decided after the proposer's run file was closed, and
`run_end` is written when `wait()` returns. So the pass needs a record of its own, at depth 0, with
the proposer and the three judges as its children. That record has no provider, and the contract has
no shape for one. Ticket 36 settles it and now blocks this ticket, because the seam check is the
wrong place to find two documents disagreeing about what a run is.

The two alternatives were considered and are worse. Parenting the judges to the proposer claims the
proposer spawned its own reviewers. A pass file outside `runs/` gives the harness a second source of
truth, which is what D10 exists to prevent.

### Three things decided here rather than ticketed

**The verdict lives in the branch commit message, not in the run record.** Ticket 17 said the apply
records the verdict it consumed into the run record. It cannot: the panel runs after the proposer's
run file is closed. The commit message on `ouroboros/<run-id>` names the pass, proposer and three
judge run ids, so the audit walk is git plus four records, and a commit message is the one place a
later patch cannot edit. Ticket 17's actual requirement, that unreachable and auditable are different
properties, is met.

**The curator writes into its own git worktree of the store**, not into `~/hydra/skills/`. No closed
ticket says this and two force it: a run pins the store at a sha, and a dirty store is committed
before pinning, so a curator writing into the live working tree collides with the next launch's
commit. Flagged in the Ouroboros spec's section 9.2 as a refinement to confirm.

**The checklist is 30 items, 24 critical and 6 advisory**, counted from the rows rather than from any
heading. Four items are amended against what the sibling documents settled after ticket 15 wrote
them: `record_source_fields` said `status_source`, which the contract renamed to `outcome_source`;
`record_append_only` forbade a second writer, which as written forbids the harvest the contract
requires; `hook_dialect_coverage` said three dialects, and the gate script answers in two; and
`billed_spend_gate` was widened from `--allow-billed` to the per-run human flag pattern, which now
has two instances.

### Sibling-document edits this session made

The seam check should run against text nobody already knows to be wrong, so the four stale passages
this ticket had flagged are fixed rather than left for the seam session.

- **Contract section 7.9 rewritten** from tickets 34 and 35. The auto-deny claim is cut, the
  conjunction `toolDenialKind == "user-rejected"` and `entrypoint == "cli"` is named, the four-step
  harvest rule is carried, and step 4 is stated as a chain walk that cannot tell a redirection from a
  change of subject. The interrupt-marker double-count goes to section 13's sweep.
- **Contract section 7.2's `correction` row** now says one producer, with a named reason per provider
  for the other two, rather than "interactive harvest only" three times.
- **Contract sections 7.8, 9.3, 16.3, 18.1 and 18.2** land ticket 33 and close two open questions.
  The Cursor estimator is struck: those runs are `unavailable`, because Cursor counts about twice
  what its transcript keeps. Section 9.3 is new and holds the display-name finding and the chat-store
  id. Section 16.3 stops saying the preference source is unverified.
- **Harness spec section 17** gains `hydra evolve`, which no document named, and a note that
  `hydra usage recount` has no work in v1 now that nothing produces `estimated`.
- **Harness spec version table** stops claiming `claude` 2.1.257 is current.

### What the seam session inherits

**The instruments are built.** The contract's producer column (section 7.2) and the Ouroboros spec's
section 6 are the two halves of "every field Ouroboros reads is one the harness writes", so that
check is a table join rather than a re-read.

**Three checks are unrun**: the second direction, whether every guarantee the harness claims is one
Ouroboros relies on and no more; D5, whether anything crosses but the run record and the skill store;
and the D12 export test end to end, which the contract's section 16 wrote field by field and section
16.3 no longer reports as failing.

**One thing to check first.** Ticket 36's answer may move the pass record, and both the Ouroboros
spec's section 4.10 and the contract's section 8.3 change with it.

## Ticket 36 is resolved, 2026-09-04

The gap the Ouroboros session filed is closed and its answer is already in the documents, so the
seam check has nothing to wait on.

**What changed under this ticket while resolving 36.** The contract's section 2 widens "run" to two
kinds, section 4 admits a null `provider`, section 5.1 gains a `command` field on every run, section
5.5 is new and holds the harness run, section 6 nulls `body_source`, section 8.4 is new on the pass
outcome, the old 8.4 became 8.5, section 8.6 is new on who reads a harness run, section 10 names the
pass as the judges' parent and states the depth cost, section 14 records that a verdict is
deliberately absent, and section 16.2 gains the kind exclusion. The Ouroboros spec's sections 4.5,
4.9 and 4.10 are rewritten and its open question 9.1 is deleted rather than answered in place, so its
section 9 is now four items.

**One earlier note in this ticket is now wrong.** The Progress note for document three says
`run_end.refusal` names the protected-path check and `run_end.error` carries the compiler output.
Ticket 36 moved both to the branch commit message, since they are results about the patch rather than
about the pass.

**The seam check's three unrun checks are unchanged**, and the first one is now cheaper: the
contract's producer column and the Ouroboros spec's section 6 are both current.

## Answer: the seam check, 2026-09-04

Versions re-read at the top of the session and unchanged from the Ouroboros session: `claude`
2.1.261, `cursor-agent` 2026.09.02-c22c1a3, `agy` 1.1.26. Nothing drifted, so every measurement in
the three documents stands as written for the first time in this map's life.

All four checks ran against all three documents, 3,341 lines. **Two seams hold, two broke.** Six
mechanical contradictions were fixed here and two gaps became tickets 37 and 38.

### Check 1: every field Ouroboros reads is one the harness writes

**Passes on the recorded population, and the instrument covers only that population.**

The table join works. Sixteen reader rows in the Ouroboros spec's section 6, every one of them
resolving to a producer in the contract's section 7.2 or a `run_start` field in section 5. Rung two's
rows are clean: `provider_version` and `model` on each judge, `rate_limit_event` correctly scoped to
`claude` alone, `run_end.remediation` real, `parent_run_id` against the pass. The producer column
did what ticket 32 built it for and turned a thirty-ticket re-read into a lookup.

Two things it surfaced. `skills.resolved` is optional on `cursor-agent` and the reader side listed it
flat, now fixed. And the column resolves provider but never launch mode, which is ticket 38.

One citation is loose rather than wrong: section 6 cites contract section 17 for "what each judge
said", and 17 is the `hydra run` return JSON rather than a record field. The record holds it as
`message` events. Both are legitimate for a supervisor inside the same binary, so this is a wrong
pointer in the seam instrument rather than a crossing. Left as is, because ticket 38 rewrites the
surrounding rows anyway.

### Check 2: every guarantee the harness claims is one Ouroboros relies on, and no more

**Broke, in the direction that matters.** Ouroboros relies on four run-time guarantees the harness
spec does not make, and it is ticket 37.

The Ouroboros spec's section 2.2 says the gate blocks `hydra prune`, `hydra reap`, `hydra export` and
every write under `~/hydra/runs/` on every run under every policy, `permissive` included, and that
the five protected configs are denied at run time as well as refused at admission. The harness spec's
section 7.5 names one universal exception, the policy directory and the gate script, and calls it
"the single place `permissive` stops being permissive". Section 15.2's reconciled table gives the
other nine protected paths "Denied at run time: no", and `~/hydra/runs/` is not in the table at all.

Neither document is internally wrong. The harness spec asserts the same three conclusions in sections
10, 14 and 17 and names no mechanism for any of them, so a builder implementing 15.2's table ships a
gate that permits all three. That is the shape of gap this check exists to find, and it is the reason
the check was worth running as its own session.

The second direction, guarantees the harness claims that nobody relies on, came back clean. Nothing
in the harness spec promises Ouroboros something it does not read.

### Check 3: nothing crosses but the run record and the skill store (D5)

**Holds.** Every candidate resolved to one of the two or to neither.

The branch and its commit message are Ouroboros writing and a human reading, not a harness crossing.
The read-only brief is the supervisor reading records and putting text in a prompt. The `hydra run`
JSON is one binary calling itself, per D8. Rung one's thrashing signal joins the outcomes to the skill
store's git history, which is crossing number two by name. `no_new_crossing` and `no_daemon` are both
on the checklist with the right ceiling.

### Check 4: the record survives the export test in D12 without loss

**Holds, with one omission fixed.** Section 16.1's mapping covers nine of the ten event types and
skipped `message_delta`, which reads as an oversight in a table that lists `permission_decision`,
`usage` and the two Hydra lines as crossing to nothing. Added as a row naming its absent producer,
because a losslessness test that silently omits a slot is the thing section 7.1 forbids.

The five exclusions reconcile across the three documents: `auth_failed`, no `run_end`, curator-
parented, harness runs by kind, and any run holding an `unclassified` tool call. `usage` crossing
nowhere is what makes 7.8's provenance guarantee hold by construction, and 16.3 no longer reports a
missing producer now that tickets 34 and 35 measured one.

### Six things fixed here rather than ticketed

Each carries a decision another document had already made, so these are the edits that session missed
rather than new choices.

- **The contract had no `## 6.` heading.** `run_end` sat under section 5.5 with no heading of its own
  while eleven cross-references pointed at "section 6". Ticket 36 inserted 5.5 above it and the
  heading went with it.
- **The harness spec's section 9 carried `status` and `status_source`.** The contract renamed both to
  `outcome` and `outcome_source` and dropped the `provider` value for having no producer. The harness
  spec is where a builder reads the delegate face, so it named a field the contract does not have and
  a value the contract struck.
- **The harness spec never mentioned the harness run.** Ticket 36's answer landed in the contract and
  the Ouroboros spec and not here, so the document that owns "what Hydra writes per run" did not know
  about a record kind Hydra writes. Section 5 gains the pass row and section 12 gains the outcome
  reading.
- **`message_delta` is now in the D12 mapping**, per check 4.
- **`skills.resolved` is marked optional on `cursor-agent`** in the Ouroboros spec's reader table.
- The `hydra run` result list in harness section 9 now matches the contract's section 17 field for
  field, including the diff stat's status as the one returned value with no record equivalent.

### One thing observed and left alone

`env_allowlist_intact` names `PYTHONPATH`, which appears nowhere in the harness spec. It entered
through D6 alongside `NODE_OPTIONS` as an injection vector on `cursor-agent` and hermes, and hermes
left the routable set. The allowlist is positive, so the variable never reaches a child either way,
and the item is about the allowlist being widened rather than about that variable. Harmless, and
noted so a later reader does not file it as drift.

### Where the destination stands

Three documents exist and the seam has been checked end to end. **They are not yet buildable from,
and the reason is two named questions rather than a fog.** Ticket 37 decides which document moves on
the `permissive` gate denials, and ticket 38 gives the interactive run a field-presence table and the
producer column its second dimension. Neither is large. Both need the human, because one changes what
`permissive` means and the other changes what the seam instrument covers.
