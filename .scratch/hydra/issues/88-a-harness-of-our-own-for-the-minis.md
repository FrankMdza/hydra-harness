# 88 - A harness of our own for the small models, with pi as the reference

Type: research
Status: claimed
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
