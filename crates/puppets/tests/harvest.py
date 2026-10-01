"""Run the puppets Python tests and keep every goal log they built.

Called by `differential.rs` as `python3 harvest.py <tests-dir> <out-dir>`. Just before each
test's tearDown removes its temp home, the goal logs in `HYDRA_PUPPETS_HOME`, the scores file
in `HYDRA_GRADER_HOME` and the vendors file in `HYDRA_PUPPETS_VENDORS` are copied to
`<out-dir>/<n>/`. Whether a test passes does not matter here; only what it left on disk.
"""

import io
import os
import shutil
import sys
import unittest
from pathlib import Path

MODULES = [
    "test_puppets",
    "test_approve_revision",
    "test_panel_degraded",
    "test_queue",
    "test_repair",
    "test_goal_log_tail",
    "test_damaged_log",
]


def main() -> int:
    tests_dir = Path(sys.argv[1])
    out = Path(sys.argv[2])
    sys.path.insert(0, str(tests_dir))
    sys.path.insert(0, str(tests_dir.parent))
    counter = [0]

    def snapshot() -> None:
        home = os.environ.get("HYDRA_PUPPETS_HOME")
        if not home or not Path(home).is_dir():
            return
        logs = sorted(p for p in Path(home).glob("*.jsonl") if p.is_file())
        if not logs:
            return
        counter[0] += 1
        case = out / f"{counter[0]:04d}"
        (case / "puppets").mkdir(parents=True)
        for log in logs:
            shutil.copyfile(log, case / "puppets" / log.name)
        grader = os.environ.get("HYDRA_GRADER_HOME")
        if grader and (Path(grader) / "scores.jsonl").is_file():
            (case / "grader").mkdir()
            shutil.copyfile(Path(grader) / "scores.jsonl", case / "grader" / "scores.jsonl")
        vendors = os.environ.get("HYDRA_PUPPETS_VENDORS")
        if vendors and Path(vendors).is_file():
            shutil.copyfile(vendors, case / "vendors.json")

    original = unittest.TestCase._callTearDown

    def call_tear_down(self):
        snapshot()
        original(self)

    unittest.TestCase._callTearDown = call_tear_down
    suite = unittest.defaultTestLoader.loadTestsFromNames(MODULES)
    unittest.TextTestRunner(stream=io.StringIO(), verbosity=0).run(suite)
    print(counter[0])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
