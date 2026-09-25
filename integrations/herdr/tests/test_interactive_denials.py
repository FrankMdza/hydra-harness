import json
import os
import tempfile
import unittest
import importlib.util
import sys
from importlib.machinery import SourceFileLoader

BRIDGE = os.path.join(os.path.dirname(__file__), "..", "bin", "hydra-bridge")
loader = SourceFileLoader("hydra_bridge", BRIDGE)
spec = importlib.util.spec_from_loader("hydra_bridge", loader)
hydra_bridge = importlib.util.module_from_spec(spec)
sys.modules["hydra_bridge"] = hydra_bridge
loader.exec_module(hydra_bridge)


def assistant_line(uuid, parent):
    return json.dumps(
        {
            "type": "assistant",
            "uuid": uuid,
            "parentUuid": parent,
            "message": {
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "toolu_01",
                        "name": "Read",
                        "input": {"file_path": "/etc/hosts"},
                    }
                ],
                "usage": {
                    "input_tokens": 1,
                    "output_tokens": 1,
                    "cache_read_input_tokens": 0,
                    "cache_creation_input_tokens": 0,
                },
            },
        }
    )


def ok_result_line(uuid, parent):
    return json.dumps(
        {
            "type": "user",
            "uuid": uuid,
            "parentUuid": parent,
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "toolu_01",
                        "content": "file contents",
                        "is_error": False,
                    }
                ],
            },
        }
    )


def gate_denial_line(uuid, parent, reason):
    return json.dumps(
        {
            "type": "user",
            "uuid": uuid,
            "parentUuid": parent,
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "toolu_02",
                        "content": reason,
                        "is_error": True,
                    }
                ],
            },
        }
    )


class TestInteractiveDenials(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.mkdtemp(prefix="hydra-bridge-interactive-")
        self.home = os.path.join(self.scratch, "home")
        os.makedirs(self.home, exist_ok=True)
        self.cwd = os.path.join(self.scratch, "cwd")
        os.makedirs(self.cwd, exist_ok=True)
        self.session = "01a07901-0000-7000-8000-000000000002"
        self.run_id = "01a07901-0000-7000-8000-000000000001"
        self.opened = []
        self.real_open = open

        def tracking_open(path, *args, **kwargs):
            self.opened.append(os.path.realpath(path))
            return self.real_open(path, *args, **kwargs)

        hydra_bridge._open = tracking_open
        hydra_bridge.TRANSCRIPT_STATE.clear()

    def tearDown(self):
        hydra_bridge._open = open
        import shutil

        shutil.rmtree(self.scratch, ignore_errors=True)

    def transcript_path(self):
        return hydra_bridge.transcript_path(self.home, self.cwd, self.session)

    def poll(self):
        return hydra_bridge.poll_interactive_denials(
            self.run_id, self.home, self.cwd, self.session
        )

    def test_a_refusal_split_across_two_polls_is_counted_once(self):
        # Claude can be halfway through a line when the bridge reads. The fragment waits for
        # the next poll instead of being skipped as bad JSON and lost.
        path = self.transcript_path()
        os.makedirs(os.path.dirname(path), exist_ok=True)
        line = gate_denial_line("u1", None, "PreToolUse:Write hook error: hydra/out-of-scope: no") + "\n"
        half = len(line) // 2
        with self.real_open(path, "w", encoding="utf-8") as handle:
            handle.write(line[:half])
        self.assertIsNone(self.poll())
        with self.real_open(path, "a", encoding="utf-8") as handle:
            handle.write(line[half:])
        self.assertEqual(self.poll(), 1)
        self.assertEqual(self.poll(), 1)

    def assert_only_scratch_reads(self):
        home = os.path.realpath(self.home)
        for path in self.opened:
            self.assertTrue(
                path.startswith(home + os.sep) or path == home,
                f"opened outside scratch HOME: {path}",
            )

    def tokens_for(self, interactive_denials):
        status = {
            "run_id": self.run_id,
            "state": "live",
            "provider": "claude",
            "policy": "standard",
        }
        return hydra_bridge.build_tokens(status, interactive_denials=interactive_denials)

    def test_denials_from_unset_to_zero_to_one(self):
        self.assertIsNone(self.poll())
        self.assertNotIn("denials", self.tokens_for(self.poll()))
        self.assert_only_scratch_reads()

        path = self.transcript_path()
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(assistant_line("a1", None) + "\n")
            handle.write(ok_result_line("u1", "a1") + "\n")
        self.assertEqual(self.poll(), 0)
        self.assertEqual(self.tokens_for(0)["denials"], "0")
        self.assert_only_scratch_reads()

        with open(path, "a", encoding="utf-8") as handle:
            handle.write(assistant_line("a2", "u1") + "\n")
            handle.write(
                gate_denial_line(
                    "u2",
                    "a2",
                    "PreToolUse:Read hook error: hydra/out-of-scope: blocked.",
                )
                + "\n"
            )
        self.assertEqual(self.poll(), 1)
        self.assertEqual(self.tokens_for(1)["denials"], "1")
        self.assert_only_scratch_reads()


if __name__ == "__main__":
    unittest.main()
