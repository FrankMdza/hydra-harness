# 16 - Assemble the three documents and check the seam

Type: task
Status: open
Blocked by: 09, 10, 11, 12, 13, 14, 15, 23, 24, 25

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
