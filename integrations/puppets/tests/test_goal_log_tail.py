"""An unterminated last line counts for neither reader, and no writer glues onto it."""

import io
import json
import os
import shutil
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import puppets


class GoalLogTailTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.puppets_home = Path(self.tmp) / "puppets"
        self.puppets_home.mkdir()
        self._prior = os.environ.get("HYDRA_PUPPETS_HOME")
        os.environ["HYDRA_PUPPETS_HOME"] = str(self.puppets_home)

    def tearDown(self):
        if self._prior is None:
            os.environ.pop("HYDRA_PUPPETS_HOME", None)
        else:
            os.environ["HYDRA_PUPPETS_HOME"] = self._prior
        shutil.rmtree(self.tmp)

    def log_path(self, goal_id: str = "g1") -> Path:
        return self.puppets_home / f"{goal_id}.jsonl"

    def plant(self, raw: bytes, goal_id: str = "g1") -> None:
        self.log_path(goal_id).write_bytes(raw)

    def log_bytes(self, goal_id: str = "g1") -> bytes:
        return self.log_path(goal_id).read_bytes()

    def invoke(self, argv: list[str]) -> tuple[int, str, str]:
        stdout, stderr = io.StringIO(), io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            try:
                code = puppets.main(argv)
            except SystemExit as exc:
                code = exc.code if isinstance(exc.code, int) else 1
        return code, stdout.getvalue(), stderr.getvalue()

    def goal_line(self) -> bytes:
        record = {
            "seq": 1,
            "ts": "2026-09-30T12:00:00.000Z",
            "event": "goal",
            "text": "test",
            "repo": "/repo",
            "base_sha": "abc",
        }
        return (json.dumps(record, separators=(",", ":")) + "\n").encode()

    def unterminated_task(self) -> bytes:
        record = {
            "seq": 2,
            "ts": "2026-09-30T12:00:00.000Z",
            "event": "task",
            "id": "t1",
            "text": "do work",
            "class": "batch_scripting",
            "grade": "low",
            "deps": [],
            "paths": [],
        }
        return json.dumps(record, separators=(",", ":")).encode()

    def test_an_unterminated_complete_line_is_ignored_by_both_readers(self):
        self.plant(self.goal_line() + self.unterminated_task())
        before = self.log_bytes()
        self.assertEqual(puppets.fold("g1").revision, 0)
        code, out, err = self.invoke(["show", "g1"])
        self.assertEqual(code, 0, err)
        self.assertEqual(json.loads(out)["revision"], 0)

        seen: dict[str, object] = {}

        def decide(events: list[dict]) -> None:
            seen["revision"] = puppets.fold("g1", events).revision
            seen["events"] = [event["event"] for event in events]
            return None

        with self.assertRaises(puppets.UnterminatedLog):
            puppets._locked_update("g1", decide)
        self.assertEqual(seen["revision"], 0)
        self.assertEqual(seen["events"], ["goal"])
        self.assertEqual(self.log_bytes(), before)

        code, _out, err = self.invoke(
            ["approve", "g1", "--table", "--expect-revision", "0"]
        )
        self.assertEqual(code, 1)
        self.assertEqual(err, "goal log g1 ends without a newline; wrote nothing; puppets.py repair g1\n")
        self.assertNotIn("current revision", err)
        self.assertEqual(self.log_bytes(), before)

    def test_approve_refuses_to_glue_onto_an_unterminated_fragment(self):
        self.plant(self.goal_line() + b'{"event":"assignment",')
        before = self.log_bytes()
        code, _out, err = self.invoke(["approve", "g1", "--table"])
        self.assertEqual(code, 1)
        self.assertEqual(err, "goal log g1 ends without a newline; wrote nothing; puppets.py repair g1\n")
        self.assertEqual(self.log_bytes(), before)
        code, _out, err = self.invoke(["approve", "g1", "--table"])
        self.assertEqual(code, 1)
        self.assertEqual(self.log_bytes(), before)
        events = puppets.read_log("g1")
        self.assertEqual([event["event"] for event in events], ["goal"])

    def test_append_event_and_set_refuse_an_unterminated_tail(self):
        self.plant(self.goal_line() + b'{"event":"assignment",')
        before = self.log_bytes()
        with self.assertRaises(puppets.UnterminatedLog) as raised:
            puppets.append_event(
                "g1",
                {"event": "assignment", "task": "t1", "provider": "codex", "policy": "judge"},
            )
        self.assertEqual(str(raised.exception), "goal log g1 ends without a newline; wrote nothing; puppets.py repair g1")
        self.assertEqual(self.log_bytes(), before)
        code, _out, err = self.invoke(
            ["set", "g1", "t1", "--provider", "codex", "--policy", "judge"]
        )
        self.assertEqual(code, 1)
        self.assertEqual(err, "goal log g1 ends without a newline; wrote nothing; puppets.py repair g1\n")
        self.assertEqual(self.log_bytes(), before)
        events = puppets.read_log("g1")
        self.assertEqual([event["event"] for event in events], ["goal"])
