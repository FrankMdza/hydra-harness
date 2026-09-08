import unittest
import importlib.util
import sys
import os
from datetime import datetime, timezone

from importlib.machinery import SourceFileLoader

BRIDGE = os.path.join(os.path.dirname(__file__), "..", "bin", "hydra-bridge")
loader = SourceFileLoader("hydra_bridge", BRIDGE)
spec = importlib.util.spec_from_loader("hydra_bridge", loader)
hydra_bridge = importlib.util.module_from_spec(spec)
sys.modules["hydra_bridge"] = hydra_bridge
loader.exec_module(hydra_bridge)


class TestMapping(unittest.TestCase):
    def test_all_rows(self):
        cases = [
            ({"state": "live"}, "working", "live"),
            ({"state": "abandoned"}, "unknown", "abandoned"),
            ({"state": "ended", "result": {"outcome": "completed"}}, "idle", "completed"),
            ({"state": "ended", "result": {"outcome": "failed"}}, "idle", "failed"),
            ({"state": "ended", "result": {"outcome": "gated"}}, "blocked", "gated"),
            ({"state": "ended", "result": {"outcome": "auth_failed"}}, "blocked", "auth_failed"),
            ({"state": "ended", "result": {"outcome": "rate_limited"}}, "blocked", "rate_limited"),
            ({"state": "ended", "result": {"outcome": "mcp_unavailable"}}, "blocked", "mcp_unavailable"),
            ({"state": "ended", "result": {"outcome": "skill_collision"}}, "blocked", "skill_collision"),
            ({"state": "ended", "result": {"outcome": "timed_out"}}, "unknown", "timed out, child gone"),
            ({"state": "ended", "result": {"outcome": "crashed"}}, "unknown", "crashed"),
            ({"state": "ended", "result": {"outcome": "unrecognized_outcome"}}, "unknown", "unrecognized_outcome"),
        ]

        self.assertEqual(len(cases), 12)

        for status_json, expected_state, expected_label in cases:
            state, label = hydra_bridge.get_herdr_state_and_label(status_json)
            self.assertEqual(state, expected_state)
            self.assertEqual(label, expected_label)

    def test_unknown_labels(self):
        self.assertEqual(
            hydra_bridge.get_herdr_state_and_label({"state": "abandoned"}),
            ("unknown", "abandoned"),
        )
        self.assertEqual(hydra_bridge.timed_out_label(True), "timed out, child alive")
        self.assertEqual(hydra_bridge.timed_out_label(False), "timed out, child gone")

    def test_build_tokens(self):
        now = datetime(2026, 9, 7, 10, 1, 30, tzinfo=timezone.utc)
        status = {
            "run_id": "01a07f18-9a28-7111-971c-f7e260c13cdc",
            "state": "live",
            "provider": "claude",
            "policy": "read-only",
            "started_at": "2026-09-07T10:00:00.000Z",
        }
        events = {
            "events": [
                {"seq": 1, "type": "message", "role": "user", "text": "hi"},
                {"seq": 2, "type": "tool_call", "name": "Read", "arguments": {}},
                {"seq": 3, "type": "tool_call", "name": "Shell", "arguments": {}},
            ]
        }

        tokens = hydra_bridge.build_tokens(status, events_data=events, now=now)
        self.assertEqual(tokens["state"], "live")
        self.assertEqual(tokens["alive"], "yes")
        self.assertEqual(tokens["elapsed"], "90")
        self.assertEqual(tokens["last_tool"], "Shell@3")
        self.assertNotIn("outcome", tokens)
        self.assertNotIn("tokens", tokens)

        ended = {
            **status,
            "state": "ended",
            "result": {
                "outcome": "completed",
                "usage": {"counts": {"total_tokens": 1500}},
            },
        }
        ended_tokens = hydra_bridge.build_tokens(ended, now=now)
        self.assertEqual(ended_tokens["state"], "ended")
        self.assertEqual(ended_tokens["alive"], "no")
        self.assertEqual(ended_tokens["outcome"], "completed")
        self.assertEqual(ended_tokens["tokens"], "1500")
        self.assertNotIn("last_tool", ended_tokens)

        title = hydra_bridge.build_title(ended, ended_tokens)
        self.assertIn("claude", title)
        self.assertIn("01a07f18", title)
        self.assertIn("completed", title)

        live_title = hydra_bridge.build_title(status, tokens)
        self.assertIn("Shell", live_title)


if __name__ == "__main__":
    unittest.main()
