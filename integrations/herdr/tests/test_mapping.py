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

    def test_denials_and_tool_rate_tokens(self):
        now = datetime(2026, 9, 24, 10, 0, 30, tzinfo=timezone.utc)
        status = {
            "run_id": "01a07f18-9a28-7111-971c-f7e260c13cdc",
            "state": "live",
            "provider": "claude",
            "policy": "standard",
            "started_at": "2026-09-24T09:59:00.000Z",
        }
        events = {
            "events": [
                {"seq": 1, "type": "tool_call", "name": "Read", "ts": "2026-09-24T10:00:05.000Z"},
                {"seq": 2, "type": "tool_call", "name": "Shell", "ts": "2026-09-24T10:00:10.000Z"},
                {"seq": 3, "type": "permission_decision", "decision": "allow"},
                {"seq": 4, "type": "permission_decision", "decision": "deny"},
                {"seq": 5, "type": "tool_call", "name": "Write", "ts": "2026-09-24T10:00:20.000Z"},
                {"seq": 6, "type": "permission_decision", "decision": "deny"},
                {"seq": 7, "type": "tool_call", "name": "Read", "ts": "2026-09-24T09:59:50.000Z"},
                {"seq": 8, "type": "tool_call", "name": "Read", "ts": "2026-09-24T10:00:25.000Z"},
            ]
        }

        self.assertEqual(hydra_bridge.count_denials(events), 2)
        self.assertEqual(hydra_bridge.count_tool_rate(events, now=now), 4)

        tokens = hydra_bridge.build_tokens(status, events_data=events, now=now)
        self.assertEqual(tokens["denials"], "2")
        self.assertEqual(tokens["tool_rate"], "4")


if __name__ == "__main__":
    unittest.main()


class ToolRateWithoutTimestamps(unittest.TestCase):
    """Build ticket 122's finding: normalized events carry no `ts`, so the rate comes from the
    growth of the tool-call total across the bridge's own polls."""

    def test_rate_is_the_growth_over_thirty_seconds(self):
        from datetime import datetime, timedelta, timezone
        history = {}
        t0 = datetime(2026, 9, 24, 12, 0, tzinfo=timezone.utc)
        calls = lambda n: {"events": [{"type": "tool_call", "name": "Bash", "seq": i} for i in range(n)]}
        self.assertEqual(hydra_bridge.count_tool_rate(calls(3), now=t0, run_id="r", history=history), 0)
        self.assertEqual(hydra_bridge.count_tool_rate(calls(5), now=t0 + timedelta(seconds=10), run_id="r", history=history), 2)
        self.assertEqual(hydra_bridge.count_tool_rate(calls(12), now=t0 + timedelta(seconds=31), run_id="r", history=history), 9)
        self.assertEqual(hydra_bridge.count_tool_rate(calls(12), now=t0 + timedelta(seconds=70), run_id="r", history=history), 0)

    def test_a_stamped_event_still_counts_by_its_timestamp(self):
        from datetime import datetime, timezone
        now = datetime(2026, 9, 24, 12, 0, 40, tzinfo=timezone.utc)
        events = {"events": [
            {"type": "tool_call", "ts": "2026-09-24T12:00:20Z"},
            {"type": "tool_call", "ts": "2026-09-24T11:59:00Z"},
        ]}
        self.assertEqual(hydra_bridge.count_tool_rate(events, now=now, run_id="r", history={}), 1)
