# 108 - One list of secret classes

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-03, by the session, from finding 9 of the Rust review (`.scratch/hydra/research/275-rust-review.md`).

## Question

Three lists disagree: the write path flags five classes (`src/record/secrets.rs:11-17`), export
redacts twelve plus OpenAI keys (`src/export.rs:578-591`), and the grader repeats export's
(`crates/grader/src/redact.rs:16-29`). The cockpit hides a line only when it carries
`secret_flag`, so an `sk-proj-`, `AKIA`, `xoxb-`, `gho_` or `github_pat_` key shows in its tail
though export would redact it, and `record::secrets::prefixes()`, which its doc says the cockpit
masks by, has no caller. Widening the write-path sweep changes what a record carries. Options:
(A) the write path takes export's list, one shared list; (B) the write path stays, the cockpit
masks by export's list on display; (C) both.

## Answer

Resolved 2026-10-27 by a four-seat panel, which the human delegated the decision to. Reasoning and votes are in `research/108-synthesis.md`.

**B, three to one.** The write path keeps its five classes, so a record stays byte-identical and the section 13 rescan claim holds. Export keeps redacting the other seven rather than refusing their runs. One table of classes lives in a new leaf crate, with a flag on the five the write path scans, and the write path, export's redactor and verifier, `hydra-records` and the cockpit all read it. The cockpit blanks each matched span on display for a line without a stored flag and keeps the whole-line hide for a line with one. Build tickets 368 and 369.
