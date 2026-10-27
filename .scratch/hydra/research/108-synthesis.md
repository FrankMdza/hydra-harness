# 108 synthesis: one list of secret classes

Panel of 2026-10-27, decided by the panel because the human said so before leaving. Seats, all under `standard`, read only:

- Opus, `claude-opus-5-5`, run `01a17e9d-68d2`.
- GPT, codex `gpt-6-luna`, run `01a17e9e-05cf`.
- Grok, cursor `grok-4.7-high`, run `01a17e9e-9f8c`.
- Flash, agy `gemini-3.8-flash-high`, run `01a17e9f-492b`.

## Vote: B, three to one

Opus, Grok and Flash voted B: the write path keeps its five classes, and the cockpit masks by the full list on display. GPT voted C.

Three reasons carried it, and the B seats agreed on all three:

- **The cache claim.** Contract section 13 stores `secret_flag` only because a rescan reproduces it exactly. Widening the sweep makes a rescan of an old record find classes its stored flag lacks. B keeps the claim true with no migration.
- **Export eligibility.** `has_secret_flag` in `src/export.rs` refuses a whole run. Under A or C every run that touched `AKIA`, `xoxb-`, `gho_`, `github_pat_`, the Slack webhook host or an OpenAI key would stop exporting, though build 128 added those classes to be redacted, not refused. GPT argued the refusal is the safer default. The others pointed out that the redactor's verifier already aborts when a class gets through.
- **False positives.** `AKIA` and the Slack host appear in this repository's own docs and source, and Flash noted `AKIA` sits inside ordinary words. A `contains` sweep at write time would flag any run that reads those files.

## Where the list lives: a leaf crate

Opus and Grok: a new crate with no dependencies, because the `hydra` binary does not depend on `hydra-records` and `hydra-records` pulls in `hydra-pyjson`. Flash would make the binary depend on `hydra-records`. The session takes the leaf crate. It is the smaller dependency for the binary, and two seats checked the dependency graph with `cargo tree`.

The crate holds the data only: the twelve prefixes with their labels, the OpenAI span rule, and a flag on the five write-path classes. The matching code stays where it is, because `hydra-records` counts Python code points and `src/export.rs` counts bytes. A test pins the flagged five.

Readers, all four of them: the write-path sweep in `src/record/secrets.rs`, which reads the flagged rows; export's redactor and its verifier in `src/export.rs`, which Grok found is a separate copy; `crates/records/src/redact.rs`; and the cockpit.

## The cockpit

Grok's rule: a line with a stored `secret_flag` stays hidden whole as today. A line without one has each matched span blanked on display, the way export cuts a token, and the rest of the line stays. That blinds the operator less than Opus's whole-line hide for the extra classes. Opus found the gaps any version must close: the call head built by `tail_line_shown` in `src/cockpit/call_head.rs`, `flagged_seqs` in `snapshot.rs`, and the reveal refusal in `toggle_reveal`, which today checks only the stored flag. `record::secrets::prefixes()` has no caller; it goes, or returns the flagged subset.

## Found on the way

- The ticket's path `crates/grader/src/redact.rs` is stale. The list is in `crates/records/src/redact.rs`.
- The gate's pattern floor holds a fifth overlapping list. It refused Opus's `grep` for the Slack host. Out of scope here.
- Contract section 13 says the sweep "runs again at harvest". Harvest goes through the same `provider_line` as the first write, so there is no second scan. The contract text is wrong in a harmless direction.

## Build tickets

1. 368: the leaf crate `crates/secret-classes`, and every list switched to it: `src/record/secrets.rs`, export's redactor and verifier, `crates/records/src/redact.rs`. Records stay byte-identical.
2. 369: the cockpit span-masks by the shared table and closes the head, `flagged_seqs` and reveal gaps; contract section 13, harness spec 14 and `SECURITY.md` say the five are the alarm subset.
