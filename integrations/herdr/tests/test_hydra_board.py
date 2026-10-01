import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BOARD = ROOT / "bin" / "hydra-board"

# What `hydra-puppets board` is to this wrapper: a command that prints a table for a goal.
STUB = """#!/bin/sh
[ "$1" = board ] || exit 2
echo "goal $2"
"""

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
        self.helper = Path(self.tmp) / "bin" / "hydra-puppets"
        self.helper.parent.mkdir()
        self.helper.write_text(STUB, encoding="utf-8")
        self.helper.chmod(0o755)

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def test_waiting_line_when_no_logs(self):
        mod = load_board_module()
        text = mod.render_once(self.home, str(self.helper), None, clear=False)
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
        text = mod.render_once(self.home, str(self.helper), None, clear=False)
        self.assertIn("hydra-board: goal goal-new", text)
        self.assertIn("goal goal-new", text)

    def planted(self):
        """A `hydra-puppets` in a directory of its own that records that it ran."""
        path = Path(self.tmp) / "planted" / "hydra-puppets"
        path.parent.mkdir()
        ran = Path(self.tmp) / "ran"
        path.write_text(f"#!/bin/sh\necho \"$*\" >> '{ran}'\necho planted\n", encoding="utf-8")
        path.chmod(0o755)
        return path, ran

    def test_helper_is_named_or_beside_the_real_hydra_and_never_on_path(self):
        mod = load_board_module()
        from unittest import mock

        planted, _ = self.planted()
        tmp = Path(self.tmp).resolve()
        helper = tmp / "bin" / "hydra-puppets"
        hydra = tmp / "bin" / "hydra"
        hydra.write_text("", encoding="utf-8")
        empty = tmp / "empty"
        empty.mkdir()
        (empty / "hydra").write_text("", encoding="utf-8")
        links = tmp / "links"
        links.mkdir()
        (links / "hydra").symlink_to(hydra)
        with mock.patch.dict(os.environ, {"PATH": str(planted.parent)}, clear=True):
            with self.assertRaisesRegex(mod.NoHelper, "no hydra-puppets beside "):
                mod.puppets_helper()
            os.environ["HYDRA_BIN"] = str(empty / "hydra")
            with self.assertRaisesRegex(mod.NoHelper, f"beside {empty / 'hydra'}; run cargo build"):
                mod.puppets_helper()
            # A bare `hydra` is found on PATH by whoever runs it. The helper is not.
            os.environ["HYDRA_BIN"] = "hydra"
            with self.assertRaises(mod.NoHelper):
                mod.puppets_helper()
            os.environ["HYDRA_BIN"] = str(hydra)
            self.assertEqual(mod.puppets_helper(), str(helper))
            # Through a symlink: beside the real binary, not beside the link.
            os.environ["HYDRA_BIN"] = str(links / "hydra")
            self.assertEqual(mod.puppets_helper(), str(helper))
            named = Path(self.tmp).resolve() / "named-hydra-puppets"
            named.write_text("#!/bin/sh\n", encoding="utf-8")
            named.chmod(0o755)
            os.environ["HYDRA_PUPPETS_BIN"] = str(named)
            self.assertEqual(mod.puppets_helper(), str(named))
            os.environ["HYDRA_PUPPETS_BIN"] = "/named/hydra-puppets"
            with self.assertRaisesRegex(mod.NoHelper, "not a file this user can run"):
                mod.puppets_helper()
            for relative in ("hydra-puppets", "planted/hydra-puppets"):
                os.environ["HYDRA_PUPPETS_BIN"] = relative
                with self.assertRaisesRegex(mod.NoHelper, "not an absolute path"):
                    mod.puppets_helper()

    def run_board(self, **env):
        planted, ran = self.planted()
        env = {
            "PATH": f"{planted.parent}:{os.environ['PATH']}",
            "HYDRA_PUPPETS_HOME": str(self.home),
            **env,
        }
        (self.home / "g1.jsonl").write_text(MINIMAL_GOAL, encoding="utf-8")
        try:
            proc = subprocess.run(
                [sys.executable, str(BOARD), "--interval", "30"],
                env=env,
                cwd=self.tmp,
                capture_output=True,
                text=True,
                timeout=3,
            )
        except subprocess.TimeoutExpired:
            self.fail(f"hydra-board ran a helper; the planted one said: {ran.exists()}")
        self.assertFalse(ran.exists(), "the helper planted on PATH ran")
        return proc

    def test_missing_helper_is_said_and_exits_1(self):
        proc = self.run_board()
        self.assertEqual(proc.returncode, 1)
        self.assertIn("no hydra-puppets beside ", proc.stderr)
        self.assertIn("; run cargo build --release --workspace", proc.stderr)

    def test_a_symlinked_hydra_never_runs_a_helper_planted_on_path(self):
        real = Path(self.tmp).resolve() / "real"
        real.mkdir()
        (real / "hydra").write_text("", encoding="utf-8")
        links = Path(self.tmp) / "links"
        links.mkdir()
        (links / "hydra").symlink_to(real / "hydra")
        proc = self.run_board(HYDRA_BIN=str(links / "hydra"))
        self.assertEqual(proc.returncode, 1)
        self.assertEqual(
            proc.stderr,
            f"hydra-board: no hydra-puppets beside {real / 'hydra'}; "
            "run cargo build --release --workspace\n",
        )

    def test_a_relative_named_helper_is_refused(self):
        proc = self.run_board(HYDRA_PUPPETS_BIN="planted/hydra-puppets")
        self.assertEqual(proc.returncode, 1)
        self.assertIn("HYDRA_PUPPETS_BIN is not an absolute path", proc.stderr)

    def test_a_named_helper_that_cannot_run_is_said_without_a_traceback(self):
        plain = Path(self.tmp) / "not-executable"
        plain.write_text("", encoding="utf-8")
        proc = self.run_board(HYDRA_PUPPETS_BIN=str(plain))
        self.assertEqual(proc.returncode, 1)
        self.assertIn("HYDRA_PUPPETS_BIN is not a file this user can run", proc.stderr)
        self.assertNotIn("Traceback", proc.stderr)

    def test_a_named_helper_that_is_a_directory_is_said_without_a_traceback(self):
        proc = self.run_board(HYDRA_PUPPETS_BIN="/usr/bin")
        self.assertEqual(proc.returncode, 1)
        self.assertIn("HYDRA_PUPPETS_BIN is not a file this user can run", proc.stderr)
        self.assertNotIn("Traceback", proc.stderr)

    def test_subprocess_one_frame_before_sleep(self):
        old = self.home / "g1.jsonl"
        old.write_text(MINIMAL_GOAL, encoding="utf-8")
        env = os.environ.copy()
        env["HYDRA_PUPPETS_HOME"] = str(self.home)
        env["HYDRA_PUPPETS_BIN"] = str(self.helper)
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
