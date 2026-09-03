# 16 - Assemble the three documents and check the seam

Type: task
Status: open
Blocked by: 09, 10, 11, 12, 13, 14, 15, 19, 20, 23, 24, 25, 26, 27, 28, 29, 30

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
