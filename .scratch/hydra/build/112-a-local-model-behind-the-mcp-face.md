# 112 - A local model behind an MCP server, `ask`, over Ollama

Type: build
Status: claimed
Blocked by:

## What

Map ticket 77 and the map's `## Not yet specified` entry on a mini agent: a small model reaches
a run only as an MCP server declared in `config/mcp.md`, admitted per policy, whose calls land in
the record as `mcp__` tool calls. Build the server: `integrations/local-llm/hydra-local-llm`, a
Python 3 program, standard library only, speaking MCP over stdio the way `src/face.rs` does
(JSON-RPC 2.0, one message per line, `initialize`, `tools/list`, `tools/call`; read `src/face.rs`
lines 100 to 200 and `tests/face.rs` for the handshake and the error shape). One tool, `ask`,
with `prompt` (required), `system` (optional) and `max_tokens` (optional, default 256), that
posts to Ollama at `http://127.0.0.1:11434/api/generate` with `stream: false` and the model
named by `HYDRA_LOCAL_MODEL` (default `gemma3:1b-it-qat`), and returns the response text as the
tool result. Ollama down, or a model not pulled, is a tool error result with the reason, never a
crash of the server. A prompt over 8,000 characters is refused with a tool error, so a run
cannot ship a file through it. Log nothing to stdout except protocol.

Do not edit `config/mcp.md`: it is protected config #5 and the human edits it. Put the exact
`toml` entry the human should paste in `integrations/local-llm/README.md`, in this file's field
vocabulary (`name`, `transport = "stdio"`, `command`, `admit`), admitting `permissive` only, and
say why: the map admits a controller to nothing narrower until a decision says otherwise.

## Done when

`integrations/local-llm/tests/test_local_llm.py` runs green under `python3 -m unittest` against
a stub HTTP server the test starts on a free port (pointed at by `HYDRA_LOCAL_LLM_URL`), and
covers: the handshake; `tools/list` naming `ask`; a call returning the stub's text; Ollama
unreachable as a tool error; the over-length refusal; a malformed request as a JSON-RPC error.
Then, on this machine, one real call through the server against the running Ollama, its output
quoted in your report. Do not run cargo. No em dashes in the README.
