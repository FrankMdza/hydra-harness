import unittest
import importlib.util
import sys
import os

# The bridge has no .py extension, so name its loader rather than letting
# importlib guess one from the suffix.
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
            ({"state": "ended", "result": {"outcome": "timed_out"}}, "unknown", "timed_out"),
            ({"state": "ended", "result": {"outcome": "crashed"}}, "unknown", "crashed"),
            ({"state": "ended", "result": {"outcome": "unrecognized_outcome"}}, "unknown", "unrecognized_outcome"),
        ]
        
        self.assertEqual(len(cases), 12)
        
        for status_json, expected_state, expected_label in cases:
            state, label = hydra_bridge.get_herdr_state_and_label(status_json)
            self.assertEqual(state, expected_state)
            self.assertEqual(label, expected_label)

if __name__ == '__main__':
    unittest.main()
