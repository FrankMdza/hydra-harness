# 16 - Assemble the three documents and check the seam

Type: task
Status: open
Blocked by: 09, 10, 11, 12, 13, 14, 15, 19, 20, 23, 24, 25, 26, 27, 28, 29, 30, 31

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
