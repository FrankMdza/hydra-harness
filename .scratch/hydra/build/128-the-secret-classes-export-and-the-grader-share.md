# 128 - The secret classes export and the grader share, widened together

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d617-f5a3 under `standard`, about 1.54M
tokens. The session applied the diff with one addition.

`redact_secrets` in `src/export.rs` and in `integrations/grader/redact.py` gain the same prefixes:
`AKIA` (AWS access key), `aws_secret_access_key=`, `xoxb-` (Slack bot token), `hooks.slack.com`
(Slack webhook, with the `http://` or `https://` in front taken along), `github_pat_` and `gho_`.
The verifier's prefix list in `scan_text` names the same ones, so an export line still carrying
any of them aborts. The `github_pat_` class is labelled `github_fine_grained_pat`, because a
marker carrying its own prefix sent the redactor into an endless loop.

The session's addition: the most common spelling of an AWS secret is the shell variable,
`AWS_SECRET_ACCESS_KEY=`, and only the lowercase form was covered. The uppercase form is now on
both sides, in the verifier and in both tests. The module doc and IMPLEMENTATION.md now say
twelve classes, and a comment in `redact_secrets` that counted "four ... the fifth" was
corrected. The write-time prefix sweep of harness spec 5 still looks for the original five, so a
record can carry one of the six new classes unflagged. Export and the grader redact it on the way
out, and that is where this ticket put the line. Slack `xoxp-` user tokens and other `xox?-`
kinds are not covered; the review did not name them.

Tests: `widened_secret_classes_are_redacted` (lib) and `widened_secret_classes_abort_the_verifier`
(`tests/export.rs`, each planted string as a value and as a JSON key), and
`test_widened_prefixes_are_redacted` in the grader. With `src/export.rs` reverted on master the
verifier test panics at `tests/export.rs:700`; the agent saw the Python side leave all six in
clear before its change. On master: `cargo test --test export` 23 of 23, the lib test, the grader
suite, fmt and clippy all pass. `SCHEMA.md`'s sentence, "the same secret classes as `hydra
export`", is still true.
