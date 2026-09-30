# 88 - A harness of our own for the small models, with pi as the reference

Type: research
Status: resolved
Blocked by:
Opened: 2026-09-30, by the session, at the human's request. Low priority by his own word.

## Question

The human asked about pi (`badlogic/pi-mono`, now Earendil): a minimal coding-agent harness in
TypeScript. Its site says "a minimal agent harness", "primitives, not features", four built-in
tools (read, write, edit, bash), no sub-agents and no plan mode by design ("build it with
extensions"), sessions stored as trees, four modes (interactive, print/JSON, RPC, SDK), 15+
providers through its own unified LLM API, and TypeScript extensions running in the same
process "with full access to the agent session, tools, and UI". MCP was left out at first and is
built in now.

Hydra is a router over four vendor CLIs: it launches, gates, records. It has no harness of its
own, and its local models (Gemma 3 1B, BitNet 2B, map ticket 87) can only answer questions;
they cannot act, because no CLI drives them with tools under Hydra's gate.

Three questions:

1. Would routing pi as a fifth provider extend Hydra? It has print/JSON and RPC modes, so Hydra
   could launch and record it. It takes API keys rather than vendor logins, which is the same
   objection map ticket 85's research made to OpenCode (P4, deferred), and its in-process
   extensions can widen what a run does, which is what Hydra's gate exists to refuse.
2. If Hydra had a harness of its own in Rust, built to Hydra's hook contract from day one, what
   would it need, and which of pi's principles are worth copying? The session's first list:
   copy the four-tool core, sessions as JSONL trees (Hydra's record is JSONL already), print and
   RPC modes, and "primitives, not features"; do not copy in-process extensions with full access,
   because a run must not be able to widen itself.
3. What is the smallest cut that gives the minis hands: an agent loop in Rust over an
   OpenAI-compatible endpoint (Ollama serves one; so would the BitNet shim of build 161), the
   four tools, every tool call passing through the same gate payload `gate.py` reads, a record
   Hydra's contract accepts, and nothing else. Cost in sessions. What it would make possible in
   map ticket 87's loop.

## Evidence

- [pi.dev](https://pi.dev/) and [badlogic/pi-mono](https://github.com/badlogic/pi-mono), read
  2026-09-30. Packages: pi-ai (unified LLM API), pi-agent-core (agent runtime with tool calling),
  pi-coding-agent (the CLI), pi-tui, pi-durable, pi-telemetry, chord.
- `src/provider/` and `src/policy/table.rs`: what Hydra needs from a provider (argv, a hook
  dialect, a stream to normalize). Build 48 measured what adding codex cost: 14 files, 1,365
  insertions, three false assumptions.
- `docs/harness-spec.md` sections 7 and 15: the gate and the protected configs.
- Map ticket 87 and build 160: the apprentice, which today answers and does not act.

## Deliver

A panel of three seats answers the three questions from this ticket and the code; the session
writes the `## Answer`.

## Answer

Resolved 2026-09-30, by the session, under the human's delegation. Three seats, three vendors,
three roles: `claude` opus as architect (run 01a0f0ca-5fa1), `cursor-agent` grok-4.7-high as
researcher (01a0f0ca-7ed1), `codex` gpt-6.1-sol as adversary (01a0f0d5-51ae). Their notes are
`research/88-seat-*.md`.

**1. pi is not routed.** All three seats. Its only veto of a tool call is a `tool_call` handler
inside the process, beside any other extension that loads, and its README says it has no
permission system and runs with the rights of whoever launched it. Its JSON and RPC events
report a tool call after it started. It takes API keys, and `Provider::subscription` has no value
for one. Routing it would move the gate inside the provider, which harness spec 7.1 exists to
refuse.

**2. A harness of Hydra's own is deferred, and its shape is decided.** When it is built, it is
the architect's cut: a separate child executable in Rust, not a loop inside Hydra's core (the
adversary's point: Hydra runs no model and holds no credential, and a child keeps that true);
four tools with `claude`'s names so `gate.py` needs no change and the table is four rows; every
call through `hook_command`, failing closed on anything but a parsed `allow`; frames on stdout
shaped like `claude`'s so the normalizer is reused; `print` mode only, no MCP, no skills, no
resume, no extensions, no session trees. About three sessions by the architect's estimate. One
decision goes to the map first: where the endpoint URL and the model name live, since a run
that can rewrite them picks its own model.

**3. Why deferred: the minis have no hands to give.** The session ran the architect's
zero-session check after the panel: ten bank tasks to `gemma3:1b-it-qat` through Ollama's
`/v1/chat/completions` with a `tools` array. Ollama answered all ten with HTTP 400,
"gemma3:1b-it-qat does not support tools". No local model on this machine emits a tool call
today, so a harness would drive nothing. The condition to reopen: a local model that emits
well-formed tool calls on 7 of the same 10 tasks. The cheapest candidate to try is a small
model Ollama lists with tool support; the 4B Gemma of map ticket 77 is the other.

What the apprentice does meanwhile, measured the same night: on the real bank the 1B model
scored 13 of 20 verifiable tasks (outcome 4 of 4, policy 3 of 3, refused count 3 of 3,
provider and model 1 of 3, interactive 0 of 3). It reads records; it cannot act on them yet.

Also from the seats: build 161's shim must serve `/v1/chat/completions` and not `/api/generate`
if it is ever to feed a harness; "harness run" already names an `evolve` pass in contract 5.5
and the provider cannot take that name; the apprentice's refusal and summary templates read
`claude` frames only, and its pairs do not yet enter `hydra export`.
