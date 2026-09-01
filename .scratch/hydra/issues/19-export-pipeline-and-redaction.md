# 19 - The export pipeline and redaction

Type: grilling
Status: open
Blocked by: 09

## Question

Graduated from the fog by ticket 09. The record schema chose to store provider frames verbatim and to redact at export, which gives the export pipeline a security job on top of a formatting one. That decision has no home until this ticket exists.

Settle:

- **What redaction removes.** Credentials and API keys are obvious. What about absolute paths carrying a username, repository names, the contents of any file an agent read, email addresses, customer data that happened to be in a fixture. Name the classes, and say which are matched by pattern and which need a model pass
- **What proves it worked.** A redactor with no test is a promise, and ticket 05 established that a promise is not a guarantee. Decide what the export refuses to emit and how it fails
- **The write-time filter's scope.** Ticket 09 put exactly one filter at write time, Hydra's own known credentials from its auth config. Confirm that boundary, or widen it and say what that costs against the never-discard rule
- **The output formats.** Supervised fine-tuning wants conversational turns, and preference optimisation wants chosen and rejected pairs from `correction` events. Decide the concrete formats and whether both come from one pass
- **Which trajectories get excluded.** Truncated runs, runs that ended in an unresolved failure, runs stuck in a retry loop. Ticket 06 found Ouroboros blocks 63.5% of self-edits, so failure trajectories are the common case and throwing them all away may be the wrong instinct
- **Deduplication**, if the corpus turns out repetitive enough to need it

D12 keeps model training out of scope. This ticket produces the files a training run would consume, and stops there.
