# 290 - The task crosses into the export

Type: build
Status: open
Blocked by:

## What

Map 113 (`.scratch/hydra/issues/113-does-the-task-cross-into-sft.md`): `run_start.task` crosses
into each exported line as its first `user` turn, preamble included, through the same redaction
pass and verification scan as every other value.

1. **Eligibility.** Every path the task names counts toward eligibility as if a tool had touched
   it. Extract absolute and `~/` paths from the task string with the same normalisation the tool
   path extractor uses; a path outside the allowlist marks the run `path_outside_allowlist`
   exactly as a tool path does. A test plants a task naming a path outside a scratch allowlist and
   reads the run's exclusion back from `--dry-run`.
2. **The line.** `export_events` (`src/export.rs:465`) puts the task first, as a `message` event
   with `role: user`, so the existing redaction walk visits it. A test plants a private repo name
   and a secret prefix in a task and reads both back redacted, and the scan abort on a planted
   unredactable secret.
3. **Contract.** Amend `docs/run-record-contract.md` 16.1: the `run_start` row now reads "its
   `task`, as the first `user` message", with one sentence pointing at map 113. Amend
   `IMPLEMENTATION.md` in the same change.
4. **Before any export with tasks leaves the machine**, the session reads the 56 eligible tasks
   with fenced code (map 113's audit) and confirms none pastes content from outside `~/hydra`.
   That is a reading job for the session, not code; note the result under Outcome.

Fable's split (preamble as `system`, the written task as `user`) is allowed if the cut is exact,
meaning the preamble is a byte-identical prefix taken from `config/preamble.md`'s rendered text;
otherwise one `user` message. Never invent a message the record does not hold.

Out of scope: `preference.jsonl` (its `prompt` already reads the proposer's task), the training
scripts under `.scratch/hydra/training/`.

## Outcome
