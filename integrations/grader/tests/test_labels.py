import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

MODULE = Path(__file__).resolve().parents[1] / "labels.py"
spec = importlib.util.spec_from_file_location("labels", MODULE)
labels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(labels)


class LabelsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)

    def write_goal(self, goal_id, events, partial_last=False):
        lines = []
        for index, event in enumerate(events):
            suffix = "" if partial_last and index == len(events) - 1 else "\n"
            lines.append(json.dumps(event) + suffix)
        (self.home / f"{goal_id}.jsonl").write_text("".join(lines))

    def test_task_run_labels(self):
        self.write_goal(
            "g1",
            [
                {"event": "goal", "text": "ship it", "repo": "/repo", "base_sha": "abc"},
                {
                    "event": "task",
                    "id": "t1",
                    "text": "edit records.py",
                    "class": "rust_launch_path",
                    "grade": "medium",
                },
                {
                    "event": "assignment",
                    "task": "t1",
                    "provider": "claude",
                    "model": "sonnet",
                    "policy": "standard",
                    "source": "default",
                },
                {"event": "run", "task": "t1", "attempt": 1, "run_id": "r-task"},
                {"event": "run_end", "task": "t1", "run_id": "r-task", "outcome": "completed"},
                {
                    "event": "human_verdict",
                    "task": "t1",
                    "run_id": "r-task",
                    "result": "applied",
                },
            ],
        )
        collected = labels.collect(self.home)
        self.assertEqual(
            collected["r-task"],
            {
                "task_id": "t1",
                "class": "rust_launch_path",
                "grade": "medium",
                "human_verdict": "applied",
                "text": "edit records.py",
            },
        )

    def test_proposer_and_plan_judge_labels(self):
        self.write_goal(
            "g1",
            [
                {"event": "goal", "text": "ship it", "repo": "/repo", "base_sha": "abc"},
                {"event": "plan_proposed", "provider": "codex", "run_id": "r-codex"},
                {"event": "plan_proposed", "provider": "cursor-agent", "run_id": "r-cursor"},
                {
                    "event": "panel_verdict",
                    "judge_run_ids": ["j1", "j2", "j3"],
                    "chosen": "codex",
                    "path": "/tmp/codex.md",
                },
            ],
        )
        collected = labels.collect(self.home)
        self.assertEqual(
            collected["r-codex"],
            {
                "task_id": None,
                "class": "plan_proposer",
                "grade": None,
                "human_verdict": None,
                "text": "ship it",
            },
        )
        self.assertEqual(collected["j2"]["class"], "plan_judge")
        self.assertEqual(collected["j2"]["text"], "ship it")
        self.assertIsNone(collected["j2"]["task_id"])

    def test_review_judge_labels_use_task_fields(self):
        self.write_goal(
            "g1",
            [
                {"event": "goal", "text": "ship it", "repo": "/repo", "base_sha": "abc"},
                {
                    "event": "task",
                    "id": "t1",
                    "text": "edit records.py",
                    "class": "rust_launch_path",
                    "grade": "medium",
                },
                {"event": "run", "task": "t1", "attempt": 1, "run_id": "r-task"},
                {
                    "event": "review",
                    "task": "t1",
                    "judge_run_ids": ["jr1", "jr2"],
                    "verdict": "pass",
                },
            ],
        )
        collected = labels.collect(self.home)
        self.assertEqual(collected["jr1"]["task_id"], "t1")
        self.assertEqual(collected["jr1"]["class"], "rust_launch_path")
        self.assertEqual(collected["jr1"]["grade"], "medium")
        self.assertEqual(collected["jr1"]["text"], "edit records.py")
        self.assertIsNone(collected["jr1"]["human_verdict"])

    def test_skips_partial_last_line_and_nested_logs(self):
        self.write_goal(
            "g1",
            [
                {"event": "goal", "text": "ship it", "repo": "/repo", "base_sha": "abc"},
                {"event": "plan_proposed", "provider": "codex", "run_id": "r-codex"},
            ],
            partial_last=True,
        )
        nested = self.home / "g2" / "plans"
        nested.mkdir(parents=True)
        (nested / "note.jsonl").write_text('{"event":"goal"}\n')
        self.assertEqual(labels.collect(self.home), {})


if __name__ == "__main__":
    unittest.main()
