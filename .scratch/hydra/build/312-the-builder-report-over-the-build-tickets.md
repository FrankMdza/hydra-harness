# 312 - The builder report over the build tickets: who built what, and whether it stood

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session. Build 301 printed acceptance and cost per task class and
model from the goal logs, and found two goals and eleven tasks. The last month's builds did not
go through goal logs: they went through `hydra run` from a ticket under `.scratch/hydra/build/`,
and each Outcome names the run, the provider and model, the tokens, and what the session did
with the diff. Sixty-three tickets name a run id today. That corpus is what the human's
question about a model that learns to delegate needs, and nothing prints it.

## What

An Outcome reads like "Built by `codex` `gpt-6.1-sol` on Hydra run `01a1340b-...` under
`standard`, 3.13M tokens, applied to master on 2026-10-13 after the diff was read whole", or
"Built 2026-09-24 by `codex` on run 01a0d36f-4447 (1.56M tokens)", or "Built by `composer-2.5`
on run 01a0d587-e3e1 under `standard`, about 0.59M". The phrasing varies, the run id is the
constant: a uuid-v7 prefix of at least 13 characters starting `01a`. The run record carries the
truth the prose repeats: provider, model, policy, outcome, tokens, duration. The prose carries
what the record cannot: whether the session applied the diff as it was, corrected it, finished
it by hand, or rejected it.

## Do

1. `hydra-puppets report tickets --dir .scratch/hydra/build --runs <runs dir>`: for every
   ticket file with `Type: build`, take the text under `## Outcome`, find every run id prefix in
   it (`01a` plus hex and hyphens, 13 characters or more), resolve each against the run records
   (the record file name starts with the id; a prefix that matches two is ambiguous and is
   listed as such), and read provider, model, policy, outcome and total tokens from the record
   with the readers `hydra-puppets` already has for `run_end` and usage (read `wait.rs` and
   `report.rs` first; do not parse the record by hand if a reader exists).
2. Classify the prose with a small fixed vocabulary, and print the classification beside the
   phrase it matched so a reader can disagree: `applied as is` (phrases such as "applied as it
   was", "entered without correction", "aplicado tal cual"), `corrected` ("needed correction",
   "the session fixed", "with one fix", "amended by the session"), `finished by hand` ("the
   session finished", "died", "capacity", "completed the mechanical rest"), `rejected`
   ("rejected", "not applied", "discarded"), and `unsaid` when none matched. Keep the phrase
   list in one place at the top of the module with a comment saying it is a heuristic over
   prose and will be wrong sometimes.
3. Print two tables. Per provider and model: tickets, runs, tokens total and median, outcome
   counts from the records (`completed`, `failed`, the rest), and the four prose classes.
   Per ticket, sorted by number: ticket, runs, model, tokens, record outcome, prose class and
   the matched phrase. Run it on this repo and put both tables in the Outcome, whole.
4. One test under `crates/puppets/tests/` on a fixture directory of four tickets and three
   fake records, covering: a ticket with two runs, an ambiguous prefix, a run id with no
   record, and each prose class once.
5. `integrations/puppets/README.md` gains the command beside `report builders`. Amend
   `IMPLEMENTATION.md` where `report.rs` is described.

## Done means

The two tables in the Outcome, with the `unsaid` count stated plainly so the human knows how
much the prose fails to say; the test; `cargo fmt --all --check`, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo nextest run --release -p hydra-puppets` green.

## Outcome
