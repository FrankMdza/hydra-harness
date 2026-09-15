# The record contract against the code and against 200 real records

Date: 2026-09-14. Method: an agent read `docs/run-record-contract.md` in full, then the writer
(`src/record/`), the three readers (`src/normalize/`), and the places that fill line 0
(`src/run.rs`, `src/interactive.rs`, `src/ouroboros/evolve.rs`). It then walked every record in
`~/hydra/runs/`, 200 files and 102,051 lines written by three providers over two weeks, and
compared the three readers against each other. `cargo test --test record_contract`: 20 of 20,
including the corpus walk.

## What the corpus says

Zero validator failures. Zero unresolved `parent_run_id` among the 16 that carry one. `seq`
gapless and equal to the line index in every file. `run_start` on line 0 everywhere,
`schema_version: 1` on all 200.

| | |
|---|---|
| outcome / source | 142 completed/derived, 29 completed/unknown, 14 gated/derived, 9 rate_limited/derived, 4 failed/derived, 1 refused |
| provider | 99 `claude`, 65 `cursor-agent`, 28 `agy`, 8 harness runs with none |
| command | 190 run, 8 evolve, 2 resume, 0 curate |
| `usage_source` | 186 provider, 13 unavailable |
| `body_source` | 192 stream, 4 harvest, 1 null |
| secret patterns caught by the scan | 47 `anthropic_api_key`, 29 `google_api_key`, 23 `github_pat`, 23 `google_oauth_token`, 19 `pem_private_key` |

Fields absent on some records all have a reason the contract gives: 8 harness runs carry no
provider block, 36 predate `scopes`, 43 predate `base_tree`, 9 judge runs get no skills.

## Six places the corpus contradicts the contract

| Contract | Corpus | Status |
|---|---|---|
| Section 6: `body_source` is null on a harness run | `"stream"` on all 8 | **fixed 2026-09-14**, `RunEnd::harness`, with a test |
| 5.3: `mcp.injected` and `mcp.resolved` are identical on a recorded `claude` run | 6 of 79 differ | build ticket 53 |
| 5.1: Hydra backfills the session id in memory for the run it owns | `watch.rs` does not | build ticket 53 |
| 5.4: `auth.allow_billed` is true when a human typed the flag | the flag does not exist | build ticket 53 |
| 5.5: seven fields of twenty-eight | seven of thirty, since `scopes` and `resumed_from` | build ticket 53 |
| 8.3's refused-spawn shape | covers one of the three refusals that reach `run::refuse` | build ticket 53 |

And five behaviours the contract does not describe at all: a recorded run with a null policy
(3 records, all pre-gate), a null `model` on a provider run (44 records), `provider_session_id`
null on every recorded run rather than sometimes, the `type` key on both Hydra bodies, and the
four `refusal` code strings the outcome derivation matches on.

## The validator checks less than the contract claims

`RUN_START_FIELDS` does not include `floor_lifted`, `env_allowlist`, or the four leaves inside
`auth`. `floor_lifted` is missing from 27 records and nothing noticed, which is the point: the
validator is what the contract says makes it checkable. Adding them is build ticket 54, and it
needs care, because 27 existing records would start failing a walk that has to keep reading
them.

## The three readers are not equally strict, and nothing says so

Each reader emits what section 7.2 allows its provider and nothing it denies. The asymmetry is
in the findings, not the events.

| | `claude` | `cursor-agent` | `agy` |
|---|---|---|---|
| Tool-table check | every name in `system/init.tools` | **only names actually called**, because Cursor's init carries no tool list | every name in `init.tools` |
| `Ungated` finding | every allowed call with no Hydra voice | only where another hook refused | same as Cursor |
| Bar for calling a leak | the call's fate is known | result present, `no-path` excused | **result status `Ok` only** |

So an `Ungated` count is not comparable across providers, a renamed Cursor tool stays invisible
until something calls it, and `agy`'s leak count is a floor where `claude`'s is not. Build
ticket 55: either the finding means the same thing on all three, or the contract says it does
not.

One inconsistency inside one file: `claude.rs` recognises Hydra's voice through `hydra_voice`,
which validates the code against `GATE_CODES`, on the stream path, and accepts any text
starting `hydra/` on the harvest path. `GATE_CODES` is also missing `unhooked` and
`capped-argument`, which the other two readers write.

## What this review did not find

No record on this machine fails the contract's structural rules. No reader emits an event type
its provider column denies. Nothing constructs `MessageDelta`, which matches 7.4. Harvest is
refused for any provider but `claude`, which matches 7.2.1. The 166 non-JSON provider lines
across 162 records all land in `raw` as strings, as section 3 requires.
