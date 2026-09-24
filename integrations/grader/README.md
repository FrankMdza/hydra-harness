# hydra-grader

Secret redaction for grading exported run data. Ports the six secret classes
`hydra export` redacts from `src/export.rs` into stdlib Python.

Nothing under `src/` imports this directory.

## Module: `redact.py`

| Function | Purpose |
|---|---|
| `redact_string(text)` | Redact known secret prefixes, PEM blocks, and OpenAI keys in a string |
| `redact_value(value)` | Same redaction applied recursively to a JSON-compatible value |
| `openai_key_spans(text)` | Return `(start, end)` byte spans of OpenAI keys (shared rule with export) |

No repo-name substitution: `config/export-allowlist.json` is not read.

## Tests

```
python3 -m unittest discover -s integrations/grader/tests -v
```
