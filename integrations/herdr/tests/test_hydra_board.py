import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[1]
BOARD = ROOT / "bin" / "hydra-board"
PUPPETS = REPO / "integrations" / "puppets" / "puppets.py"

MINIMAL_GOAL = (
    '{"seq":1,"ts":"2026-09-24T12:00:00.000Z","event":"goal",'
    '"text":"test","repo":"/repo","base_sha":"abc"}\n'
)


def load_board_module():
    import importlib.machinery
    import importlib.util

    loader = importlib.machinery.SourceFileLoader("hydra_board", str(BOARD))
    spec = importlib.util.spec_from_loader("hydra_board", loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class TestHydraBoard(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.home = Path(self.tmp) / "puppets"
        self.home.mkdir()

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_waiting_line_when_no_logs(self):
        mod = load_board_module()
        text = mod.render_once(self.home, PUPPETS, None, clear=False)
        self.assertIn(mod.WAITING_LINE, text)

    def test_picks_most_recently_changed_goal(self):
        old = self.home / "goal-old.jsonl"
        new = self.home / "goal-new.jsonl"
        old.write_text(MINIMAL_GOAL, encoding="utf-8")
        new.write_text(MINIMAL_GOAL, encoding="utf-8")
        now = time.time()
        os.utime(old, (now - 10, now - 10))
        os.utime(new, (now, now))
        mod = load_board_module()
        self.assertEqual(mod.newest_goal_id(self.home), "goal-new")
        text = mod.render_once(self.home, PUPPETS, None, clear=False)
        self.assertIn("hydra-board: goal goal-new", text)
        self.assertIn("goal goal-new", text)

    def test_subprocess_one_frame_before_sleep(self):
        old = self.home / "g1.jsonl"
        old.write_text(MINIMAL_GOAL, encoding="utf-8")
        env = os.environ.copy()
        env["HYDRA_PUPPETS_HOME"] = str(self.home)
        try:
            subprocess.run(
                [sys.executable, str(BOARD), "--interval", "30"],
                env=env,
                capture_output=True,
                text=True,
                timeout=1,
                check=False,
            )
        except subprocess.TimeoutExpired as exc:
            out = exc.stdout or b""
            err = exc.stderr or b""
            if isinstance(out, bytes):
                out = out.decode()
            if isinstance(err, bytes):
                err = err.decode()
            combined = out + err
        else:
            self.fail("expected hydra-board to block on --interval")
        self.assertIn("goal g1", combined)


if __name__ == "__main__":
    unittest.main()
