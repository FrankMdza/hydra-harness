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
    def test_a_sprite_skin_sleeps_and_fires_per_head(self):
        import random
        hydra_mascot.load_skin("codex")
        cells, overlays = hydra_mascot.sprite_frame(["sleeping", "awake", "firing"], [0.0, 0.0, 1.0], 3, random.Random(1))
        chars = "".join(ch for (ch, _rgb) in overlays.values())
        self.assertTrue(any(c in "zZ" for c in chars), "the sleeping head drifts z's")
        self.assertTrue(any(c in "░▒▓█" for c in chars), "the firing head breathes flame")
        self.assertEqual(len(cells[0]), len(hydra_mascot.PIX_ROWS[0]) + 2 * hydra_mascot.MARGIN)

    def test_skins_cycle_and_the_pet_needs_no_file(self):
        self.assertEqual(hydra_mascot.SKINS[0], "pet")
        hydra_mascot.load_skin("pet")
        self.assertEqual(hydra_mascot.SKINS[(hydra_mascot.SKINS.index("agy") + 1) % len(hydra_mascot.SKINS)], "pet")
