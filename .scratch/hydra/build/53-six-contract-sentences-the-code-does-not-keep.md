# 53 - Six contract sentences the code does not keep

Type: build
Status: resolved
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

## Outcome

Five of the six were the sentence and one was the audit. Measured over 207 records on 2026-09-15.

1. **The sentence, narrowed.** `mcp.injected` and `mcp.resolved` are identical on a recorded
   `claude` run **whose servers all started**. Of the 77 records carrying both lists, 76 agree and
   one does not: `01a07f3f-3241` injected `hydra` and the run's own `system/init` frame says
   `{"name": "hydra", "status": "failed"}`. `resolved` is derived from the tool names the probe
   reported, so a server that failed contributes none. The audit counted six because it read
   `resolved: null` as a difference; five records carry null, which is unanswered rather than empty.
   The contract now says a difference means a server Hydra wrote that the provider could not start,
   and that harness spec 6.1's launch check runs before the child and cannot see it.
2. **The sentence, deleted.** Nothing backfills `provider_session_id`, and a claim about what
   happens in memory is not something a record can show. 4 of 203 carry a non-null value and all
   four are interactive `claude` runs, where Hydra mints the id before the child starts.
3. **The sentence, honest now.** `auth.allow_billed` is false on every record ever written, and
   harness spec 17 lists `--allow-billed` as unbuilt. The field stays so a future billed run has
   the same shape as today's corpus, and the contract says that rather than describing a flag.
4. **Seven of twenty-four**, not twenty-eight: eight keys absent on a harness run, nine null, seven
   filled, `type` left out of the count. `gate_sha`, `parent_policy` and `resumed_from` joined the
   absent row, and `provider` and `model` moved to a row of their own because they live on the
   envelope rather than in the body.
5. **8.3 described one refusal of four.** It has a table now: `policy_narrowing`,
   `skill_collision`, `mcp_unavailable`, `auth_preflight`, each with its outcome. "No provider
   fields" is true of `policy_narrowing` alone, which is decided before anything is probed; the
   other three fire after the pre-flight has read the version, the skills and the auth. The one
   refused spawn on this machine, `01a07514-ac48`, is a `skill_collision` carrying a 28-name
   `skills.resolved`, and that is the record working: a collision is only legible beside the
   listing that found it.
6. **The three undescribed shapes are described.** A recorded run with a null `policy` is pre-gate
   (three records) and an ungated interactive run reads the same way. A null `model` on 44 of 203
   provider runs means the launch named none and the vendor picked. The `type` key on both Hydra
   bodies is how a reader tells them apart.

Also written while here: `gate_sha` had no row and no paragraph in 5.1, which build ticket 49 owed
the contract and did not pay. It has both, plus the note in 5.6 that a gated interactive run writes
it absent.
