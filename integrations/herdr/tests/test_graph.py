import unittest
import importlib.util
import sys
import os
from importlib.machinery import SourceFileLoader

GRAPH = os.path.join(os.path.dirname(__file__), "..", "bin", "hydra-graph")
loader = SourceFileLoader("hydra_graph", GRAPH)
spec = importlib.util.spec_from_loader("hydra_graph", loader)
hydra_graph = importlib.util.module_from_spec(spec)
sys.modules["hydra_graph"] = hydra_graph
loader.exec_module(hydra_graph)

FIXTURE = {
    "run_id": "run-test-001",
    "projection": {
        "run_id": "run-test-001",
        "provider": "claude",
        "policy": "standard",
        "outcome": "completed",
        "usage": {"usage_source": "provider", "counts": {"total_tokens": 1500}},
    },
    "events": [
        {
            "seq": 0,
            "type": "run_start",
            "provider": "claude",
            "policy": "standard",
            "ts": "2026-09-07T10:00:00Z",
        },
        {
            "seq": 1,
            "type": "message",
            "role": "user",
            "text": "read the file\nand summarize",
            "block": 0,
        },
        {
            "seq": 2,
            "type": "tool_call",
            "call_id": "call-1",
            "name": "Read",
            "arguments": {"path": "/tmp/foo.txt"},
            "capability": "file_read",
        },
        {
            "seq": 3,
            "type": "permission_decision",
            "call_id": "call-1",
            "decision": "allow",
            "decider": "gate",
            "reason": "path in scope",
        },
        {
            "seq": 4,
            "type": "tool_result",
            "call_id": "call-1",
            "status": "ok",
            "output": "hello world\nline two",
            "truncated": False,
        },
        {
            "seq": 5,
            "type": "usage",
            "source": "provider",
            "counts": {"total_tokens": 1500, "input_tokens": 1000, "output_tokens": 500},
        },
    ],
    "findings": [],
}

LONGER_FIXTURE = {
    **FIXTURE,
    "events": FIXTURE["events"]
    + [
        {
            "seq": 6,
            "type": "message",
            "role": "assistant",
            "text": "done",
            "block": 1,
        },
        {
            "seq": 7,
            "type": "run_end",
            "outcome": "completed",
            "ts": "2026-09-07T10:05:00Z",
        },
    ],
}


class TestGraph(unittest.TestCase):
    def test_header(self):
        lines, _ = hydra_graph.render(FIXTURE)
        self.assertEqual(
            lines[0],
            "run-test-001  claude  standard  completed  tokens=1500",
        )

    def test_call_join(self):
        lines, _ = hydra_graph.render(FIXTURE)
        call_line = lines[3]
        self.assertIn("Read(", call_line)
        self.assertIn("allow", call_line)
        self.assertIn("gate", call_line)
        self.assertIn("path in scope", call_line)
        result_line = lines[4]
        self.assertTrue(result_line.startswith("│"))
        self.assertIn("ok: hello world", result_line)

    def test_incremental(self):
        _, seen = hydra_graph.render(FIXTURE)
        lines, _ = hydra_graph.render(LONGER_FIXTURE, seen)
        self.assertEqual(len(lines), 2)
        self.assertIn("assistant: done", lines[0])
        self.assertIn("run_end", lines[1])


if __name__ == "__main__":
    unittest.main()
