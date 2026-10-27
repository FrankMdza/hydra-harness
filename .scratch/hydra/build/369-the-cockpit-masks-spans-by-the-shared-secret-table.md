# 369 - The cockpit masks each secret span by the shared table, and the docs say the five are the alarm subset

Type: build
Status: resolved
Blocked by: 368

## Objective

Carry out the second half of map ticket 108, `.scratch/hydra/issues/108-one-list-of-secret-classes.md`, with the panel's reasoning in `.scratch/hydra/research/108-synthesis.md`. Read both, then build 368's Outcome, which created `crates/secret-classes`.

Today the cockpit hides a line only when its envelope carries `secret_flag`, which the write path sets for five classes. A line holding `AKIA`, `xoxb-`, `gho_`, `github_pat_`, the Slack webhook host or an OpenAI key shows in full, though export would redact it.

## Rules

1. A line whose envelope carries `secret_flag` stays hidden whole, as today, and the reveal key stays refused for it.
2. A line without the flag is shown with each span the shared table matches blanked, the way export cuts a token: the prefix and the rest of the token up to the first quote or whitespace, and the OpenAI span rule with its lengths. The rest of the line stays. The blank names the class, for example `[aws_access_key]`.
3. The rule applies everywhere the cockpit prints run text: the tail through `tail_line_shown` in `src/cockpit/call_head.rs` line 543, the call head built from a call's inputs, the graph in `src/cockpit/graph.rs`, and the detail view in `src/cockpit/snapshot.rs` around lines 1060 and 1218. `flagged_seqs` at line 1103 keeps reading the stored flag only.
4. `toggle_reveal` in `src/cockpit/app/mod.rs` line 931 reveals a span-masked line only to the extent the stored flag allows: a span from the shared table stays blanked after reveal. Say in a comment why.
5. `record::secrets::prefixes()` in `src/record/secrets.rs` has no caller. Delete it.

The matcher counts bytes, the way `src/export.rs` does. If export's matcher can be reused without pulling the export module into the cockpit, reuse it; otherwise write a small one beside the cockpit and test it against export's on the same inputs.

## Do

1. Tests first: in the cockpit's own tests, a tail line, a call head and a graph label each holding `AKIAPlantedSecretKey` and an OpenAI-shaped key, with no stored flag, render with both spans blanked and the rest intact; a flagged line still renders hidden whole; reveal does not unblank a span. A line with the prose `` the `sk-proj-` prefix `` stays as it is, matching the OpenAI length rule.
2. Docs: `docs/run-record-contract.md` section 13 near line 1398 says the write path scans the five alarm classes of the shared table, that the cockpit masks the full table on display, and that export redacts the full table. Fix the sentence that says the sweep "runs again at harvest": harvest writes through the same `provider_line`, so there is one scan. `SECURITY.md` where it names the redactor's twelve classes says the same. `docs/harness-spec.md` only if it names the list.
3. Amend `IMPLEMENTATION.md` where it describes the cockpit's secret hiding.
4. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo nextest run --release -p hydra --lib cockpit`, and `cargo nextest run --release -p hydra --test export`.

## Outcome

Built. The cockpit reads the full shared table through a byte matcher in `call_head.rs`.
Unflagged tail lines, call heads, full inputs, graph messages and graph results replace each
matched span with its class name in brackets. Snapshots mask before reveal. Stored flags still
hide whole lines and refuse reveal. `flagged_seqs` still reads only the stored flag.
The unused `record::secrets::prefixes` is gone.

Tests first reproduced the exposed call input and revealed tail. New cockpit tests cover both
planted keys, surrounding prose, short OpenAI prefixes, detail calls, graph messages and results,
stored flags and reveal. A parity test compares the cockpit matcher with export on the same
inputs across every prefix, OpenAI body lengths and boundaries, Unicode, Slack URLs and PEM blocks.

Measured. `cargo fmt --all --check` passed.
`cargo clippy --release --workspace --all-targets -- -D warnings` passed.
`cargo nextest run --release -p hydra --lib cockpit` passed 18 of 18.
`cargo nextest run --release -p hydra --test export` passed 26 of 26.

Found. Export's span rules also include the Slack URL scheme and carry PEM blocks through their
footer. The cockpit follows both. The harness spec repeated the harvest rescan claim, so it and
contract section 13 now say harvest uses the same `provider_line` scan. `SECURITY.md` and
`IMPLEMENTATION.md` distinguish the five alarm classes from full-table display masking and export.
