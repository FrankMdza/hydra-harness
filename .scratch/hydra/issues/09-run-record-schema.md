# 09 - The run record schema

Type: grilling
Status: resolved
Blocked by: 01

## Question

D5 makes the run record one of the two things the harness and Ouroboros share, and D10 makes it plain JSONL under `~/hydra/runs/`, one file per session, source of truth. This ticket decides what a line looks like.

Settle:

- The event vocabulary: what event types exist, and which are required against optional
- The envelope every event carries: session id, parent session id for delegated runs, provider, model, timestamp, sequence
- How a delegated sub-run links to its parent, and whether the child writes its own file or into the parent's
- How reasoning tokens are carried, and what happens for providers that never expose them
- How tool calls, their arguments and their results are represented across four providers that each describe them differently
- How a human correction in diff review is recorded, since D12 needs the rejected and chosen pair to survive
- What is deliberately not recorded, and why

Test the schema against concrete scenarios before calling it done: a session that crashes mid-tool-call, a delegation three levels deep, a provider that emits no reasoning at all, and a run where the human rejects every hunk.

D12 is the hard constraint. If the record cannot be converted to SFT and preference-pair formats without loss, it is wrong.

## What the research changed

- **Ticket 02** puts Hydra in the parent position, owning the pty per provider, so it sees raw output rather than whatever a terminal hands back. The schema can assume a complete byte stream.
- **Ticket 07** found the one thing that could break D5. A TUI pane that survives its subprocess dying implies live attach, and live attach needs a channel that static JSONL does not give it. Inventing a second channel would be a third thing crossing between harness and Ouroboros, which D5 forbids. Decide here whether the run file can serve a live reader (tail plus fsync discipline, say) or whether live attach is ruled out.

## What ticket 01 changed

The findings name ten gaps. Treat them as the schema's specification of what must be optional, not as background reading.

Three fields Hydra owns outright, because no provider can supply them reliably:

- **Run id.** Claude and Gemini accept `--session-id` before launch. Cursor needs a `create-chat` round-trip. Hermes cannot be told an id and does not print the one it used. So Hydra mints its own and stores the provider's id as a separate, sometimes late, sometimes absent field.
- **Timestamps.** Only Gemini stamps every frame. Claude stamps some. Cursor and Hermes stamp none. Hydra stamps every line at read time, and keeps the provider's own stamp separately when present, because the two differ by pipe latency.
- **Delegation lineage.** Claude has `parent_tool_use_id`, Hermes has `parent_session_id`, Gemini has one internally but does not surface it, Cursor has nothing. The parent records lineage at spawn. Never read it back from the child.

**Termination is not a frame.** Claude always writes one `result`. Gemini usually does but can exit with empty stdout. Cursor writes one only on success. Hermes writes none. Process exit is the authoritative end of a run; the `result` frame is optional enrichment. The schema must not require a terminator event.

**The D12 problem to solve here.** Cursor emits no token counts at all, and there is no recovery path short of counting characters. Decide what a Cursor run's usage fields hold: null, an estimate flagged as an estimate, or an absent block. Whatever you pick has to survive export without silently becoming a fake number in a training set.

## Answer

**One file per run, not per session.** `runs/<run-id>.jsonl`, run id a UUIDv7 so files sort by time. A session is a tree of runs linked by `parent_run_id` in the envelope and reconstructed at read time. Children never append to a parent's file, because concurrent appenders on one file is where JSONL durability gets hard. This clarifies D10 rather than reversing it.

**Envelope plus raw, and nothing is discarded at write time.** Every line is a Hydra envelope wrapping the provider's frame verbatim. The envelope carries `run_id`, `parent_run_id`, `seq`, `ts`, `provider` and `model`. Normalization is a library function at read time, never a stored field.

The reasoning, since this is the one irreversible decision here: the corpus exists to be trained on later, and nobody yet knows which fields that will need. An adapter bug fixed next month should fix last month's records too, and it can only do that if the raw frame survived. The costs are real and accepted: readers need the normalizer library, and no normalized field is greppable straight out of a file.

**Eleven normalized event types**, versioned, with the version in `run_start`: `run_start`, `message_delta`, `message`, `thinking`, `tool_call`, `tool_result`, `permission_request`, `permission_decision`, `usage`, `correction`, `run_end`. Because raw is kept, growing this list later is cheap, so it stays small on purpose. `thinking` populates from Claude in print mode and from Gemini and Hermes over ACP, never from Cursor. `permission_decision` populates from Claude, and from Gemini and Hermes over ACP.

**Usage carries its provenance.** A `usage_source` field on every run, one of `provider`, `estimated`, `unavailable`. Never a bare null, because null cannot be told apart from a provider reporting zero, and never a silent estimate.

Cursor gets `estimated` rather than `unavailable`. It emits no token counts but does emit `model` on its `system/init` frame, so the estimator picks a tokenizer by model family instead of guessing. A `usage_estimator` field records which tokenizer ran. Because raw text is kept, `hydra usage recount` can re-derive counts later with a better tokenizer, or for Claude-family runs by calling Anthropic's token counting endpoint on stored text. Today's rough number is upgradeable without rerunning anything. That command belongs in the harness spec, not in its own ticket.

**Human corrections are an event, not a sidecar.** `correction` carries the `seq` of the `tool_call` or `tool_result` it corrects, plus the rejected and the chosen content. The pair has to sit inside the same trajectory for preference-pair export to work, and a sidecar file is a second thing to keep in sync for no gain.

**The run file serves live readers.** Append line-buffered with an fsync per line, and let readers tail. Agent events arrive at single digits per second, so fsync is noise on an SSD, and a tailer waits for a newline before parsing. D5 stays at exactly two crossings and the eventual Textual TUI needs no privileged channel. Revisit only with evidence from a real load.

**Redaction happens at export, with one write-time exception.** Storing raw frames means storing file contents, environment variables and whatever an agent happened to read. The bill for choosing raw. Hydra filters its own known credentials at write time, the ones visible in its auth config, so it never writes its own secrets. Everything else stays on disk and gets cleaned in the export pipeline.

The reasoning: `runs/` sits on this laptop under this user account, next to the source and the dotenv files that already hold those secrets, so encrypting at rest adds ceremony without changing who can read it. The export is what leaves the machine, and that is where redaction earns its keep. `runs/` is already in `.gitignore`.

That last decision needs somewhere to live, so the export pipeline graduated out of the fog into ticket 19.

**Termination.** Process exit is the authoritative end of a run. The `result` frame is optional enrichment, because Cursor writes one only on success and Hermes writes none at all. The schema must not require a terminator event.

**Still open, and deliberately downstream:** mapping four incompatible tool-call representations onto one `tool_call` shape. That is adapter work for the harness spec, not a schema decision, and the raw frame means getting it wrong is recoverable.

## Amended by ticket 12

Two changes to the auth side of the schema.

**The write-time credential filter has a defined input set.** This ticket said Hydra filters "the
ones visible in its auth config" at write time. Hydra owns no auth config, so that set would be
empty. It is instead the values Hydra injected into the child's environment for that run, which in
v1 means `GEMINI_API_KEY` on a run carrying `--allow-billed` and nothing at all otherwise. Hydra
knows the value at injection time and strips that exact string from frames as it writes them.
Anything broader stays best-effort and belongs to ticket 19 at export.

**Auth fields, and no `plane` field.** D4's correction says a plane names the auth mechanism and
nothing else, so a field called `plane` would be a synonym for mechanism. Carry `auth.mechanism`
(`subscription-oauth` or `api-key`), `auth.source` (`keychain`, `env` or `none`) and
`auth.allow_billed`. Never carry the injected value, a prefix of it, or its length.

Ticket 12 also needs a terminal outcome for auth failure that is distinct from task failure, so
Ouroboros never reads an expired token as a lesson. Ticket 14 owns the signature taxonomy.
