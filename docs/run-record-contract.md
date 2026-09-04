# Hydra run record contract

The format Hydra writes and Ouroboros reads. Under D5 this and the skill store are the only two
things that cross between the two systems, and under D10 the record is the source of truth: nothing
needs to be running for it to be complete, and anything not in a run record did not happen.

This is one of three documents. `docs/harness-spec.md` covers how Hydra launches, feeds and records
the three provider CLIs. `docs/ouroboros-spec.md` covers both rungs of self-improvement. The three
are meant to be read together.

Provider versions this document was written against, read 2026-09-04 at 12:38:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.257 | 2026-09-04, unchanged since 2026-09-01 |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-04, unchanged since 2026-09-02 |
| `agy` | 1.1.26 | 2026-09-04, unchanged since earlier that day |

Two of the three self-update without asking, and it has invalidated a finding four times. Every
frame shape below carries the version it was measured on. Re-read all three versions before
trusting any of them, and re-measure rather than inherit when one has moved.

## 1. What the record is for

Three readers, and they want different things from the same file.

**Ouroboros** reads a corpus of finished runs and looks for failures worth learning from. It needs
to tell a run that failed at its task from a run that never got to try, which is why section 8's
outcome enum is longer than it looks like it should be.

**`hydra status`, `hydra logs` and `hydra show`** read a file that may still be growing, and have to
answer whether the process behind it is alive without asking anything but the filesystem.

**`hydra export`** turns finished runs into training data. D12 keeps training out of scope and keeps
lossless export as a standing constraint, so the record is wrong if a trajectory cannot leave it in
SFT and preference-pair shape. Section 16 is the test.

Nothing else reads it. A TUI eventually will, and ticket 07's Textual choice puts it in a separate
process reading `~/hydra/runs/` rather than on a channel of its own, which is what keeps D5 at two
crossings.

## 2. The file

**One file per run**, `~/hydra/runs/<run-id>.jsonl`, run id a UUIDv7 so the directory sorts by
time and a run's start is recoverable from its name.

A **run** is one provider process Hydra launched, or one it refused to launch (section 8.3). A
**session** is a tree of runs linked by `parent_run_id`, reconstructed at read time and stored
nowhere. This is the clarification ticket 09 made to D10 rather than a reversal of it: D10 said one
file, and the question D10 did not answer was one file per what.

**Children never append to a parent's file.** Concurrent appenders on one file is where JSONL
durability stops being free, and the tree costs nothing to rebuild from a field that has to exist
anyway.

**At most one process appends to a run file at a time, and the child is never that process.**
Death is a conclusion a reader draws from section 5.2's liveness fields, never an event a bystander
writes. Section 6 has the ownership rule in full, because harvesting a run whose owner died needs a
more careful statement than this one.

**Append line-buffered, one `fsync` per line, and readers tail.** Agent events arrive at single
digits per second, so the `fsync` is noise on an SSD, and a tailer waits for a newline before
parsing. This is what lets a live reader work with no privileged channel. Revisit it with evidence
from a real load and not before.

`~/hydra/runs/` is in `.gitignore`. `hydra prune` moves run files rather than deleting them unless
a flag says otherwise, because anything deleted is training data destroyed.

## 3. The line

Every line is a Hydra envelope. Lines come in two kinds and the difference decides what the reader
has to do with them.

**A provider line** wraps one provider frame verbatim, in a `raw` field. Nothing is discarded, no
field is dropped, and no value is rewritten, with the single exception in section 13. The normalized
events in section 7 are derived from `raw` at read time by a library function, never stored.

**A Hydra line** carries an event body Hydra authored and has no `raw` field. There are exactly two,
the opening and the closing of a run, and they exist because a launcher knows things no provider
frame contains. Everything else is derived, including the permission decisions and the corrections,
which surprises people and is the subject of section 11.

```jsonc
// provider line
{"run_id":"...","parent_run_id":null,"seq":14,"ts":"2026-09-04T12:38:02.114Z",
 "provider":"agy","model":"gemini-3.8-flash-high","secret_flag":null,
 "raw":{"event":"step_update","step_update":{...}}}

// Hydra line
{"run_id":"...","parent_run_id":null,"seq":15,"ts":"2026-09-04T12:38:02.119Z",
 "provider":"agy","model":"gemini-3.8-flash-high","secret_flag":null,
 "hydra":{"type":"run_end","exit_code":0,"refusal":null,"body_source":"stream", ...}}
```

**Why raw survives.** The corpus exists to be trained on later and nobody yet knows which fields
that will need. An adapter bug fixed next month should fix last month's records too, and it can only
do that if the frame it misread is still there. The costs are real and accepted: a reader needs the
normalizer library, and no normalized field is greppable straight out of a file. This is the one
irreversible decision in the contract, and it is why every other mistake here is recoverable.

## 4. The envelope

Six fields on every line, plus one exception.

| Field | Type | Notes |
|---|---|---|
| `run_id` | UUIDv7 | matches the filename; repeated per line so a concatenated corpus survives |
| `parent_run_id` | UUIDv7 or null | written by the parent at spawn, never read back from the child |
| `seq` | integer | monotonic from 0 within the file, gapless |
| `ts` | RFC 3339, UTC, millisecond | when Hydra read the line, not when the provider wrote it |
| `provider` | `claude`, `cursor-agent` or `agy` | the binary, never the model's vendor |
| `model` | string | the model id as the provider reported it, or as Hydra passed it when the provider reports none |
| `secret_flag` | array or null | section 13 |

**`ts` is Hydra's clock and the provider's own stamp stays inside `raw`.** The two differ by pipe
latency and only one of them exists on all three providers: `cursor-agent` stamps `thinking` frames
with `timestamp_ms` and little else, `claude` stamps some frames, `agy` stamps none. A record whose
timestamps exist on one provider is not a record.

**`parent_run_id` is written by the parent at spawn and never reconciled against the child.** Only
`claude` has a native lineage field, `parent_tool_use_id`, and it describes in-process subagents
rather than delegation. Reading lineage back from a child would make the tree depend on the provider
that happens to be at the bottom of it.

**`seq` is gapless on purpose.** A gap means a line was lost, which is a bug worth being able to
see. It is per file, so it says nothing about ordering across a session; use `ts` for that, and
accept that two children's timestamps interleave.

## 5. `run_start`

The first line of every run file, a Hydra line, written before the child process starts. It carries
everything about the run that is true at launch and knowable only by the launcher.

A refused spawn writes this line and then `run_end`, and nothing between them (section 8.3).

### 5.1 Identity, lineage and shape

| Field | Notes |
|---|---|
| `schema_version` | integer, the contract version this file was written under (section 15) |
| `hydra_version` | the binary that wrote it |
| `launch_mode` | `recorded` or `interactive` |
| `provider_version` | the string the binary printed, read at launch |
| `provider_session_id` | the provider's own id, sometimes late and sometimes absent |
| `task` | the full task string as passed, preamble included |
| `policy` | the policy name in force |
| `policy_sha` | the sha of the delegation policy directory, pattern floor included |
| `cwd` | the directory the child was launched in |
| `worktree` | path and branch name, or null under `--shared-tree` |
| `depth` | integer, 0 for a run a human started |

**Hydra mints the run id and the provider gets told or not.** `claude` accepts `--session-id`
before launch, `agy` mints a `conversation_id` in its `init` event, `cursor-agent` needs a
round-trip. Rather than three schemes, Hydra owns the id and records the provider's own beside it,
which `hydra resume` reads to hand the provider its own resume flag. A resumed run gets a fresh run
id linked to the old one, so one file per run survives.

**`provider_session_id` may arrive after `run_start`.** On `cursor-agent` it lands on the `system/init`
frame, on `agy` on `init`. Hydra backfills the field in memory for the run it owns and does not
rewrite line 0. A reader that needs it takes it from `raw` on the provider's own init frame, which
is where it will always be.

**`task` is recorded verbatim, preamble and all.** Ticket 28 asked whether a run should record the
provenance of what it acted on and the answer was that the raw frame already carries it. The
preamble rides on the task string rather than an injected skill, so it lands here and needs no hash
of its own. A child's result is wrapped in a per-call randomised delimiter on the way back to its
parent, and that marker is in the parent's frames too.

**`policy_sha` is what makes section 7.6 work.** The permission decisions in the record are
re-derived from the policy, so a reader a month later needs the policy exactly as it was, and the
pattern floor moves independently of the five policy names. The compiled gate script's own sha is
deliberately not carried: the script is a pure function of the policy source and the Hydra version,
both of which are here, so a third sha would prove nothing the first two do not.

### 5.2 Liveness

| Field | Notes |
|---|---|
| `pid` | the child's process id, or null on a refused spawn |
| `process_start_time` | the kernel's start time for that pid |

**A run is alive when a process matching both still exists.** The start time is not optional,
because macOS recycles pids and a bare pid can point at something unrelated an hour later. No
sidecar, no lockfile, and no second source of truth to contradict D10.

**A null `pid` is a run that never had a process**, which today means a refused spawn. A reader must
not read null as dead-and-unharvested and go looking for a vendor transcript that does not exist.

**Liveness answers whether a process exists and nothing about whether it is working.** A run that is
alive and emitting nothing looks exactly like a run working hard. There is no stall detector, and
section 17 says why.

### 5.3 What Hydra injected

Three groups, each carrying a sha and a name list, and the reason is the same all three times: a sha
without names does not say what was selected, and names without a sha do not say what bytes they
resolved to.

| Field | Notes |
|---|---|
| `skills.store_sha` | the commit the run's skill snapshot was checked out at |
| `skills.linked` | the skill names Hydra linked for this provider |
| `skills.resolved` | the listing the provider itself reported |
| `mcp.doc_sha` | the sha of the canonical MCP document |
| `mcp.injected` | the servers Hydra wrote |
| `mcp.resolved` | the servers the provider reported |
| `env_allowlist` | the variable names passed to the child, never their values |

**`skills.resolved` exists because the other two lie about what the model saw.** Claude Code fits
its whole skill listing into a character budget and renders the overflow as bare names. Same
worktree and model, one run with nine injected links and one without: bare entries went from 8 to
14, and three of the six newly stripped were the user's own plugin skills. Nothing warned. So
injection changes what the model sees of skills Hydra never touched, and only the provider's own
listing records it. `claude` and `agy` both expose it for free. On `cursor-agent` it costs a paid
run or a read of the vendor's session store, so the field is optional there and a reader must handle
its absence rather than treating it as an empty listing.

**`mcp.injected` and `mcp.resolved` are identical on a recorded `claude` run and can differ
everywhere else.** `--strict-mcp-config` makes the set an argument Hydra wrote rather than the state
of five directories at spawn time. An interactive `claude` run keeps the human's servers, and `agy`
has no per-run off switch at all, so on those two the difference is the record's only account of
what the run actually got.

**These are not optional bookkeeping.** Everything Hydra generates per run lives in
`~/hydra/spawn/<run-id>/` and is reaped with the worktree, so the artifacts are gone by the time
anyone reads the record. If reading a spawn artifact were needed to interpret a run, the record
would not be the source of truth. That is the test D10 puts on this section.

**`env_allowlist` carries names and never values.** The allowlist is `HOME`, `USER`, `PATH`, `TERM`,
plus `SHELL` and `TMPDIR` for `claude`. Recording the names makes a wrong `HOME` visible after the
fact, which matters because a wrong `HOME` logs a provider out silently rather than failing.

### 5.4 Auth and attribution

| Field | Values |
|---|---|
| `auth.mechanism` | `subscription-oauth` or `api-key` |
| `auth.source` | `keychain`, `env` or `none` |
| `auth.allow_billed` | boolean, true only when a human typed the flag on this run |
| `auth.gated` | boolean, true when Hydra fully controls this provider's environment |
| `subscription` | which subscription paid |

**There is no `plane` field.** D4 says a plane names the auth mechanism and nothing else, so a
`plane` field would be a synonym for `auth.mechanism`. Pricing and interactivity are a second axis
that cuts across both planes.

**`auth.gated` costs nothing in v1 and is not therefore pointless.** Every routed provider qualifies
today, because the one that could not be gated stopped being routed for that reason among others. It
exists so a run that was actually gated is distinguishable from one where Hydra could not enforce
the gate, rather than something a reader infers from which provider ran. A provider Hydra cannot
gate is either given a config root Hydra owns or is not routed; it is never launched with the gap
recorded. The field is what makes that rule checkable.

**`subscription` names the payer, and Hydra authors it at spawn.** `agy` serves Claude Opus 4.6 on
the Google subscription, so `provider` stopped answering which vendor's model replied and the two
questions came apart. `claude` answers the question three ways, through `apiKeySource`, through
`modelUsage[*].provider`, and through whether `rate_limit_event` appears at all. `agy` answers it
nowhere on the stream. Hydra knows which binary it spawned and which subscription that binary is
logged into, so authoring the field is the only version that answers identically on all three.

**One field, not two.** The model's vendor stays derivable from `model`, since every id on all three
providers names its vendor in its first token: `claude-`, `gemini-`, `composer-`, `gpt-oss-`. A
vendor field would be a second exception to section 10's rule against stored derived fields, bought
for nothing.

**No account email, anywhere.** Reading `agy`'s CLI log for it was considered and rejected. It is
worse than absent, because it pulls an address into the record that export would then have to scrub,
and Hydra would have chosen to put it there.

## 6. `run_end`

The last line of a run file, a Hydra line, and **it is not guaranteed to exist**.

| Field | Notes |
|---|---|
| `exit_code` | the child's exit status, or null when no process ran |
| `error` | text Hydra captured that is not on the stream, or null |
| `refusal` | the Hydra-side check that refused, or null (see below) |
| `remediation` | the string that tells a human what to fix, or null |
| `body_source` | `stream` or `harvest` (section 12) |
| `ended_at` | RFC 3339 |

**`outcome` is not stored here.** It is derived at read time from this line and the stream, like
every other normalized value, and for the same reason: an adapter that misclassifies a run today
gets fixed next month and last month's records reclassify on the next read. An outcome written into
the file would be wrong forever.

**`refusal` is the exception that makes that work.** Four outcomes are things Hydra decided rather
than read: a spawn refused for naming a wider policy, a skill name collision at launch, an MCP
launch check or Cursor pre-flight that failed, and an auth pre-flight that failed. There is no
stream to derive those from, so `refusal` names the check and the derivation reads it as the
strongest input it has. The other outcomes come from the stream.

**Process exit is the authoritative end of a run, and the provider's own terminator is optional
enrichment.** `claude` always writes exactly one `result` frame, `cursor-agent` writes one only on
success, and no contract can be built on a frame that two of three providers withhold on the path
that matters most. So `run_end` is Hydra's line, written when `wait()` returns.

**A missing `run_end` is a fact rather than a corruption.** If Hydra dies, nothing writes it. A dead
pid with no `run_end` is exactly ticket 14's abandoned-run class, and it is derivable only because
section 5.2 put the pid in the file before the child started. Export drops these, because a
trajectory with no ending teaches a model to stop mid-tool-call.

**Ownership transfers to whoever proves the previous owner is dead.** The rule that nothing appends
to a run file except its owner is real, and taken literally it would forbid the harvest that
completes an abandoned interactive run. The precise rule is that there is at most one writer at a
time, and a process may take the file only after the liveness test in section 5.2 says the recorded
pid is gone. The child never touches the file under any reading. This reconciliation is made here
rather than inherited, because the two sentences it comes from were written in the same ticket and
do not survive being read side by side.

**`error` and `exit_code` live here rather than only in what `hydra run` returns.** Ticket 11 put
`exit_code`, `status`, `status_source` and `error` in the JSON an orchestrator gets back and ticket
09 put none of them in the file, which under D10 means the JSON carried facts the source of truth
did not have. Section 17 makes the JSON a projection of this line instead.

## 7. The ten normalized event types

Ten, versioned, with the version in `run_start`. The list stays small on purpose: raw frames survive
under section 3, so adding a type later is cheap and getting one wrong is recoverable.

### 7.1 The producer column

**Every type names the frames it derives from, per provider.** This is the column that makes the
contract checkable rather than aspirational, and it does three jobs.

It stops the contract lying. An event type nobody writes is a promise to whoever builds against it,
which is why `permission_request` is not on the list any more: it populated from ACP and from
`claude`'s `can_use_tool`, no routable provider speaks ACP, and every recorded run launches with the
permissive flag plus the gate, so the decision is made before the process starts and nothing asks
Hydra for permission during a run.

It carries the disagreements. `thinking` has three answers by provider, and `usage` is scoped to a
run on two providers and to a conversation on the third. Those belong in a column beside the type
rather than in prose a reader has to assemble.

And it is the instrument for the seam check, which asks whether every field Ouroboros reads is one
the harness actually writes. With this column that is a table lookup.

### 7.2 The table

| Type | Kind | `claude` 2.1.257 | `cursor-agent` 2026.09.02 | `agy` 1.1.26 |
|---|---|---|---|---|
| `run_start` | Hydra | always | always | always |
| `run_end` | Hydra | on exit | on exit | on exit |
| `message` | derived | `assistant` and `user` frames, text content blocks | `assistant` frames, plus the echoed `user` frame | `step_update`, `step_type` `agent_response` or `user_input` |
| `message_delta` | derived | **none in v1**, lit by `--include-partial-messages` | **none in v1**, lit by `--stream-partial-output` | **none in v1**, and no flag would light it |
| `thinking` | derived | `thinking` content blocks, with `signature` | `thinking/delta` frames, no flag needed | **never text**, count only, see `usage` |
| `tool_call` | derived | `tool_use` blocks on `assistant` frames | `tool_call` frames, `subtype: started` | `step_update`, `step_type: tool` |
| `tool_result` | derived | `tool_result` blocks on `user` frames, joined on `tool_use_id` | inside the `tool_call` object on `subtype: completed` | `step_update`, `step_type: tool`, `state` `DONE` or `ERROR` |
| `permission_decision` | derived | one per `tool_call`, from `policy` and the call | same | same |
| `usage` | derived | `result.usage` | `result.usage`, when present | every `step_update` at `DONE`; `result` is per conversation, section 9.2 |
| `correction` | derived | interactive harvest only | interactive harvest only | interactive harvest only |

### 7.3 `message`, and the `agy` field that is not a delta

On `claude` and `cursor-agent` a message arrives whole. On `agy` it arrives in a field called
`text_delta` on one or more `step_update` frames sharing a `step_index`, and **the normalizer
concatenates them over that `step_index` to produce one `message`.**

**The field name invites exactly the wrong reading and the frames do not support it.** Measured on
1.1.26 on 2026-09-04: a prompt asking for the numbers one to ten, one per line, produced the entire
ten-line body in a single `ACTIVE` frame's `text_delta`, followed by a `DONE` frame whose
`text_delta` was the trailing newline. That is a buffered flush wearing a streaming name, the same
shape `cursor-agent` has without `--stream-partial-output`. So `agy` does not stream token by token,
and a reader that treats each `text_delta` as a partial token is building on one sample of two
chunks that happened to split at a newline.

`message` carries `role`, `text`, and the `step_index` or content-block index it came from, so the
concatenation is reproducible from `raw` rather than trusted.

### 7.4 `message_delta`, dark with a reason

**No producer in v1, on any provider.** Both partial-output flags are off: recorded runs are
detached and unwatched, `hydra logs --follow` reads whole lines, and no export format wants
intra-message granularity.

That is the same position that cost `permission_request` its slot, and the distinction that keeps
this one is narrow but real. No provider *can* produce a permission request any more, while two ship
a flag that would light message deltas tomorrow. `agy` is the odd one: it ships no such flag and
would not light this type if it did, because section 7.3 shows its `text_delta` is a flush.

Three consumers want this type and all three are deferred: the TUI, a live `--follow`, and a stall
detector. Whoever turns the flags on should expect the type to light on two providers and not the
third.

### 7.5 `thinking`, where text and tokens come apart

**Reasoning text and reasoning tokens are two capabilities and no provider gives both.**

| Provider | Text | Count |
|---|---|---|
| `claude` | `thinking` content blocks, with `signature` | `result.usage.output_tokens_details.thinking_tokens` |
| `cursor-agent` | `thinking/delta` frames, no flag | none on the stream |
| `agy` | none | `usage.thinking_tokens`, per step and on `result` |

So `claude` gives both, `cursor-agent` gives text alone, `agy` gives a count alone. The `thinking`
event carries text and the count rides on `usage`, which keeps a provider that reports one from
implying the other.

**`cursor-agent`'s text is provider-reported reasoning and the record claims no more.** Whether it
is the model's own trace or a provider summary is not answerable from the CLI, and `model` reports
the display name `Auto` rather than an id. Its binary carries a `reasoningTokens` field internally
and the print-mode writer drops it, so this is a gap that could close on any self-update.

**`agy`'s count is wired, at least on Google ids.** A trivial 1.1.26 probe on the default model
reported `thinking_tokens: 172` with no reasoning text anywhere on the stream. Ticket 20 saw
`thinking_tokens: 0` on a Claude id and left open whether the field is unwired there; this narrows
that question to Claude ids rather than answering it.

`claude` also streams a live estimate as `system/thinking_tokens` with `estimated_tokens`, which its
own schema calls approximate progress rather than billed tokens. It stays in `raw` and produces no
`usage` event.

### 7.6 `tool_call` and `tool_result`

**The three providers describe tool traffic in three incompatible shapes**, and mapping them onto
one is the harness's tool-call adapter rather than a schema decision. What the contract fixes is the
shape the adapter must produce and what it is allowed to refuse.

`tool_call` carries `call_id`, `name`, `arguments` and the capability Hydra's own table maps the
name to. `tool_result` carries the same `call_id`, a `status` of `ok`, `error` or `unclassified`,
the output, and a truncation marker when the output was cut.

**The join key is the hard part on two of three.** `claude` gives a real one, `tool_use_id`, on both
sides. `cursor-agent` puts the result inside the `tool_call` object on the `completed` frame, so the
two events come from one frame and the adapter mints the pairing. `agy` puts both on `step_update`
with `step_type: tool`, distinguished by `state`, so the pairing is `step_index`.

**The adapter may return `unclassified` and that is a feature.** `agy` gives a permission denial and
a file-not-found the same `state: ERROR` and the same `error.type: TOOL_ERROR`, and both messages
contain the word "permission", so a substring rule is wrong rather than weak. An adapter that says
it cannot tell keeps the gap visible per provider instead of averaged away. Guessing here produces a
curator that rewrites a skill because a file was missing.

**Nothing upgrades an `unclassified` result to `ok`.** All three providers close a failed run
reporting success, so a label read off the terminator would be a lie.

**A truncated result carries a visible marker inside its string.** The alternative teaches a model
that a cut-off output was the whole output.

**`agy`'s hook `overwrite` is forbidden and the reason is this section.** Its `PreToolUse` can
shallow-merge into a call's arguments so a rewritten call is what runs. A rewritten call means the
record no longer shows what the model actually asked for, and rung one learns from what agents
tried, so a silently corrected trajectory would teach the curator that the agent got it right.

### 7.7 `permission_decision`

One per `tool_call`, derived, carrying `call_id`, `decision` of `allow` or `deny`, `decider` and
`reason`.

**The decider is derived from the run's own policy, never from the provider's reason text.** Hydra
re-evaluates the policy recorded in `run_start` against the call recorded in the stream. That is the
only mechanism available: calling back into `hydra` per tool call is ruled out, the gate script
cannot append to the run file, and the spawn directory the gate lives in is reaped with the
worktree.

**It says which policy accounts for the refusal, not which process fired.** That distinction is the
honest limit and it exists because the reason text cannot carry the field. On `agy` the first deny
short-circuits, so Hydra's hook may never run and the reason belongs to whichever hook did. On
`cursor-agent` every hook's message is concatenated, and one probe reached the model as
`Rejected: t30 CUR-PROJ says allow`, quoting the hook that allowed. A field parsed out of that is
worse than no field.

So `decider` is `hydra_policy` when the run's own policy refuses the call, and `external` when the
call was refused and the policy would have allowed it. `external` means something outside Hydra's
policy denied, which on this machine means the user's own 418 lines of `PreToolUse` guards, and on
`agy` means nothing today because no user-scope hook config exists for it. That is a fact about this
machine rather than a property of Hydra, and it changes the day someone writes one.

**Provider reason text is recorded verbatim in `raw` and is corroboration only**, with three
different strengths worth knowing before anyone leans on it:

| Provider | What the stream says about who denied |
|---|---|
| `claude` | `--include-hook-events` gives `hook_started` and `hook_response` frames naming the hook, with `exit_code`, `outcome` and stdout. Strong, and independent |
| `agy` | the tool error names the hook by a generated id, `jsonhook__<name>_<event>_<group>_<handler>`, and carries Hydra's own reason string verbatim. Strong, with the short-circuit caveat above |
| `cursor-agent` | every hook's message concatenated. Not usable for attribution |

**A disagreement between the derived field and `claude`'s hook frames is a finding, not noise.** It
is the only place in the record where two independent sources answer the same question.

**The re-derivation gives the record a gate-leak check for free.** Every tool call is evaluated
against the policy at read time, so a call the provider allowed and the policy denies is a gate that
leaked. `claude`'s result frame gives a second instance directly: `subagent_stats.spawned` above
zero under a policy that denies in-process subagents means the same thing. Neither reopens the
decision to deny subagents, because a count is not a trajectory; both make the denial checkable,
which it otherwise would not be.

### 7.8 `usage`

Carries the parts and its provenance. Section 9 has the field mapping and the two traps in it.

| Field | Notes |
|---|---|
| `usage_source` | `provider`, `estimated` or `unavailable`, decided per run |
| `usage_estimator` | which tokenizer ran, null unless `usage_source` is `estimated` |
| the counts | section 9 |

**Never a bare null.** Null cannot be told apart from a provider reporting zero, and a silent
estimate is worse than either. This is the field that keeps a rough number from becoming a fake
measurement in a training set.

**`usage_source` is decided per run, not per provider.** `cursor-agent` reports counts on its
`result` frame in both JSON output formats, on the success path and the tool-rejection path, so it
is `provider` most of the time. But its print-mode `catch` prints to stderr and exits 1 without
writing a `result` frame at all, which is every hard failure and every kill by design, and its
emitter appends the block conditionally even on success. So `usage` is optional on the success path
too, and a run that ends with no counts falls to `estimated`.

**`agy` reports usage twice and the two agree only on a first turn.** Once on the step's `DONE`
frame and once on `result`. On a resumed conversation the `result` figure covers every turn in the
conversation rather than this run, which section 9.2 measures.

### 7.9 `correction`

Derived from harvested frames, carrying the `seq` of the `tool_call` or `tool_result` it corrects,
the rejected content, and the chosen content. The vendor transcript entries it comes from are stored
verbatim like any other frame, so the sweep in section 13 reads them and never-discard covers them.

**One producer in v1: a human correcting a tool call in an interactive run, recovered at harvest.**
The pair has to sit inside the same trajectory for preference-pair export to work, and a sidecar
file would be a second thing to keep in sync for no gain.

**A policy denial is not a correction, and this is written down rather than left to judgement.**
`claude` labels a headless auto-deny `user-rejected` exactly as it labels a real one. In a recorded
run there is no human to reject anything, so `user-rejected` there is Hydra's own policy answering,
and training on it teaches a model to obey a config that changes. A pair recorded today would be
wrong the day the policy widens. Interactive plus `user-rejected` is a person; recorded plus
`user-rejected` is the gate, and Hydra knows which mode it launched.

Rung one's merged-or-dropped proposals and rung two's panel verdicts carry preference signal in a
different shape, a diff plus a verdict rather than a trajectory plus a turn. They are recorded as
ordinary runs and are not forced into this event.

## 8. Run outcomes

The enum belongs to this document. The harness spec lists the same nine and says so.

**An outcome is a derived value, never a stored one** (section 6). What follows is the vocabulary the
normalizer produces, not a field to look for in a file.

| Outcome | Means | Rung one |
|---|---|---|
| `completed` | the task ran to an end | learns |
| `failed` | the task ran and failed | learns |
| `timed_out` | the watcher gave up; the child may still be running | learns |
| `crashed` | the process died | learns |
| `gated` | the gate refused, or a spawn was refused for naming a wider policy | never |
| `auth_failed` | not logged in, or the login lapsed mid-run | never |
| `skill_collision` | a linked skill name the provider did not resolve as Hydra's | never |
| `mcp_unavailable` | the launch check or the Cursor pre-flight refused | never |
| `rate_limited` | the provider refused its own limit | never |

**Recorded and learned-from are different questions.** The right-hand column says what rung one
learns from and never what Hydra keeps. Every run in this table is recorded in full, verbatim,
whatever its outcome. A reader who collapses "rung one never learns from this" into "throw it away"
has the record backwards.

**Every outcome is derived from the event stream, never from the exit status.** All three providers
exit 0 and report success on a run where the gate refused everything. `claude` closed
`terminal_reason: completed, is_error: false` with a real denial sitting in `permission_denials`.
`agy` returned `status: SUCCESS`, an empty response and exit 0 on a run whose only tool was
auto-denied. `cursor-agent` returned `is_error: false` with one tool rejected and one failed.

**A rate-limited `claude` run reports `subtype: "success"` with `is_error: true` and
`api_error_status: 429`.** A recorder keying off `subtype` files an exhausted run as a clean one.
This is the single nastiest shape in the contract, because it is the one that looks correct.

### 8.1 `denied` against `gated`, settled

Ticket 11's status enum carried `denied`. Ticket 27 later minted `gated` for the same situation and
the harness spec listed `gated` alone, leaving the enum to this document.

**`denied` is dropped.** The two named one thing, `gated` is the more accurate name for it, and a
taxonomy with two labels for one state is a bug with a schema. `denied` also read as a per-tool-call
fact, which is `permission_decision`'s job in section 7.7, so keeping both invited a reader to file
a single refused call as a run outcome.

### 8.2 `outcome_source`

How far the outcome can be trusted. Three values, and they are three genuinely different levels of
confidence rather than shades of one.

| Value | Means |
|---|---|
| `refusal` | Hydra refused the launch itself, so the outcome is a fact it wrote rather than a reading (section 6's `refusal` field) |
| `derived` | the adapter classified the run from the stream |
| `unknown` | the adapter could not, because the run holds a tool result it returned `unclassified` for and nothing else settles it |

**`provider` is dropped, and this is decided here rather than inherited.** Ticket 11 gave the field
three values, mirroring `usage_source`, and `unknown` earned its place on hermes, which exits 0 on a
failed turn. Hermes is not routable, but `unknown` survives it because the adapter's `unclassified`
gives it a new producer.

`provider` does not survive. It meant the provider's own terminator was trustworthy, and on the
current three it never is: `cursor-agent`'s `result.subtype` is only ever `"success"` in the shipped
code, `agy` reports `SUCCESS` on a fully denied run, and `claude` reports `success` on a 429. **No
routable provider produces a run-level status worth reading**, so the value has no producer, and
section 7.1's rule applies to enum values as much as to event types. It comes back the day a
provider ships an honest terminator, and nothing else has to change for that.

### 8.3 A refused spawn is a run record

A child refused for naming a policy wider than its parent's writes a run file: `run_start`, then
`run_end` with `refusal` naming the narrowing check, and nothing between them. It derives to outcome
`gated` with `outcome_source: refusal`.

The `run_start` carries the requested policy and the parent's beside it, `pid` and
`process_start_time` null, and no provider fields. No child process starts, so without the record
the attempt is invisible, and under D10 anything not in a run record did not happen. Attempted
escalation is exactly the signal the corpus should carry.

### 8.4 Provider-specific fields stay in `raw`

Three `claude` fields have readers and none of them becomes a normalized field, because a normalized
field that exists on one provider is a shape that changes with the provider, which is what the whole
normalization layer exists to prevent. The contract names their paths so nobody rediscovers them:

| Field | Path in `raw` | Who wants it |
|---|---|---|
| `permission_denials[]` | `result.permission_denials` | corroborating section 7.7 |
| `subagent_stats` | `result.subagent_stats`, with `spawned`, `max_depth`, `spawned_by_subagents` and a `refused` breakdown | the gate-leak check in section 7.7 |
| `rate_limit_event` | its own frame, with `rateLimitType`, a `status` of `allowed`, `allowed_warning` or `rejected`, a `resetsAt` epoch and a `utilization` fraction per window | rung two's early abort |

**The rate-limit reading drives no routing.** It appears on every `claude` turn at normal
utilization, and rung two reads the last run's `status` to abort early rather than spend two other
judges' quota on a review that cannot reach quorum. Nothing routes on it. Nothing should key on
`utilization`, which was caught moving backwards, 0.24 to 0.05 on `seven_day` against an unchanged
`resetsAt`. `resetsAt` gives any reading a free staleness test.

## 9. Usage, and the two traps in it

The three providers report token counts in three field vocabularies. They agree on the arithmetic,
which was not what the closed tickets said, and they disagree on what a count is scoped to, which no
ticket noticed at all.

| Normalized field | `claude` | `cursor-agent` | `agy` |
|---|---|---|---|
| `input_tokens` | `usage.input_tokens` | `usage.inputTokens` | `usage.input_tokens` |
| `output_tokens` | `usage.output_tokens` | `usage.outputTokens` | `usage.output_tokens` |
| `cache_read_tokens` | `usage.cache_read_input_tokens` | `usage.cacheReadTokens` | `usage.cache_read_tokens` |
| `cache_write_tokens` | `usage.cache_creation_input_tokens` | `usage.cacheWriteTokens` | absent |
| `thinking_tokens` | `usage.output_tokens_details.thinking_tokens` | absent | `usage.thinking_tokens` |
| `total_tokens` | derived | derived | `usage.total_tokens`, and see below |

### 9.1 All three report input net of cache, so store the parts and derive the total

`raw_input = input_tokens + cache_read_tokens + cache_write_tokens`, over whichever of the three a
provider reports.

**No provider reports full input with cache read beside it.** Ticket 25 recorded two conventions,
`agy` reporting full input and `cursor-agent` subtracting, and put the split in ticket 09 as
something the normalized usage event had to carry. It is wrong, and it was made on a sample where
`agy` reported `cache_read_tokens: 0`, which cannot distinguish the two readings.

Measured on `agy` 1.1.26, 2026-09-04. A fresh run reported input 14,304, output 191, cache read 0,
total 14,495. Resuming that conversation reported, on the step frame, input 6,577, output 193, cache
read 8,128, total 6,770. **`total_tokens` is `input_tokens + output_tokens` and excludes cache read
entirely**, and 6,577 plus 8,128 reconstructs a second-turn prompt of the right size. `agy`
subtracts, like the other two.

`cursor-agent` was already proven from outside its binary: two runs of one identical prompt reported
14,629 and 11,557 input, both summing to 16,933 with their cache reads. `claude` follows Anthropic's
API convention, and one observed frame settles it beyond argument, since a real prompt reported
`input_tokens: 2` alongside `cache_creation_input_tokens: 10558`.

**So there is no `usage_convention` field.** One convention with one value is not a variable, and a
field that can only say one thing invites a reader to believe it could say another. The reconstruction
rule above is a contract rule instead. If a provider ever diverges, the normalizer branches on
`provider_version`, which section 5.1 puts on every `run_start` and section 15 explains, and the
field comes back then.

**Never copy `cursor-agent`'s `inputTokens` into a total.** Its emitter computes
`max(inputTokens - cacheReadTokens - cacheWriteTokens, 0)`, so a warm cache reports 0 input on a turn
that consumed tens of thousands. A normalized field that copies it is wrong in the direction that
looks like a cheap run.

**`cacheWriteTokens` read 0 in all seven `cursor-agent` captures across two tickets and never
moved.** Worth knowing before anything keys a decision on it.

### 9.2 `agy`'s `result` usage is per conversation, not per run

**A resumed `agy` run reports its predecessor's tokens as its own.** In the probe above, the second
run's `result` frame carried input 20,881, output 384 and thinking 336. Those are the sums across
both runs: 14,304 plus 6,577, 191 plus 193, 172 plus 164. The step frame for that run alone carried
6,577, 193 and 164.

Hydra mints a fresh run id per resume, so the two runs are two files. Taking usage from `result` on
the second would count the first run's tokens twice in any corpus that sums them.

**So on `agy` a run's usage is the sum of its own `step_update` frames, and `result.usage` is
recorded and not used for that.** The record keeps both, because per-step usage is the only place a
multi-turn run's cost is attributable to a step, and because a discrepancy between the two is how a
resumed run announces itself.

Whether `claude` and `cursor-agent` do the same on their own resume paths is unmeasured. Neither is
resumed by anything in v1 except a human typing `hydra resume`, and both report usage only on
`result`, so on those two the question has no cheap answer and no current reader. It is on the map
as fog rather than assumed either way.

## 10. Lineage, and what a tree answers

`parent_run_id` builds the tree and three questions fall out of it that nothing else answers.

**Depth** is the distance to a run with a null parent, computed at spawn with no extra bookkeeping,
and capped at 2. The cap has a known blind spot: an in-process subagent gets no run id and writes no
run file, so it does not count. Every policy but `permissive` denies those spawns, which is what
makes the cap mean what it says.

**A retry loop** is a shape in the tree rather than a field, which is why nothing records one.

**Who reviewed a patch** is a tree of three judge runs under one parent. Rung two's judges are three
`hydra run` calls, so a judge's identity is a provider version plus whatever model that subscription
served that day, and neither is pinnable: `agy` moved four times in the life of this design. Section
5.1 already puts `provider_version` and `model` on every `run_start`, so the requirement is met by
the ordinary record rather than by anything review-specific. Without it, "which model approved the
commit that broke this" has no answer a month later.

**Runs whose parent is the curator are excluded from two corpora**, the one the curator reads and
the one export produces, for the same reason both times. That is a rule the Ouroboros spec owns; the
record's job is that `parent_run_id` makes it computable.

## 11. Stored against derived

**Normalization is a read-time library function.** The ten event types in section 7 exist in the
reader, not on disk. Nothing writes a normalized field into a run file, with the exceptions below.

**The two Hydra lines are not an exception to this.** `run_start` and `run_end` carry facts no
provider frame holds, so there is nothing to derive them from and storing them is the only option.
Nothing else qualifies. `permission_decision` looks like it should, because Hydra authors the
decision, and it does not: the policy is in `run_start` and the call is in `raw`, so the decision is
a function of two things already on disk. `correction` looks like it should, because a human made
it, and it does not: the vendor's transcript recorded it and harvest stores those frames verbatim.

**A run's outcome is derived too, and that is the sharpest case.** It is the field most readers
want stored, and storing it would freeze today's classification into a corpus meant to be read for
years. Section 6's `refusal` field is the narrow exception, and it is narrow because Hydra refusing
a launch is a fact rather than a reading.

**One real exception, `secret_flag`, and it is deliberate.** Section 13 sets it on the write path
from a scan of the frame it is wrapping, which is a derived value stored beside the thing it was
derived from. It holds because a rescan reproduces the flag exactly, making it a cache rather than a
fact, and because what it buys cannot be had at read time: a live credential sitting in `runs/`
becomes visible the day it lands rather than at the first export. It also covers harvested bodies,
which never pass the write path at all.

**Derived-at-read-time is what makes adapter bugs cheap.** An adapter that misreads `agy`'s
`step_type: tool` today is fixed next month and last month's records normalize correctly on the next
read. Nothing is migrated, because nothing was written.

## 12. Streamed against harvested

`run_end.body_source` says which, and the two are not the same evidence.

**`stream`** means Hydra tee'd the provider's frames as they arrived, which is every recorded run.

**`harvest`** means Hydra converted the vendor's own transcript into the record after the process
exited, which is every interactive run. A vendor CLI keeping its own TUI cannot also emit a
structured stream, so an interactive run is recorded late rather than not at all.

**A harvested body arrives all at once**, so anything reading records has to cope with a run file
that was `run_start` and nothing else a moment earlier. A run that looks empty may simply not have
been harvested yet, and that is not the same as a run that produced nothing.

**Harvest is idempotent and any process may do it**, subject to section 6's ownership rule. The
envelope names the provider and the provider's session id, so harvest is a pure function of two
files on disk and does not care who runs it or when. `hydra status` finds a dead run with an
unharvested body and completes it.

**Only `claude` can be harvested live**, because it is the only one confirmed to write an
append-only log during the run. A contract three providers cannot meet is not a contract, so
harvest-at-exit is the contract and live tailing is a per-provider optimisation.

## 13. Never discard

**Nothing is dropped, reordered or rewritten at write time.** The raw frame is stored verbatim. This
is the rule the rest of the contract leans on, and it has exactly one exception and one addition.

**The exception: Hydra strips its own injected values.** The exact string Hydra put in the child's
environment for that run, which in v1 means `GEMINI_API_KEY` on a run carrying `--allow-billed` and
nothing at all otherwise. Hydra knows the value at injection time, so it can promise to strip it and
keep the promise. The leak this closes has nothing to do with where the key is stored: an agent runs
`env`, the provider streams that output back as a frame, and frames are written verbatim.

**The addition: `secret_flag` detects and never rewrites.** A prefix sweep for `AIza`, `sk-ant-`,
`ghp_`, `ya29.` and PEM headers runs on the write path and again at harvest. A hit puts the matched
class names on the envelope and leaves the frame untouched.

Everything else stays on disk and is cleaned at export. `~/hydra/runs/` sits on this laptop under
this user account, next to the source and the dotfiles that already hold those credentials, so
encrypting at rest would add ceremony without changing who can read it. Export is what leaves the
machine, and that is where redaction earns its keep.

**The bill for raw, stated plainly:** storing frames verbatim means storing file contents,
environment variables, and whatever an agent happened to read. A recorded `claude` run also carries
the stdout of the user's own hooks, 418 lines of Python that can print anything, which arrives
through `--include-hook-events`. None of that is a reason to discard. It is the reason export scopes
by path before it scrubs by pattern.

## 14. What is deliberately not recorded

| Not recorded | Why |
|---|---|
| A normalized event type, on disk | section 11; the reader derives them |
| `plane` | D4 says a plane names the auth mechanism, so it would be a synonym for `auth.mechanism` |
| The model's vendor | derivable from `model`, whose first token names it on all three providers |
| Any credential value, prefix or length | `auth.*` says the mechanism and the source and stops |
| The account email | it would create work for the redactor that Hydra chose to create |
| A diff stat | derivable from the branch, and the file writes are already in the `tool_call` events |
| A heartbeat, a liveness event, a death event | liveness is derived from `pid` and `process_start_time` by a reader |
| A run's own outcome, before it has one | there is no `status: running` field; a run with no `run_end` and a live pid is running |
| A stored session tree | rebuilt from `parent_run_id` at read time |
| In-process subagent work as its own run | it gets no run id and writes no file, which is why every policy but `permissive` denies it |

**The last row is the one with a cost.** An in-process subagent is work with no record at all, on the
exact trajectory Ouroboros learns from, and denying the spawn is the only way to keep D10 true. A
delegated `claude` child therefore cannot use `Task`, which is a real capability loss on coding runs.
`--forward-subagent-text` is passed anyway, so that under `permissive`, where the spawns are allowed,
their text is at least on the stream.

## 15. Versioning

**`schema_version` is an integer on `run_start`**, and it versions the normalized vocabulary rather
than the file format. The envelope's shape has not changed and is not expected to.

**Adding an event type is a minor change and needs no migration**, because raw frames survive: a
reader on version N+1 derives the new type from records written under version N. That is the whole
reason section 7 keeps the list small. Adding one costs nothing later; carrying one nobody writes
costs a reader's trust now.

**Removing or repurposing a type is a breaking change** and increments the version. Removing
`permission_request` before v1 shipped cost nothing because no file exists.

**A provider version change is not a schema change.** `provider_version` is on every `run_start` for
exactly this: a normalizer can branch on it, and a claim measured on one build does not silently
become a claim about another. Given that two of three providers self-update without asking, this is
the field that keeps the corpus interpretable across a year of drift.

## 16. The export test (D12)

D12 keeps training out of scope and makes lossless export a standing constraint on the record. This
section is the test, and it names what "without loss" means here, because the naive reading is
wrong.

**Losslessness means every slot a training format has is either filled from the record or left
empty, and nothing is invented to fill one.** It does not mean every recorded field crosses. Several
deliberately do not, and a record that forced them across would be worse.

### 16.1 The mapping

Two files from one traversal, in TRL's format and the explicit-prompt preference format.

| Record | `sft.jsonl` | `preference.jsonl` |
|---|---|---|
| `message` | a message, `role` and `content` | part of `prompt` |
| `thinking` text | the `thinking` key, where the provider gave text | part of `prompt` |
| `tool_call` | assistant message `tool_calls`, arguments as a dict | part of `prompt` |
| `tool_result` | a `tool` message joined on `tool_call_id`, truncation marker inside the string | part of `prompt` |
| `correction` | nothing | the pair: `rejected` and `chosen` |
| `permission_decision` | nothing | nothing |
| `usage` | nothing | nothing |
| `run_start`, `run_end` | nothing | nothing |
| `run_id` | nothing | nothing |

**TRL rather than the OpenAI or ShareGPT shapes, and the middle reason is the record's.** OpenAI's
preference format is single-turn and text only, so a `correction` at turn 40 of a tool-heavy
trajectory does not fit it. OpenAI also encodes tool arguments as a JSON string, so a redactor
walking JSON values cannot see a path or a key inside one without parsing the nested string, which
would manufacture a class of secret the redactor structurally misses; TRL's dict puts every argument
at a value the walk already visits. And ShareGPT has no `tool_call_id`, so a trajectory with two
calls in flight cannot be reassembled. Emitting OpenAI-shaped files later is a mechanical conversion
from TRL, and the reverse is not.

**`thinking` is absent rather than fabricated where a provider gives no text.** An `agy` line
carries no thinking key. Inventing one from `thinking_tokens` would put a number where a format
expects prose.

**`usage` crosses nowhere, and no sidecar carries it.** That is what makes section 7.8's provenance
guarantee hold by construction: an `estimated` count cannot be read downstream as a measurement if
it never leaves.

**`permission_decision` does not cross**, and its effect does. The model saw a tool result carrying
an error, and that result crosses as part of the trajectory. The decision event carries policy
attribution, which is Hydra's bookkeeping rather than something the model experienced.

**The training line carries no run id.** The manifest holds the line-to-run map, so traceability
survives and the UUIDv7's embedded timestamp stays in a file no training run reads.

### 16.2 What the record has to answer for export to run at all

**Every path the run touched**, extracted through the tool-call adapter. A run holding any tool call
the adapter returned `unclassified` for is not exportable, so a run with `outcome_source: unknown` is
outside the corpus by the same rule. An empty corpus beats one holding a run whose reads were
invisible.

**Every path every run beneath it touched**, because a parent's `tool_result` can carry a child's
summary of work done in another worktree. Without the subtree check, delegation walks content out of
an excluded workspace and into the corpus.

**Its own outcome**, because three exclusions key on it: `auth_failed` runs, in which no task
happened; runs with no `run_end`, whose trajectory has no ending; and runs whose parent is the
curator. Everything else exports with its outcome attached, retry loops included. The exporter
judges validity and never quality, because two of three self-edit proposals are blocked in the
system this design borrows from, so a corpus of successes would describe a state Hydra rarely
occupies.

### 16.3 Where the test currently fails

**`preference.jsonl` has one source and that source is unverified.** `correction` populates only
from a human correcting a tool call in an interactive run, recovered at harvest, and nobody has
established that any of the three vendor transcripts records the rejected and chosen contents in a
form that survives conversion. If none does, the preference half of D12 has no producer and section
7.9 should lose its slot by section 7.1's own rule. This is filed rather than hedged, as ticket 34.

## 17. What `hydra run` returns

The JSON an orchestrator gets back is a projection of the record, plus exactly one field that is
not in it, named below.

`run_id`, `outcome`, `outcome_source`, `exit_code`, `error`, the final assistant text, the worktree
branch, a diff stat, `usage` with its `usage_source`, and the run file path.

**The result stays small on purpose.** Delegation exists to keep a child's trajectory out of the
orchestrator's context, and a result that returned the whole trajectory would defeat the feature it
implements. `hydra show <run-id> --events` is how an orchestrator asks for more, explicitly.

**The diff stat is the one field with no record equivalent**, computed from git at return time. It
is not a record field because it is derivable from the branch while the branch exists, and because
the file writes it summarises are already in the `tool_call` events, which is the version that
survives after a branch is deleted.

**On the MCP face the child's text is wrapped in a boundary whose delimiter is randomised per call**,
picked after the child has returned so the child cannot close its own envelope. The marker is in the
parent's frames, so the record shows the wrapping rather than only the text.

## 18. Open questions this contract does not close

Each of these is a ticket rather than a paragraph.

### 18.1 The Cursor estimator has nothing to key on

`usage_source: estimated` was specified as a model-aware tokenizer picked by model family, on the
grounds that `cursor-agent` reports `model` on its `system/init` frame. It reports the display name
`Auto`, not an id, so the family the estimator is supposed to select on may not be knowable from the
record at all. Every `cursor-agent` run that fails hard lands here, because its print-mode `catch`
writes no `result` frame. Filed as ticket 33.

### 18.2 `correction` has an unverified producer

Section 16.3. Filed as ticket 34.

### 18.3 What a resumed run's usage is scoped to on two providers

Section 9.2 measured `agy` reporting a whole conversation's tokens on a resumed run's `result` frame.
`claude` and `cursor-agent` report usage only on `result` and neither has been resumed under
measurement, so whether they do the same is unknown. Nothing in v1 resumes a run except a human
typing `hydra resume`, and usage crosses nowhere into export, so the exposure today is that
`hydra run`'s returned figure could be wrong for a resumed run on two of three providers.

### 18.4 No stall detector, so a wedged run is `completed` or nothing

Liveness is a pid and a start time, so a run that is alive and emitting nothing is indistinguishable
from a run working hard, and none of the outcomes in section 8 covers it. Message deltas are the
cheap heartbeat and section 7.4 turns them off for reasons that hold on their own. `agy` keeps a
heartbeat by accident, emitting `step_update` with `state: ACTIVE` and shipping no flag to stop it,
so the signal exists on one provider of three. Left unbuilt on this design's own principle: Hydra has
run zero times and a timeout invented now is a guess with a decimal point on it. On the map as fog.

### 18.5 Nothing reads the policy signal

`gated` runs carry a signal about the delegation policy rather than about a skill, and a run that
failed on too narrow a policy is a policy bug. The record holds it in full and nothing in v1 reads
it. This breaks no seam, because D5 requires every field Ouroboros reads to be one the harness
writes and says nothing about the converse. On the map as fog, beside the second-corpus patch, which
has the same shape.
