"""Secret redaction for the grader integration.

Ports the secret classes hydra export redacts from src/export.rs: known key
prefixes, PEM blocks, and the OpenAI key span rule. No repo-name substitution.
"""

from __future__ import annotations

from typing import Any

OPENAI_PROJECT_BODY = 20
OPENAI_LEGACY_BODY = 32


def _token_byte(b: int) -> bool:
    if b >= 128:
        return False
    c = chr(b)
    return c.isalnum() or c in "_-"


def openai_key_spans(text: str) -> list[tuple[int, int]]:
    """Byte spans of OpenAI keys in ``text``, left to right."""
    bytes_arr = text.encode("utf-8")
    spans: list[tuple[int, int]] = []
    from_idx = 0
    while True:
        rel = text[from_idx:].find("sk-")
        if rel == -1:
            break
        at = from_idx + rel
        from_idx = at + 3
        if at > 0 and _token_byte(bytes_arr[at - 1]):
            continue
        rest = text[at + 3 :]
        body_start: int
        min_len: int

        matched_kind = next(
            (kind for kind in ("proj-", "svcacct-", "admin-") if rest.startswith(kind)),
            None,
        )
        if matched_kind is not None:
            body_start = at + 3 + len(matched_kind)

            def allowed(b: int) -> bool:
                c = chr(b)
                return c.isalnum() or c in "_-"

            min_len = OPENAI_PROJECT_BODY
        else:
            body_start = at + 3

            def allowed(b: int) -> bool:
                return chr(b).isalnum()

            min_len = OPENAI_LEGACY_BODY

        length = 0
        pos = body_start
        while pos < len(bytes_arr) and allowed(bytes_arr[pos]):
            length += 1
            pos += 1
        end = body_start + length
        if length >= min_len and (end == len(bytes_arr) or not _token_byte(bytes_arr[end])):
            spans.append((at, end))
            from_idx = end
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
        return {key: redact_value(val) for key, val in value.items()}
    return value
