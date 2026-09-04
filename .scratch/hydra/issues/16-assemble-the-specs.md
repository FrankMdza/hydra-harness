# 16 - Assemble the three documents and check the seam

Type: task
Status: open
Blocked by: 09, 10, 11, 12, 13, 14, 15, 19, 20, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34

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
