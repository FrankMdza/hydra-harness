"""`repair` saves an unterminated goal-log tail and truncates the log to its last newline."""

import io
import json
import os
import shutil
import sys
import tempfile
import threading
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import puppets


class RepairTests(unittest.TestCase):
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

    def test_repair_prints_the_tail_and_writes_nothing(self):
        tail = b'{"event":"assignment","task":"t1"'
        raw = self.goal_line() + tail
        self.plant(raw)
        code, out, err = self.invoke(["repair", "g1"])
        self.assertEqual(code, 0, err)
        self.assertEqual(err, "")
        self.assertIn(tail.decode(), out)
        self.assertIn("g1.torn-<UTC timestamp>.txt", out)
        self.assertIn("rewrite", out)
        self.assertEqual(self.log_bytes(), raw)
        self.assertEqual(self.torn_files(), [])

    def test_repair_cuts_the_printed_tail_to_300_characters(self):
        tail = b"x" * 300 + b"TAIL-PAST-CUT"
        raw = self.goal_line() + tail
        self.plant(raw)
        code, out, err = self.invoke(["repair", "g1"])
        self.assertEqual(code, 0, err)
        self.assertIn("x" * 300, out)
        self.assertNotIn("TAIL-PAST-CUT", out)
        self.assertEqual(self.log_bytes(), raw)
        self.assertEqual(self.torn_files(), [])

    def test_repair_yes_moves_the_tail_beside_the_log(self):
        tail = b'{"event":"assignment","task":"t1"'
        raw = self.goal_line() + tail
        self.plant(raw)
        stamp = "2026-09-30T19:00:00.000Z"
        original = puppets.utc_now
        puppets.utc_now = lambda: stamp
        try:
            code, out, err = self.invoke(["repair", "g1", "--yes"])
        finally:
            puppets.utc_now = original
        self.assertEqual(code, 0, err)
        self.assertEqual(err, "")
        dest = self.puppets_home / f"g1.torn-{stamp}.txt"
        self.assertEqual(self.torn_files(), [dest])
        self.assertEqual(dest.parent, self.log_path().parent)
        self.assertEqual(dest.read_bytes(), tail)
        self.assertEqual(self.log_bytes(), self.goal_line())
        self.assertTrue(self.log_bytes().endswith(b"\n"))
        self.assertEqual(self.log_bytes() + dest.read_bytes(), raw)
        self.assertIn(dest.name, out)
        record = puppets.append_event("g1", {"event": "note", "text": "after repair"})
        self.assertEqual(record["seq"], 2)
        self.assertEqual(dest.read_bytes(), tail)

    def test_repair_yes_keeps_a_log_that_never_had_a_newline(self):
        raw = b'{"event":"goal","text":"only tail"'
        self.plant(raw)
        code, _out, err = self.invoke(["repair", "g1", "--yes"])
        self.assertEqual(code, 0, err)
        torn = self.torn_files()
        self.assertEqual(len(torn), 1)
        self.assertEqual(torn[0].read_bytes(), raw)
        self.assertEqual(self.log_bytes(), b"")
        self.assertEqual(self.log_bytes() + torn[0].read_bytes(), raw)

    def test_a_log_with_no_torn_tail_says_so_and_exits_0(self):
        raw = self.goal_line()
        self.plant(raw)
        for argv in (["repair", "g1"], ["repair", "g1", "--yes"]):
            code, out, err = self.invoke(argv)
            self.assertEqual(code, 0, err)
            self.assertEqual(err, "")
            self.assertIn("no torn tail", out)
            self.assertEqual(self.log_bytes(), raw)
            self.assertEqual(self.torn_files(), [])
        self.log_path().write_bytes(b"")
        code, out, err = self.invoke(["repair", "g1", "--yes"])
        self.assertEqual(code, 0, err)
        self.assertIn("no torn tail", out)
        self.assertEqual(self.log_bytes(), b"")
        self.assertEqual(self.torn_files(), [])
        self.log_path().unlink()
        code, out, err = self.invoke(["repair", "g1"])
        self.assertEqual(code, 0, err)
        self.assertIn("no torn tail", out)
        self.assertFalse(self.log_path().exists())
        code, out, err = self.invoke(["repair", "missing", "--yes"])
        self.assertEqual(code, 0, err)
        self.assertIn("no torn tail", out)
        self.assertEqual(self.torn_files("missing"), [])

    def test_unterminated_log_names_the_repair_command(self):
        self.plant(self.goal_line() + b'{"event":"assignment",')
        before = self.log_bytes()
        with self.assertRaises(puppets.UnterminatedLog) as raised:
            puppets.append_event(
                "g1",
                {"event": "assignment", "task": "t1", "provider": "codex", "policy": "judge"},
            )
        self.assertIn("puppets.py repair g1", str(raised.exception))
        self.assertEqual(self.log_bytes(), before)
        code, _out, err = self.invoke(["set", "g1", "t1", "--provider", "codex", "--policy", "judge"])
        self.assertEqual(code, 1)
        self.assertIn("puppets.py repair g1", err)
        self.assertEqual(self.log_bytes(), before)

    def test_repair_yes_waits_for_the_log_lock(self):
        tail = b'{"event":"assignment",'
        raw = self.goal_line() + tail
        self.plant(raw)
        path = self.log_path()
        started = threading.Event()
        release = threading.Event()

        def hold() -> None:
            with path.open("rb") as handle:
                puppets.fcntl.flock(handle.fileno(), puppets.fcntl.LOCK_EX)
                try:
                    started.set()
                    release.wait(timeout=5)
                finally:
                    puppets.fcntl.flock(handle.fileno(), puppets.fcntl.LOCK_UN)

        holder = threading.Thread(target=hold)
        holder.start()
        self.assertTrue(started.wait(timeout=5))
        box: dict[str, tuple[int, str, str]] = {}

        def run_repair() -> None:
            box["result"] = self.invoke(["repair", "g1", "--yes"])

        worker = threading.Thread(target=run_repair)
        worker.start()
        try:
            worker.join(0.4)
            self.assertTrue(worker.is_alive())
            self.assertEqual(self.log_bytes(), raw)
            self.assertEqual(self.torn_files(), [])
        finally:
            release.set()
        worker.join(timeout=5)
        holder.join(timeout=5)
        self.assertFalse(worker.is_alive())
        code, _out, err = box["result"]
        self.assertEqual(code, 0, err)
        torn = self.torn_files()
        self.assertEqual(len(torn), 1)
        self.assertEqual(torn[0].read_bytes(), tail)
        self.assertEqual(self.log_bytes() + torn[0].read_bytes(), raw)
