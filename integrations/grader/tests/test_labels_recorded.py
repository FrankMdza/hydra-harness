"""The labels read through `hydra-puppets show` are the labels the Python fold gave.

Map 93 let `puppets.py` go on one condition: the grader's labels, computed through the Python
fold and through the shell-out, are equal on every goal log there is. Build ticket 196 ran that
comparison with both alive, and `crates/puppets/tests/record.py` wrote the fold's side of it
into each `crates/puppets/tests/fixtures/read/*.json`: the goal log (`seed`), what
`puppets.py show` exited with, and under `labels` what `labels.py` derived from `puppets.fold`
before it was rewritten. This test is that comparison with the fold's side read from disk.

The inputs are every goal log the puppets Python tests built and the edge logs of the
differential tests, one of which holds human verdicts, reviews and plans.
"""

import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import puppets_bin

MODULE = Path(__file__).resolve().parents[1] / "labels.py"
spec = importlib.util.spec_from_file_location("labels_recorded", MODULE)
labels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(labels)

FIXTURES = puppets_bin.CHECKOUT / "crates" / "puppets" / "tests" / "fixtures" / "read"


def blob(value):
    return value["text"].encode("utf-8") if "text" in value else bytes.fromhex(value["hex"])


def pairs(found):
    """As `record.py` wrote them: every pair in order, as text, so NaN equals NaN."""
    return json.dumps([[run_id, row] for run_id, row in found.items()])


class RecordedLabelsTests(unittest.TestCase):
    def test_the_shell_out_gives_the_labels_the_python_fold_gave(self):
        paths = sorted(FIXTURES.glob("*.json"))
        compared = refused = count = verdicts = 0
        with mock.patch.dict(os.environ, {"HYDRA_PUPPETS_BIN": str(puppets_bin.built())}):
            for path in paths:
                recorded = json.loads(path.read_text(encoding="utf-8"))
                goal = recorded["goal"]
                with tempfile.TemporaryDirectory() as temp:
                    home = Path(temp) / "puppets"
                    home.mkdir()
                    for file in recorded["seed"]:
                        target = Path(temp) / file["path"]
                        target.parent.mkdir(parents=True, exist_ok=True)
                        target.write_bytes(blob(file["bytes"]))
                    log = home / f"{goal}.jsonl"
                    # The first step is `show <goal>` under `puppets.py`.
                    self.assertEqual(recorded["steps"][0]["args"], ["show", goal], path.name)
                    if recorded["steps"][0]["code"] != 0:
                        # The fold refused the log, so the helper must, and loudly.
                        with self.assertRaises(labels.LabelsError, msg=recorded["label"]):
                            labels.show_goal(goal, home)
                        refused += 1
                        continue
                    state = labels.show_goal(goal, home)
                    found = labels.labels_for_goal(state, labels.committed_events(log))
                self.assertEqual(
                    {"pairs": pairs(found)}, recorded["labels"], recorded["label"]
                )
                compared += 1
                count += len(found)
                verdicts += sum(1 for row in found.values() if row["human_verdict"])
        print(
            f"labels: {compared} goal logs equal with {count} labels, "
            f"{verdicts} of them with a human verdict; {refused} logs refused by both"
        )
        # The recording is not allowed to shrink to nothing without this failing.
        self.assertGreater(compared, 100)
        self.assertGreater(count, 200)
        self.assertGreaterEqual(verdicts, 10)
        self.assertGreater(refused, 10)


if __name__ == "__main__":
    unittest.main()
