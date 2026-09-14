# 08 - Account and tier inventory

Ticket: [08 - Inventory the accounts and tiers](../issues/08-account-inventory.md)

**Withdrawn before this repository was published, 2026-09-12.** This document listed which
account signed in to each provider on the machine Hydra was built on, and which subscription
tier each carried. It answered ticket 08 and its answer is summarized in that ticket and in the
map's D4 and D20, which is what the design needs. The inventory itself is one person's account
list and belongs on that person's machine.

Anyone reproducing it needs no special method: each vendor CLI prints its own account with
`claude /status`, `cursor-agent status` and `agy auth status`, and the run record carries
`auth.mechanism` and `auth.source` per run, which is the part the design depends on.
