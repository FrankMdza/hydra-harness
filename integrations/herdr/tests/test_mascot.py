import unittest
import importlib.util
import sys
import os
from importlib.machinery import SourceFileLoader

MASCOT = os.path.join(os.path.dirname(__file__), "..", "bin", "hydra-mascot")
loader = SourceFileLoader("hydra_mascot", MASCOT)
spec = importlib.util.spec_from_loader("hydra_mascot", loader)
hydra_mascot = importlib.util.module_from_spec(spec)
sys.modules["hydra_mascot"] = hydra_mascot
loader.exec_module(hydra_mascot)


def live_pane(pane_id, run_id, denials=0, tool_rate=0, interactive=False):
    tokens = {
        "run_id": run_id,
        "state": "live",
        "alive": "yes",
        "denials": str(denials),
        "tool_rate": str(tool_rate),
    }
    if interactive:
        tokens["launch_mode"] = "interactive"
    return {"pane_id": pane_id, "tokens": tokens}


class TestMascot(unittest.TestCase):
    def test_no_live_panes(self):
        heads, fires, extra, red, _, _ = hydra_mascot.compute_watch_frame([], {}, {}, 100.0)
        self.assertEqual(heads, ["awake", "awake", "awake"])
        self.assertEqual(fires, [0.0, 0.0, 0.0])
        self.assertEqual(extra, 0)
        self.assertFalse(red)

    def test_one_live_pane(self):
        panes = [live_pane("p1", "run-1")]
        heads, fires, extra, red, _, _ = hydra_mascot.compute_watch_frame(panes, {}, {}, 100.0)
        self.assertEqual(heads, ["awake", "sleeping", "sleeping"])
        self.assertEqual(fires, [0.0, 0.0, 0.0])
        self.assertEqual(extra, 0)
        self.assertFalse(red)

    def test_three_live_panes(self):
        panes = [live_pane(f"p{i}", f"run-{i}") for i in range(3)]
        heads, fires, extra, red, _, _ = hydra_mascot.compute_watch_frame(panes, {}, {}, 100.0)
        self.assertEqual(heads, ["awake", "awake", "awake"])
        self.assertEqual(fires, [0.0, 0.0, 0.0])
        self.assertEqual(extra, 0)
        self.assertFalse(red)

    def test_tool_rate_fires(self):
        panes = [live_pane("p1", "run-1", tool_rate=6)]
        heads, fires, _, red, _, _ = hydra_mascot.compute_watch_frame(panes, {}, {}, 100.0)
        self.assertEqual(heads[0], "awake")
        self.assertEqual(fires[0], 1.0)
        self.assertFalse(red)

    def test_interactive_never_fires(self):
        panes = [live_pane("p1", "run-1", tool_rate=12, interactive=True)]
        _, fires, _, _, _, _ = hydra_mascot.compute_watch_frame(panes, {}, {}, 100.0)
        self.assertEqual(fires, [0.0, 0.0, 0.0])

    def test_denials_flash_red(self):
        panes = [live_pane("p1", "run-1", denials=2)]
        prev = {"p1": 1}
        _, _, _, red, next_prev, flash_until = hydra_mascot.compute_watch_frame(
            panes, prev, {}, 100.0
        )
        self.assertTrue(red)
        self.assertEqual(next_prev["p1"], 2)
        self.assertGreater(flash_until[0], 100.0)

        _, _, _, red_later, _, _ = hydra_mascot.compute_watch_frame(
            panes, next_prev, flash_until, 101.5
        )
        self.assertFalse(red_later)

    def test_more_than_three_live_runs(self):
        panes = [live_pane(f"p{i}", f"run-{i}") for i in range(5)]
        heads, fires, extra, _, _, _ = hydra_mascot.compute_watch_frame(panes, {}, {}, 100.0)
        self.assertEqual(heads, ["awake", "awake", "awake"])
        self.assertEqual(fires, [1.0, 1.0, 1.0])
        self.assertEqual(extra, 2)

    def test_ended_panes_are_ignored(self):
        panes = [
            {
                "pane_id": "p1",
                "tokens": {
                    "run_id": "run-1",
                    "state": "ended",
                    "alive": "no",
                    "denials": "1",
                    "tool_rate": "9",
                },
            }
        ]
        heads, fires, extra, _, _, _ = hydra_mascot.compute_watch_frame(panes, {}, {}, 100.0)
        self.assertEqual(heads, ["awake", "awake", "awake"])
        self.assertEqual(fires, [0.0, 0.0, 0.0])
        self.assertEqual(extra, 0)


if __name__ == "__main__":
    unittest.main()


class SkinTests(unittest.TestCase):
    def test_the_two_skins_alternate(self):
        self.assertEqual(hydra_mascot.SKINS, ["hd", "pet"])
        hydra_mascot.load_skin("pet")
        with self.assertRaises(SystemExit):
            hydra_mascot.load_skin("codex")

    def test_the_hd_skin_renders_to_cells(self):
        cells = hydra_mascot.hd_cells(0.5, ["awake", "sleeping", "firing"], [0.0, 0.0, 1.0], cols=48)
        self.assertEqual((len(cells), len(cells[0])), (24, 48))
        filled = sum(1 for row in cells for (t, b) in row if t or b)
        self.assertGreater(filled, 100)


class FramesTests(unittest.TestCase):
    POSES = ["idle", "working", "one_asleep", "two_asleep", "all_asleep", "refused", "meme", "fire",
             "hurt_0", "hurt_1", "hurt_2"]

    def test_frames_writes_every_pose_with_an_index(self):
        import json
        import re
        import subprocess
        import tempfile

        with tempfile.TemporaryDirectory() as out:
            done = subprocess.run([sys.executable, MASCOT, "--frames", out],
                                  capture_output=True, text=True)
            self.assertEqual(done.returncode, 0, done.stderr)
            index = json.load(open(os.path.join(out, "index.json")))
            self.assertEqual(sorted(index["poses"]), sorted(self.POSES))
            for pose, entry in index["poses"].items():
                self.assertEqual((entry["rows"], entry["cols"]), (12, 24), pose)
                text = open(os.path.join(out, entry["file"])).read()
                rows = [line for line in text.split("\n") if line]
                self.assertEqual(len(rows), entry["rows"], pose)
                for line in rows:
                    plain = line
                    for esc in ("\x1b[0m", "\x1b[K"):
                        plain = plain.replace(esc, "")
                    plain = re.sub(r"\x1b\[[0-9;]*m", "", plain)
                    self.assertEqual(len(plain), entry["cols"], pose)

    def test_the_poses_differ(self):
        import tempfile

        with tempfile.TemporaryDirectory() as out:
            hydra_mascot.write_frames(out)
            texts = {p: open(os.path.join(out, p + ".ans")).read() for p in self.POSES}
            self.assertEqual(len(set(texts.values())), len(self.POSES))


class HurtTests(unittest.TestCase):
    """Build ticket 192: a head whose seat died, crashed or was rate limited is drawn red."""

    @staticmethod
    def reds(cells):
        return sum(1 for row in cells for pair in row for rgb in pair
                   if rgb and rgb[0] > 150 and rgb[1] < 90 and rgb[2] < 90)

    def test_the_compact_dragon_draws_a_hurt_head_red(self):
        awake = hydra_mascot.pet_cells(0.3, ["awake"] * 3, [0.0] * 3, False, 1)
        for i in range(3):
            heads = ["awake"] * 3
            heads[i] = "hurt"
            hurt = hydra_mascot.pet_cells(0.3, heads, [0.0] * 3, False, 1)
            self.assertTrue(hurt != awake, i)
            self.assertGreater(self.reds(hurt), self.reds(awake), i)

    def test_the_hd_dragon_draws_a_hurt_head_red(self):
        awake = hydra_mascot.hd_cells(0.3, ["awake"] * 3, [0.0] * 3, cols=48)
        hurt = hydra_mascot.hd_cells(0.3, ["awake", "hurt", "awake"], [0.0] * 3, cols=48)
        self.assertGreater(self.reds(hurt), self.reds(awake))

    def test_each_hurt_frame_hurts_its_own_head(self):
        poses = {name: heads for name, heads, *_ in hydra_mascot.COCKPIT_POSES}
        for i in range(3):
            heads = ["awake"] * 3
            heads[i] = "hurt"
            self.assertEqual(poses[f"hurt_{i}"], heads)
