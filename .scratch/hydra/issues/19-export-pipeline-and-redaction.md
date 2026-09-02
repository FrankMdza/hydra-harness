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

## What the format survey found

Formats, shapes and sources in
[research/19-export-formats-and-redaction.md](../research/19-export-formats-and-redaction.md).

The formats disagree precisely on tool traffic, which is most of an agent trajectory, so the choice
of SFT shape is not cosmetic. OpenAI's preference format is single-turn and text only, while TRL's
takes full message lists with tool support, which is what decides whether SFT and DPO can come from
one pass. Neither gitleaks nor trufflehog rewrites content, so Hydra writes the redactor whatever it
wraps.

## What ticket 12 settled

**Write-time filtering covers only what Hydra injected.** That is `GEMINI_API_KEY` on a run carrying
`--allow-billed`, and nothing otherwise. Hydra can promise to strip a value it handed over and
nothing more, so every other secret that lands in a raw frame is this ticket's problem.

**Pattern scanning belongs here, as best-effort.** Known shapes worth sweeping: `AIza` for Google,
`sk-ant-` for Anthropic, `gh[pousr]_` for GitHub, `ya29.` for Google OAuth access tokens. Frames are
stored verbatim, so an agent that ran `env` or read a dotfile has put real credentials on disk.

**One machine-local fact for the threat model.** `~/hydra` has no git remote and never has, and the
three plaintext credential files found on this machine live outside the repo. Export is the only
path by which any of this leaves the laptop, which is what makes redaction here worth its cost.
