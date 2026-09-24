"""Secret redaction for the grader integration.

Ports the secret classes hydra export redacts from src/export.rs: known key
prefixes, PEM blocks, and the OpenAI key span rule. No repo-name substitution.
"""

from __future__ import annotations

from typing import Any

OPENAI_PROJECT_BODY = 20
OPENAI_LEGACY_BODY = 32


def _token_char(c: str) -> bool:
    return c.isascii() and (c.isalnum() or c in "_-")


def openai_key_spans(text: str) -> list[tuple[int, int]]:
    """Character spans of OpenAI keys in ``text``, left to right. One index space, the
    string's: the first port mixed str.find offsets with a UTF-8 byte array and mis-scanned
    any text with a non-ASCII character before the key (the security review of goal ea69da1f)."""
    spans: list[tuple[int, int]] = []
    from_idx = 0
    n = len(text)
    while True:
        at = text.find("sk-", from_idx)
        if at == -1:
            break
        from_idx = at + 3
        if at > 0 and _token_char(text[at - 1]):
            continue
        rest = text[at + 3 :]
        kind = next((k for k in ("proj-", "svcacct-", "admin-") if rest.startswith(k)), None)
        if kind is not None:
            body_start = at + 3 + len(kind)
            allowed = _token_char
            min_len = OPENAI_PROJECT_BODY
        else:
            body_start = at + 3
            allowed = lambda c: c.isascii() and c.isalnum()  # noqa: E731
            min_len = OPENAI_LEGACY_BODY
        pos = body_start
        while pos < n and allowed(text[pos]):
            pos += 1
        length = pos - body_start
        if length >= min_len and (pos == n or not _token_char(text[pos])):
            spans.append((at, pos))
            from_idx = pos
    return spans


def redact_secrets(text: str) -> str:
    out = text
    for prefix, label in [
        ("AIza", "google_api_key"),
        ("sk-ant-", "anthropic_api_key"),
        ("ghp_", "github_pat"),
        ("ya29.", "google_oauth_token"),
        ("-----BEGIN", "pem_private_key"),
    ]:
        while True:
            at = out.find(prefix)
            if at == -1:
                break
            if prefix == "-----BEGIN":
                footer_rel = out[at:].find("-----END")
                if footer_rel != -1:
                    footer = at + footer_rel
                    close_rel = out[footer + len("-----END") :].find("-----")
                    if close_rel != -1:
                        end = footer + len("-----END") + close_rel + len("-----")
                    else:
                        end = len(out)
                else:
                    end = len(out)
            else:
                end = at + len(out[at:])
                for i, c in enumerate(out[at:]):
                    if c.isspace() or c in "\"'":
                        end = at + i
                        break
            out = out[:at] + f"[REDACTED:{label}]" + out[end:]
    for at, end in reversed(openai_key_spans(out)):
        out = out[:at] + "[REDACTED:openai_api_key]" + out[end:]
    return out


def redact_string(text: str) -> str:
    """Apply secret redaction to a string."""
    return redact_secrets(text)


def redact_value(value: Any) -> Any:
    """Apply secret redaction recursively to a JSON-compatible value."""
    if isinstance(value, str):
        return redact_string(value)
    if isinstance(value, list):
        return [redact_value(item) for item in value]
    if isinstance(value, dict):
        return {redact_string(key) if isinstance(key, str) else key: redact_value(val) for key, val in value.items()}
    return value
