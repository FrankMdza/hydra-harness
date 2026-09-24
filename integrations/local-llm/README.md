# hydra-local-llm

A stdio MCP server that exposes one tool, `ask`, backed by a local Ollama `generate` call.
Nothing under `src/` imports this directory. A run reaches it only when a human adds the server
to `config/mcp.md` and the run's policy admits it.

## Environment

| Variable | Default | Purpose |
|---|---|---|
| `HYDRA_LOCAL_MODEL` | `gemma3:1b-it-qat` | Ollama model name passed to `/api/generate` |
| `HYDRA_LOCAL_LLM_URL` | `http://127.0.0.1:11434/api/generate` | Generate endpoint (tests point this at a stub) |

Diagnostics go to stderr. Stdout carries MCP JSON-RPC only.

## Tool: `ask`

| Argument | Required | Default | Meaning |
|---|---|---|---|
| `prompt` | yes | | Question or task for the local model |
| `system` | no | | Optional system prompt |
| `max_tokens` | no | `256` | Cap on generated tokens (`num_predict` in Ollama) |

Prompts over 8,000 characters are refused with a tool error so a run cannot ship a file through
this surface. Ollama down, or a model that is not pulled, is also a tool error with the reason,
never a crash of the server.

## Run

```
python3 integrations/local-llm/hydra-local-llm
```

## Admit in `config/mcp.md`

Paste the block below into the fenced `toml` in `config/mcp.md`. Adjust `command` to the path
on your machine. **`admit` is `permissive` only** because map ticket 77 admits a controller to
nothing narrower until a decision says otherwise; under `standard` the server would start and the
gate would refuse every `mcp_call` anyway.

```toml
[[server]]
name = "local-llm"
transport = "stdio"
command = "python3"
args = ["/absolute/path/to/hydra/integrations/local-llm/hydra-local-llm"]
admit = ["permissive"]
```

## Tests

```
python3 -m unittest integrations/local-llm/tests/test_local_llm.py
```

The tests start a stub HTTP server on a free port and set `HYDRA_LOCAL_LLM_URL` to it. They never
touch a real `~/hydra` layout or a live Ollama unless you run the server by hand.
