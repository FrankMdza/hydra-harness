# 128 - The secret classes export and the grader share, widened together

Type: build
Status: claimed
Blocked by:

## What

The security review of goal ea69da1f (run 01a0d59c-9f01) measured `integrations/grader/redact.py`
against planted keys: it redacts the five classes `src/export.rs` redacts (Google API keys, Anthropic
keys, GitHub `ghp_`, Google OAuth tokens, PEM blocks) and the OpenAI span rule, and leaves an AWS
access key, an `aws_secret_access_key=` value, Slack `xoxb-` tokens and webhooks, GitHub `github_pat_`
and `gho_` tokens in clear. That is parity with export, so the dataset is no worse than an export,
and no better. Widen both at once, so `SCHEMA.md`'s "the same secret classes as hydra export" stays
true: add the classes to `redact_secrets` in `src/export.rs` and to the verifier's prefix list, port
them to `redact.py`, and pin each with a planted string in `tests/export.rs` and
`integrations/grader/tests/test_redact.py`.

## Done when

The same planted strings are redacted on both sides, the export verifier aborts on each, the grader
suite and `cargo test --test export` are green, and `SCHEMA.md` still says what it says.
