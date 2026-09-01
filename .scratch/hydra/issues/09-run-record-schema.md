# 09 - The run record schema

Type: grilling
Status: open
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
