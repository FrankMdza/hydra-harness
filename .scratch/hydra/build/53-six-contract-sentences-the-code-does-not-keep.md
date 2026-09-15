# 53 - Six contract sentences the code does not keep

Type: build
Status: open
Blocked by:

## What

From the conformance audit, `docs/reviews/2026-09-14-record-contract-conformance.md`. Each is
small on its own and the set wants one pass, because every one of them is a sentence a reader
would trust.

1. **5.3**: "`mcp.injected` and `mcp.resolved` are identical on a recorded `claude` run". Six of
   79 differ, and `01a07f3f-3241` has `["hydra"]` against `[]`. Either the sentence is wrong or
   the injection is.
2. **5.1**: "Hydra backfills the field in memory for the run it owns", about
   `provider_session_id`. `watch.rs` does not, and every recorded run carries null;
   `resume.rs` recovers the id from `raw` instead.
3. **5.4**: `auth.allow_billed` is "true only when a human typed the flag on this run". There
   is no such flag in `src/cli.rs`; all three writers hardcode false. Harness spec 17 lists
   `--allow-billed` as unbuilt, so the contract is describing a future.
4. **5.5**: "seven fields of twenty-eight". `scopes` and `resumed_from` made it thirty.
5. **8.3**: the refused-spawn shape covers one of the three refusals that reach `run::refuse`.
   The other two write provider fields the section does not mention; `01a07514-ac48` carries a
   28-name `skills.resolved` on a refused spawn.
6. Undescribed behaviours the corpus shows: a recorded run with a null policy (3 pre-gate
   records), a null `model` on a provider run (44 records), the `type` key on both Hydra
   bodies, and the four `refusal` code strings `Refusal::outcome_of` matches on.

Fix the code where the code is wrong, fix the sentence where the sentence is, and say which in
each case.
