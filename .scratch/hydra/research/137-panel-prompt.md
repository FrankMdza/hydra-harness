# Decision panel for map ticket 137

You are one seat on a four-seat decision panel. Read only: do not edit, create or delete a file in
the repository, and do not commit. Scratch experiments under `/tmp` are fine. Your seat name is in
your task line. The human is present and decides on the panel's recommendation, so commit to one
option per question and say why.

## Read

1. `.scratch/hydra/issues/137-the-lab-s-next-arc-cloud-experts-and-post-quantum.md`, the questions.
2. `.scratch/hydra/research/137-sources-digest.md`, the five documents and the state of the lab.
   The documents' numbers are unverified claims; say so when your answer rests on one.
3. `.scratch/hydra/issues/133-tiny-experts-from-recorded-runs.md` and
   `.scratch/hydra/research/133-the-plan-of-2026-10-15.md`, the MoE work this builds on.
4. `.scratch/hydra/issues/77-a-local-model-for-the-mode.md` (its Answer and addendum) and
   `.scratch/hydra/training/round1/README.md`, the cloud cost and the one training round.
5. Whatever code you need to check a claim: `src/sandbox.rs` for bubblewrap, `crates/apprentice/`,
   `crates/grader/`, `config/export-allowlist.json` if it exists, `docs/harness-spec.md`.

## Answer

For each of the five questions: your option, the reason in two or three sentences, and the one
risk that would make you change your vote. For question 3, rank all four and name the Rust crate
for your first choice. For question 4, one line per idea: map now, fog, or drop. For question 5,
three tickets, one line each, smallest first, each with what it measures or builds and how you
would know it worked.

Check at least two claims against the repository and say which, with file and line. A claim you
did not check, say so.

At most 900 words. End with `## Pending from me`, `## Changed` and `## Found`.
