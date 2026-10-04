"""Mirror the redaction cases in tests/export.rs."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from redact import openai_key_spans, redact_string, redact_value  # noqa: E402


class RedactPrefixTests(unittest.TestCase):
    def test_anthropic_prefix_is_redacted(self) -> None:
        text = "here is sk-ant-secret-key-value"
        redacted = redact_string(text)
        self.assertNotIn("sk-ant-", redacted)
        self.assertIn("[REDACTED:anthropic_api_key]", redacted)

    def test_known_prefixes_are_redacted(self) -> None:
        cases = [
            ("token sk-ant-planted end", "anthropic_api_key", "sk-ant-"),
            ("xAIzaSyPlanted", "google_api_key", "AIza"),
            ("ghp_plantedtoken", "github_pat", "ghp_"),
            ("ya29.plantedtoken", "google_oauth_token", "ya29."),
        ]
        for text, label, prefix in cases:
            redacted = redact_string(text)
            self.assertIn(f"[REDACTED:{label}]", redacted)
            self.assertNotIn(prefix, redacted)

    def test_prefix_secrets_redacted_in_json_values(self) -> None:
        for text in [
            "token sk-ant-planted end",
            "xAIzaSyPlanted",
        ]:
            value = {"output": text}
            redacted = redact_value(value)
            self.assertIn("[REDACTED:", json.dumps(redacted))


    def test_widened_prefixes_are_redacted(self) -> None:
        hook = "https://" + "hooks.slack.com/services/T00/B00/planted"
        cases = [
            ("AKIAPlantedSecretKey", "aws_access_key", "PlantedSecretKey"),
            (
                "aws_secret_access_key=planted-aws-secret-value",
                "aws_secret_access_key",
                "planted-aws-secret-value",
            ),
            ("AWS_SECRET_ACCESS_KEY=planted-aws-env-secret", "aws_secret_access_key", "planted-aws-env-secret"),
            ("xoxb-planted-slack-bot-token", "slack_bot_token", "planted-slack-bot-token"),
            (hook, "slack_webhook", "hooks.slack.com"),
            ("github_pat_plantedtokenvalue", "github_fine_grained_pat", "plantedtokenvalue"),
            ("gho_plantedoauthtokenvalue", "github_oauth_token", "plantedoauthtokenvalue"),
        ]
        for text, label, leaked in cases:
            redacted = redact_string(text)
            self.assertIn(f"[REDACTED:{label}]", redacted, text)
            self.assertNotIn(leaked, redacted, text)


class PemRedactionTests(unittest.TestCase):
    def test_pem_block_is_redacted_to_footer_not_first_space(self) -> None:
        body = (
            "-----BEGIN RSA PRIVATE KEY-----\n"
            "MIIEowIBAAKCAQEAplanted\n"
            "bodyline2\n"
            "-----END RSA PRIVATE KEY-----\n"
            "after"
        )
        redacted = redact_string(body)
        self.assertIn("[REDACTED:pem_private_key]", redacted)
        self.assertNotIn("MIIEowIBAAKCAQEAplanted", redacted)
        self.assertNotIn("bodyline2", redacted)
        self.assertNotIn("PRIVATE KEY", redacted)
        self.assertIn("after", redacted)

    def test_pem_with_crlf_is_redacted(self) -> None:
        text = (
            "-----BEGIN RSA PRIVATE KEY-----\r\n"
            "short-body\r\n"
            "-----END RSA PRIVATE KEY-----"
        )
        redacted = redact_string(text)
        self.assertIn("[REDACTED:pem_private_key]", redacted)
        self.assertNotIn("short-body", redacted)

    def test_unfinished_pem_redacts_to_end_of_text(self) -> None:
        text = "before -----BEGIN PRIVATE KEY-----\nno footer"
        redacted = redact_string(text)
        self.assertIn("before", redacted)
        self.assertNotIn("-----BEGIN", redacted)
        self.assertNotIn("no footer", redacted)
        self.assertIn("[REDACTED:pem_private_key]", redacted)

    def test_redacted_pem_leaves_no_header(self) -> None:
        for text in [
            (
                "before -----BEGIN RSA PRIVATE KEY-----\n"
                "MIIE body\n"
                "-----END RSA PRIVATE KEY----- after"
            ),
            "before -----BEGIN PRIVATE KEY-----\nno footer",
        ]:
            redacted = redact_string(text)
            self.assertNotIn("-----BEGIN", redacted, redacted)


class OpenAiKeyTests(unittest.TestCase):
    def test_openai_keys_are_redacted(self) -> None:
        for key in [
            "sk-proj-planted_openai-key-value-0123",
            "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "sk-svcacct-planted_openai-key-value-0123",
        ]:
            redacted = redact_string(key)
            self.assertIn("[REDACTED:openai_api_key]", redacted)
            self.assertNotIn(key, redacted)

    def test_openai_keys_in_context_are_redacted(self) -> None:
        for key in [
            "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD",
            "sk-proj-planted_openai-key-value-0123",
            "sk-svcacct-planted_openai-key-value-0123",
        ]:
            redacted = redact_string(f"key `{key}`, done")
            self.assertEqual(redacted, "key `[REDACTED:openai_api_key]`, done")

    def test_short_sk_in_prose_is_left_alone(self) -> None:
        prose = "the sk- prefix in prose is not a key"
        redacted = redact_string(prose)
        self.assertIn("sk-", redacted)
        self.assertNotIn("[REDACTED:openai_api_key]", redacted)

    def test_sk_in_compound_words_is_left_alone(self) -> None:
        for words in [
            "task-management-system-v2",
            "disk-utilization-report",
            "risk-assessment-notes",
            "flask-sqlalchemy==3.1.1",
            "the `sk-proj-` prefix",
        ]:
            self.assertEqual(redact_string(words), words)

    def test_openai_spans_match_export_rule(self) -> None:
        planted_proj = "sk-proj-planted_openai-key-value-0123"
        planted_legacy = "sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD"
        for text in [planted_proj, planted_legacy]:
            spans = openai_key_spans(text)
            self.assertEqual(len(spans), 1)
            self.assertEqual(text[spans[0][0] : spans[0][1]], text)

    def test_prose_sk_has_no_spans(self) -> None:
        for prose in [
            "the sk- prefix in prose is not a key",
            "task-management-system-v2",
            "disk-utilization-report",
        ]:
            self.assertEqual(openai_key_spans(prose), [])


class JsonRedactionTests(unittest.TestCase):
    def test_nested_json_values_are_redacted(self) -> None:
        value = {
            "events": [
                {"output": "sk-ant-planted-secret"},
                {"arguments": {"nested": "AIzaSyPlanted"}},
            ]
        }
        redacted = redact_value(value)
        dumped = json.dumps(redacted)
        self.assertIn("[REDACTED:anthropic_api_key]", dumped)
        self.assertIn("[REDACTED:google_api_key]", dumped)
        self.assertNotIn("sk-ant-planted", dumped)
        self.assertNotIn("AIzaSyPlanted", dumped)


if __name__ == "__main__":
    unittest.main()


class NonAsciiBeforeAKey(unittest.TestCase):
    """The security review of goal ea69da1f: a character index used into a byte array
    mis-scanned any text with an accent before the key."""

    def test_legacy_key_after_an_accent(self):
        out = redact_string("\u00e9 sk-abcdefghijklmnopqrstuvwxyz0123456789ABCD")
        self.assertEqual(out, "\u00e9 [REDACTED:openai_api_key]")

    def test_project_key_after_accents_keeps_the_tail(self):
        out = redact_string("\u00e9\u00e9\u00e9 sk-proj-planted_openai-key-value-0123 tail")
        self.assertEqual(out, "\u00e9\u00e9\u00e9 [REDACTED:openai_api_key] tail")

    def test_dict_keys_are_redacted_too(self):
        out = redact_value({"sk-ant-planted": "AIzaPlanted"})
        self.assertEqual(out, {"[REDACTED:anthropic_api_key]": "[REDACTED:google_api_key]"})
