# 380 - Round two's preflight: the real bill, the fenced tasks, and a budget with cleanup in it

Type: build
Status: resolved
Blocked by:

## Objective

Map 137, ticket 1 of four. Before any GPU is rented for round two, every number round two rests
on has a source or is marked unknown, and build 290's leftover precondition is met. No spend.

## Scope

1. Round one's real cost, read from the GCP billing console or a billing export, beside the
   README's estimate of 0.25 USD. If the session cannot read it, it is the human's item.
2. Build 290 item 4: read every eligible task that carries fenced code, and list any whose fence
   holds content from outside `~/hydra`. Each one is either excluded from the export or kept with
   a reason.
3. Round two's budget in writing: GPU hours, disk, egress, and cleanup, against the 10 USD cap,
   which warns and does not cut.
4. GCP is empty before the round: no instance, disk or bucket.

## Done when

The Outcome holds the four items, each with a source or "unknown", and `training/round1/README.md`
points at it.

## Outcome

Done by the session on 2026-11-04, no spend, nothing left the machine. The export was written to
the session's scratchpad and read there.

1. **Round one's real cost: unknown.** `gcloud` shows no cost data without a billing export to
   BigQuery, which the project does not have. The human reads it in the console (Billing,
   Reports, project `<gcp-project>`, 2026-10-04). The 0.25 USD stays an estimate.
2. **Build 290 item 4, the fenced tasks.** `hydra export` today: 334 eligible runs, 56 tasks with
   fenced code. **37 of the 56 paste private-repo code or results** (details withheld), and none of them names a path outside `~/hydra`, so eligibility lets them
   through. That is the residue Grok named on map 113 ("free text can paste a file that no path
   check sees"), and it is real. Each of the 37 records was read and confirms it. The other 19
   carry Hydra's own diffs (`src/paths.rs`, `src/lib.rs`, `src/cli.rs`) and the apprentice's JSON
   task shape, which is inside the allowlist. The path scan's other hits were false: `etc.` in
   Spanish prose and `~` as pandas negation or "about".
   For round two the 37 are excluded by id: `training/round2/exclude-private-repo.txt`. Whether
   eligibility should catch pasted content by itself is Grok's option C on map 113, which is a
   decision, so it goes to the human rather than into code here.
3. **Round two's budget.** One spot GPU VM with `--max-run-duration 2h` and its boot disk, no
   bucket, nothing else billed. Ceiling for the round: 1 USD, map 137's target, read off the bill
   after the round. Prices for today's spot T4 or L4 were not read; round one's estimate is the
   only number, so the first act of 383 is to read the price page and write it into 383.
4. **GCP before the round:** empty. `gcloud compute instances list`, `compute disks list` and
   `storage buckets list` all list 0 items.

Assumed and false: that build 290's precondition would find paths. It found pasted content with
no path at all.
