import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

MODULE = Path(__file__).resolve().parents[1] / "triage.py"
import importlib.util
spec = importlib.util.spec_from_file_location("triage", MODULE)
triage = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = triage
spec.loader.exec_module(triage)


class TriageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runs = self.root / "runs"
        self.runs.mkdir()
        self.home = self.root / "grader"
        self.home.mkdir()

    def record(self, ident, lines, outcome="failed"):
        (self.runs / f"{ident}.jsonl").write_text("\n".join(json.dumps(l) for l in lines) + "\n")
        dataset = self.home / "dataset.jsonl"
        rows = []
        if dataset.exists():
            rows = [json.loads(x) for x in dataset.read_text().splitlines()]
        rows.append({"run_id": ident, "outcome": outcome})
        dataset.write_text("\n".join(json.dumps(r) for r in rows) + "\n")

    def run_triage(self):
        triage.run_triage(self.home, self.runs)
        out = self.home / "triage.jsonl"
        if out.exists():
            return [json.loads(x) for x in out.read_text().splitlines()]
        return []

    def test_quota(self):
        self.record("r1", [
            {"raw": {"type": "rate_limit_event"}},
            {"raw": {"type": "result", "is_error": True, "api_error_status": 429}},
            {"hydra": {"type": "run_end", "exit_code": 1}},
        ], outcome="rate_limited")
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "quota")
        self.assertIn("429", res[0]["evidence"])

    def test_a_rate_limit_event_alone_is_not_quota(self):
        # `claude` sends this frame on healthy runs too.
        self.record("r1", [
            {"raw": {"type": "rate_limit_event"}, "ts": "2026-09-24T00:00:00Z"},
            {"hydra": {"type": "run_end", "exit_code": 1}, "ts": "2026-09-24T00:01:00Z"},
        ])
        self.assertEqual(self.run_triage()[0]["cause"], "unknown")

    def test_codex_usage_limit_sentence_is_quota(self):
        self.record("r1", [
            {"raw": {"type": "turn.failed", "error": {"message": "You've hit your usage limit."}}},
            {"hydra": {"type": "run_end", "exit_code": 1}},
        ])
        self.assertEqual(self.run_triage()[0]["cause"], "quota")

    def test_gate_refusal(self):
        self.record("r1", [{"raw": {"type": "message"}}, {"hydra": {"type": "run_end", "refusal": "policy_narrowing"}}])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "gate")

    def test_gate_last_tool_refused(self):
        self.record("r1", [
            {"raw": {"type": "tool_call", "tool_call": {"name": "Bash", "toolCallId": "1"}}},
            {"raw": {
                "type": "system",
                "subtype": "hook_response",
                "output": json.dumps({"permissionDecision": "deny", "permissionDecisionReason": "hydra/"})
            }},
            {"hydra": {"type": "run_end"}}
        ])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "gate")

    def test_provider_crash_no_run_end(self):
        self.record("r1", [{"hydra": {"type": "run_start"}}, {"raw": {"type": "message"}}])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "provider_crash")

    def test_provider_crash_frame(self):
        self.record("r1", [
            {"raw": {"type": "message"}},
            {"hydra": {"type": "run_end", "exit_code": 1, "error": "the child died on signal Killed: 9"}},
        ], outcome="crashed")
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "provider_crash")

    def test_never_started(self):
        self.record("r1", [{"hydra": {"type": "run_start"}}, {"hydra": {"type": "run_end"}}])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "never_started")

    def test_stall(self):
        self.record("r1", [
            {"raw": {"type": "message"}, "ts": "2026-09-24T00:00:00Z"},
            {"hydra": {"type": "run_end"}, "ts": "2026-09-24T00:15:00Z"}
        ])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "stall")

    def test_collision(self):
        self.record("r1", [{"hydra": {"type": "run_end", "refusal": "skill_collision"}}])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "collision")

    def test_unknown(self):
        self.record("r1", [
            {"raw": {"type": "message"}, "ts": "2026-09-24T00:00:00Z"},
            {"hydra": {"type": "run_end", "exit_code": 1}, "ts": "2026-09-24T00:01:00Z"}
        ])
        res = self.run_triage()
        self.assertEqual(res[0]["cause"], "unknown")

if __name__ == "__main__":
    unittest.main()
