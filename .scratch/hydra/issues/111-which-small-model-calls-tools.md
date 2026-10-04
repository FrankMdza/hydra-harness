# 111 - Which small local model calls tools

Type: research
Status: resolved
Blocked by:
Opened: 2026-10-04, by the session, at the human's request of 2026-10-03: try the small models
built for tool use (Llama 3.2 3B, Qwen 2.5 3B, Phi-4-mini, Granite 4, Ministral 3B) against map
88's condition, and make the one that passes the local daily driver.

## Answer

Measured by the session on 2026-10-04 on this Mac (M2, 8 GB), Ollama 0.34.4, one model loaded
at a time, nothing else running but an idle cockpit. The script is
`.scratch/hydra/research/111-tool-call-bakeoff.py` and the raw answers
`111-tool-call-bakeoff-results.jsonl`: ten tasks, the four tools under `claude`'s names (`Read`,
`Write`, `Edit`, `Bash`), sent to `/v1/chat/completions` with a `tools` array at temperature 0.
A task passes only when the first answer is a tool call whose arguments parse, name the right
tool and carry the argument the task needs.

| Model | Passed | Median | On disk |
|---|---|---|---|
| `qwen2.5:3b` | 10 / 10 | 0.9 s | 1.9 GB |
| `ministral-3:3b` | 10 / 10 | 0.7 s | 3.0 GB |
| `llama3.2:3b` | 8 / 10 | 0.6 s | 2.0 GB |
| `granite4:micro` | 7 / 10 | 1.0 s | 2.1 GB |
| `phi4-mini` | 0 / 10 | 2.7 s | 2.5 GB |
| `gemma3:1b-it-qat` (control) | 0 / 10 | | 1.0 GB |

How they failed: `llama3.2:3b` once wrapped its call in a second, nested function object and once
read `tests/*.txt` instead of listing; `granite4:micro` three times read the file before an edit
or a listing, which a real loop would turn into a second turn rather than an error; `phi4-mini`
answered in prose every time, no tool call, which points at Ollama's template for it rather than
the model; `gemma3:1b` is refused by Ollama with HTTP 400, as in map 88.

**Map 88's condition to reopen the harness, 7 of 10, is met by four models.** Ten tasks is a
small sample and they are single-turn; a model that passes here still has to hold up across a
loop with results fed back.

**Daily driver.** `qwen2.5:3b` and `ministral-3:3b` tie on calls. On an 8 GB Mac `qwen2.5:3b`
costs a gigabyte less; `ministral-3:3b` is a little faster. Read on Hugging Face on 2026-10-04: `Qwen/Qwen2.5-3B-Instruct` ships under the Qwen Research
License Agreement, non-commercial; `mistralai/Ministral-3-3B-Instruct-2512` under Apache 2.0. For a
portfolio piece that may be fine-tuned and shipped, that points at `ministral-3:3b`. Map 112
carries the choice with the harness's open decision.
