# 19 - The export pipeline and redaction

Type: grilling
Status: resolved
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

## What ticket 21 added

**Claude Code already ships the heuristic this ticket needs.** Its subprocess env scrub carries a
27-substring secret-name test, alongside an MCP env allowlist, gated on
`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`. Worth reading before inventing a pattern list from scratch.

**The secret-name surface is wider than ticket 12 knew.** Hermes alone declares 34
`api_key_env_vars`, and `CURSOR_AUTH_TOKEN`, `CURSOR_API_KEY`, `AGY_ADC_AUTH`, `JETSKI_OAUTH_TOKEN`
and `GOOGLE_APPLICATION_CREDENTIALS` all carry credentials. Full per-provider lists in
[research/21-provider-env-requirements.md](../research/21-provider-env-requirements.md).

## What ticket 13 added

**Harvested records bring a second source of untrusted content.** Interactive runs are recorded by
converting the vendor's own transcript, so whatever an agent read during an interactive session
enters the record through a path that never passed the write-time filter. Redaction here has to
cover harvested bodies, not only streamed frames.

## Answer

The ticket asked six questions, and the research answered a seventh sitting underneath them. File
contents are the largest class in a trajectory by volume, no pattern touches them, and the aggregate
leaks even when no single string does, since a run that read twelve files describes the shape of a
private codebase. Nothing redacts that. So the ticket reorganizes around scoping rather than
scrubbing: decide what may leave the machine, and redaction shrinks to a class with a fixed shape and
a contract you can test.

### Default deny, allowlisted by path

A run is exportable only if every path it read or wrote sits inside a repo declared exportable.
Computed from the record at export time, default deny.

The allowlist is a file in the repo. It joins the delegation policy and the environment allowlist as
config no rung of Ouroboros may write, and it becomes a protected path under ticket 15, so a rung-two
patch touching it is refused before review rather than blocked by it. It is Hydra's third piece of
config and the only one whose failure mode is data leaving the machine rather than a bad action taken
on it.

Export is a command a human types. No run, no curator and no delegation policy can reach it, which is
the shape ticket 12 gave `--allow-billed`.

**The check is only as good as the tool-call adapter.** Ticket 09 left mapping the providers'
incompatible tool-call representations onto one shape as open adapter work. Extracting "every path
this run touched" runs through that adapter, so a tool shape it fails to recognize is a path nobody
sees. The safe default follows the `unclassified` rule below: a run containing any tool call the
adapter could not classify is not exportable. An empty corpus beats one holding a run whose reads
were invisible.

Enforcing eligibility during the run rather than auditing it afterwards is an extension, not the
contract. Ticket 26 proved all three providers hard-block a tool call from a config Hydra writes, but
whether a block can be scoped to a path argument on all three is unverified. Written as an audit, the
spec is implementable today.

### The unit is the run, eligibility runs over the subtree

One run, one line. A parent exports only if every run beneath it is exportable, because a parent's
`tool_result` carries a child's summary of work done in another worktree. Without the subtree check,
delegation walks content out of an excluded workspace and into the corpus. A child exports as its own
line, since it is a complete task attempt on its own terms.

### What redaction removes

Four classes with mechanisms, and a residue with none.

| Class | Mechanism |
|---|---|
| Hydra's injected values | literal match against known values |
| Prefixed credentials, PEM blocks, JWTs, connection strings carrying credentials | regex plus entropy |
| The home directory prefix | literal replacement of a string known at export time |
| Email addresses | stable pseudonym derived from a hash of the address |

No model pass. Scoping removed the class a model was for, Presidio ships the honest ceiling for
everything non-model in its own disclaimer, and a model moves that ceiling rather than lifting it. It
also cannot be tested the way a rule can.

The email pseudonym dodges the all-or-nothing choice the research called the only defensible pair.
One person stays one person across a trajectory, so the conversation still reads, and no real address
survives.

**The residue, named rather than implied:** private repository and project names, customer data
inside a fixture, secrets split across messages, and unprefixed proprietary tokens. Nothing catches
these. The allowlist is what carries them, and saying so out loud is the honest statement of what
default deny buys.

Borrow the rules, write the tool. agent-sweep is MIT with 207 rules, 187 of them mapped to the
gitleaks pack, plus a checksum-validated BIP-39 detector and safety invariants worth copying. What
Hydra cannot copy is its shape, since it rewrites agent history in place and never-discard forbids
that. Hydra's redactor reads `runs/` and writes elsewhere, touching nothing.

### The write-time filter gains a detector, not a rewriter

Ticket 12's boundary holds. The only thing rewritten at write time is a value Hydra injected. A
prefix sweep for `AIza`, `sk-ant-`, `ghp_`, `ya29.` and PEM headers runs on the write path and again
at harvest, setting a flag on the envelope and leaving the frame verbatim. Never-discard survives
intact, ticket 13's harvest path stops being a hole, and a live credential on disk becomes visible
the day it lands rather than at the first export.

This is a stored derived field, which ticket 09 ruled against. The exception holds because a rescan
reproduces the flag, making it a cache rather than a fact, and what it buys is a same-day warning no
read-time function can give.

### The formats

TRL, explicit-prompt preference, both files from one traversal.

Three reasons, and the middle one belongs to this ticket rather than to taste. OpenAI's preference
format is single-turn and text only, so a `correction` at turn 40 of a tool-heavy trajectory does not
fit it. OpenAI also encodes tool arguments as a JSON string, so a redactor walking JSON values cannot
see a path or a key inside one without parsing the nested string, which would manufacture a class of
secret the redactor structurally misses; TRL's dict puts every argument at a value the walk already
visits. And ShareGPT has no `tool_call_id`, so a trajectory with two calls in flight cannot be
reassembled. Emitting OpenAI-shaped files later stays possible as a conversion from TRL, and that
direction is mechanical while the reverse is not.

**Preference pairs have exactly one valid source in v1:** a human correcting a tool call in an
interactive run. Policy denials are ruled out in writing. Ticket 14 established that `user-rejected`
in a recorded run is Hydra's policy answering rather than a person, and training on it teaches a
model to obey a config that changes, so a pair recorded today is wrong the day the policy widens.

Rung one's merged-or-dropped proposals and rung two's panel verdicts carry preference signal in a
different shape, a diff plus a verdict rather than a trajectory plus a turn. They are not forced into
this format. Whether they become a second corpus goes to the fog.

**Fields no format carries, with nothing invented to fill them.** Reasoning text goes in Harmony's
`thinking` key where a provider gave text, which after ticket 25 means Cursor and Claude; `agy`
reports a count and no text, so an `agy` line carries no thinking key rather than a fabricated one. A
truncated tool result carries a visible marker inside its string, because the alternative teaches a
model that a cut-off output was the whole output. Usage crosses nowhere, no sidecar and no metadata
file, so ticket 09's provenance guarantee holds by construction and a Cursor `estimated` count cannot
be read downstream as a measurement. The training line carries no run id; the manifest holds the
line-to-run map, so traceability survives and the UUIDv7's embedded timestamp stays in a file no
training run reads.

TRL has no per-message loss control, so masking tool results out of the loss is the trainer's
problem. D12 puts that past this ticket's edge.

### Which runs are excluded, and which are kept

The exporter judges validity, never quality. Ticket 06 measured 63.5% of self-edits blocked, so
failure is the common case, and a corpus of successes would describe a state Hydra rarely occupies.

Three exclusions, all of them "no complete task attempt exists" rather than "this one went badly".
Auth failures, which ticket 12 gave their own outcome and in which no task happened. Runs abandoned
with no `run_end`, because the trajectory has no ending, every format flattens away the fact that it
was cut, and the model would learn to stop mid-tool-call. And runs whose parent is the curator, per
ticket 14's rule against learning from itself.

Everything else exports with its outcome attached, retry loops included. A retry tree is the best
record in the corpus of what did not work.

**The exporter never upgrades an `unclassified` outcome to success.** Ticket 14 found three of four
providers close a failed run clean and lets the adapter say it cannot tell, so a label read off the
terminator would be a lie.

### Deduplication

None in v1. Hydra has run zero times, so a policy now is a guess with a decimal point on it, and
retry trees are near-duplicate by design, which is the signal rather than the noise. The manifest
carries a count of trajectories sharing a normalized prompt hash, so whoever opens the first real
corpus can see whether it repeats. Exact-hash dedup is a small addition once that number asks for it,
and near-dup clustering would eat the retry trees.

### What proves it worked, and how export fails

Redaction runs after normalization, over the line about to be emitted. A second scan then reads the
finished file before it gets a name. Two passes with different jobs: the first removes, the second
asks whether removal worked. Scanning what actually leaves is the only version testable end to end,
and the normalizer can produce strings that sat in no raw frame.

**A hit on the second scan aborts everything.** The export emits nothing and exits non-zero. Output
that the redactor's own rules still flag means the redactor is broken, and a partial corpus from a
broken redactor is worse than no corpus. Dropping only the offending run would hide the bug behind a
slightly smaller file.

Proof comes in two pieces:

- A fixture suite over the four mechanised classes, asserting both directions. The secret is gone and
  the trajectory around it survives, because a redactor that emits an empty file passes every "no
  secret found" test.
- A canary on every export. Inject a synthetic secret of each class into a throwaway record and
  assert it never reaches the output. The suite tests the rules; the canary tests that the stage ran
  at all with its pack loaded.

The residue gets no test, and the spec says so rather than implying coverage. What stands in for it
is the allowlist and the subtree check, testable in the other direction: a run that touched a path
outside the allowlist is refused.

### What one export produces

A write-once directory holding `sft.jsonl`, `preference.jsonl` and `manifest.json`. The manifest
carries the rule pack version and sha, the allowlist sha, the Hydra version, per-file counts, the
prompt-hash collision counts, the line-to-run map, and an exclusion list naming every run considered
and not exported with the reason it was dropped.

The exclusion list earns its place. Two separate rules here throw runs away, and without a record
"why is this run not in the corpus" has no answer, while an allowlist typo that silently empties the
corpus looks exactly like a quiet week.

No promise of a byte-identical re-export, because the rule pack moves underneath one. A corpus is
identified by what produced it, which is why the manifest pins versions rather than hashing output.

### What this changes elsewhere

- **Ticket 14** gains a third protected config. Rung one may never write the delegation policy or the
  environment allowlist; the export allowlist joins them, and it is the only one whose failure is
  data leaving the laptop.
- **Ticket 15** gains a protected path, so a rung-two patch touching the export allowlist is refused
  before review.
- **Ticket 09** gains the envelope flag from the write-time sweep, with its exception to the
  no-stored-derived-fields rule written down.
- **Ticket 16** takes the export contract from here. `hydra export` belongs in the harness spec.
