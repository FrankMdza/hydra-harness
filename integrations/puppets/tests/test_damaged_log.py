"""A newline-terminated blank or non-JSON line makes the goal log damaged."""

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


class DamagedLogTests(unittest.TestCase):
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

    def torn_files(self, goal_id: str = "g1") -> list[Path]:
        return sorted(self.puppets_home.glob(f"{goal_id}.torn-*.txt"))

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

    def task_line(self) -> bytes:
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
        return (json.dumps(record, separators=(",", ":")) + "\n").encode()

    def damaged_message(self, *lines: int) -> str:
        nums = ", ".join(str(n) for n in lines)
        return (
            f"goal log g1 is damaged at line {nums}; wrote nothing; "
            "puppets.py repair g1"
        )

    def test_read_log_raises_on_interior_and_last_bad_lines(self):
        self.plant(self.goal_line() + b"not json\n" + self.task_line())
        with self.assertRaises(puppets.DamagedLog) as raised:
            puppets.read_log("g1")
        self.assertEqual(str(raised.exception), self.damaged_message(2))

        self.plant(self.goal_line() + self.task_line() + b"{\"event\":}\n")
        with self.assertRaises(puppets.DamagedLog) as raised:
            puppets.read_log("g1")
        self.assertEqual(str(raised.exception), self.damaged_message(3))

        self.plant(self.goal_line() + b"\n" + self.task_line())
        with self.assertRaises(puppets.DamagedLog) as raised:
            puppets.read_log("g1")
        self.assertEqual(str(raised.exception), self.damaged_message(2))

    def test_read_log_still_ignores_an_unterminated_tail(self):
        self.plant(self.goal_line() + b"{\"event\":\"task\"")
        events = puppets.read_log("g1")
        self.assertEqual([event["event"] for event in events], ["goal"])

    def test_show_prints_the_named_error(self):
        self.plant(self.goal_line() + b"not json\n" + self.task_line())
        code, out, err = self.invoke(["show", "g1"])
        self.assertEqual(code, 1)
        self.assertEqual(out, "")
        self.assertEqual(err, self.damaged_message(2) + "\n")
        self.assertNotIn("Traceback", err)

    def test_locked_update_refuses_a_damaged_log(self):
        self.plant(self.goal_line() + b"not json\n" + self.task_line())
        before = self.log_bytes()
        with self.assertRaises(puppets.DamagedLog):
            puppets._locked_update("g1", lambda _events: {"event": "note"})
        self.assertEqual(self.log_bytes(), before)

    def test_damage_is_reported_before_an_unterminated_tail(self):
        self.plant(self.goal_line() + b"not json\n" + b'{"event":"task"')
        code, out, err = self.invoke(["show", "g1"])
        self.assertEqual(code, 1)
        self.assertEqual(out, "")
        self.assertEqual(err, self.damaged_message(2) + "\n")

    def test_repair_dry_run_lists_bad_lines(self):
        self.plant(self.goal_line() + b"not json\n" + self.task_line())
        code, out, err = self.invoke(["repair", "g1"])
        self.assertEqual(code, 0)
        self.assertEqual(err, "")
        self.assertIn("bad line 2: not json", out)
        self.assertIn("would move", out)
        self.assertEqual(self.log_bytes(), self.goal_line() + b"not json\n" + self.task_line())

    def test_repair_yes_rewrites_and_appends_a_repair_event(self):
        self.plant(self.goal_line() + b"not json\n" + self.task_line())
        code, out, err = self.invoke(["repair", "g1", "--yes"])
        self.assertEqual(code, 0)
        self.assertEqual(err, "")
        self.assertIn("moved", out)
        events = puppets.read_log("g1")
        self.assertEqual([event["event"] for event in events], ["goal", "task", "repair"])
        self.assertEqual(events[-1]["removed"], 1)
        state = puppets.fold("g1", events)
        self.assertEqual(state.revision, 3)

    def test_repair_yes_on_two_bad_lines_and_a_torn_tail(self):
        raw = self.goal_line() + b"bad1\n\n" + self.task_line() + b'{"event":"x"'
        self.plant(raw)
        code, out, err = self.invoke(["repair", "g1", "--yes"])
        self.assertEqual(code, 0)
        self.assertEqual(err, "")
        events = puppets.read_log("g1")
        self.assertEqual([event["event"] for event in events], ["goal", "task", "repair"])
        self.assertEqual(events[-1]["removed"], 2)
        self.assertEqual(len(self.torn_files()), 1)

    def test_repair_event_voids_earlier_table_approval(self):
        events = [
            json.loads(self.goal_line().decode()),
            json.loads(self.task_line().decode()),
            {
                "seq": 3,
                "ts": "t",
                "event": "approval",
                "kind": "table",
                "revision": 1,
                "yes": True,
            },
            {"seq": 4, "ts": "t", "event": "repair", "removed": 1},
        ]
        state = puppets.fold("g1", events)
        self.assertTrue(any(a.get("voided") for a in state.approvals if a["kind"] == "table"))
