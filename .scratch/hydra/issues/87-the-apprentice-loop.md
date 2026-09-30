# 87 - The apprentice loop: the local model works the bank, and a panel grades it

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-09-30, by the session, at the human's request.

## Question

The human wants the local model always doing something, and the thing he wants it to learn
first is Hydra itself: how to use it, how each provider's harness behaves, what a record says.
His words: the model tries tasks and solutions, then the panel gives it the training set for each
harness, and the set grows.

What exists: `gemma3:1b-it-qat` on Ollama, served as the MCP tool `ask` and used by build 146 as
a judge, where it failed (3 of 15). The grader dataset with 553 rows, 521 classified. A verdict
model. `export_preference.rs`, which already reads preference pairs (corrections and judge
verdicts) for `hydra export`. `docs/rungs-spec.md` line 299: "A/B across store shas needs
repeated tasks, which real work does not provide." Phase 1.6 of the plan asks for a bank of 20
repeatable tasks and nobody wrote it.

What constrains: D10, nothing has to be running for the harness to exist. 8 GB: Ollama loaded
beside the agents put the swap at 17 GB on 2026-09-29. The local model cannot be a Hydra
provider today: it has no CLI and cannot use tools reliably. Quota: every panel seat spends it.

## Answer

Resolved 2026-09-30, by the session, under the human's delegation. The human can reverse it.

**A nightly batch outside `src/`, three steps, and a bank that grades itself where it can.**

1. **The bank.** `integrations/apprentice/bank/` holds tasks in two classes. *Verifiable*: a
   question about a real record with an answer the script computes from the record (the outcome,
   which tool the gate refused, how many seats a session launched, which provider ran it, what
   `hydra show` would print for a field). These grade themselves, cost no quota, and are the
   repeated tasks rung two never had. *Open*: write the `hydra run` command for a task, choose the
   provider and policy for a task and say why, summarize what a run did, say what a refusal means.
   These need a grader. Twenty of each to start, drawn from records, with the run id as the seed
   so the bank regenerates.
2. **The attempt.** `apprentice.py attempt` asks the local model each task through Ollama, one
   at a time, and writes `grader/apprentice/attempts.jsonl`: task id, prompt sha, answer, model,
   time. Verifiable tasks are scored on the spot. It runs from launchd at 04:00 local, when no
   agent is alive, for at most 30 minutes; and by hand any time. Hydra works with it stopped.
3. **The grade and the pairs.** `apprentice.py grade` collects the open answers of the week and
   the session launches one panel of three seats through `hydra run` (the `review` shape's
   providers), which grades each answer on a fixed rubric and writes a better answer when the
   local one fails. `apprentice.py pairs` turns that into preference pairs, one per task: the
   prompt, the chosen answer, the rejected one, the harness it is about, and who graded it, in
   the shape `export_preference.rs` reads, so they leave the machine only through `hydra
   export`. That file is the training set per harness; it grows every week.
4. **What trains on it.** Nothing yet. Phase 3 of the plan says when a LoRA on Gemma 3 4B QAT is
   worth a rented GPU: when the pairs are in the hundreds and the verifiable score has stopped
   rising on prompt changes alone. The weekly verifiable score is the number that says whether
   the loop learns anything.

Not a provider, not a daemon, no records to any API: the panel reads the local model's answers
and the bank's prompts, which are questions about records the same vendors already ran.

Build ticket 160 for steps 1 and 2 and the pairs writer; the first grading panel is a session
task, not code.

Reversed by: a verifiable score that does not move in four weeks, which would say the 1B model
is too small for this and the loop should wait for the 4B.
