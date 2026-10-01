import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest import mock

import puppets_bin

MODULE = Path(__file__).resolve().parents[1] / "labels.py"
spec = importlib.util.spec_from_file_location("labels", MODULE)
labels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(labels)

GOAL = {"event": "goal", "text": "ship it", "repo": "/repo", "base_sha": "abc"}


class LabelsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.use(HYDRA_PUPPETS_BIN=str(puppets_bin.built()))

    def use(self, **env):
        """Set these variables for the rest of the test; `None` unsets one."""
        patch = mock.patch.dict(os.environ, {k: v for k, v in env.items() if v is not None})
        patch.start()
        self.addCleanup(patch.stop)
        for key, value in env.items():
            if value is None:
                os.environ.pop(key, None)

    def stub(self, body):
        """A stand-in for `hydra-puppets` that runs this shell and nothing else."""
        path = self.home / "bin" / "hydra-puppets"
        path.parent.mkdir(exist_ok=True)
        path.write_text("#!/bin/sh\n" + body)
        path.chmod(path.stat().st_mode | stat.S_IXUSR)
        return path

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

    def write_classes(self, name, lines):
        (self.home / name).write_text("".join(json.dumps(x) + "\n" for x in lines))

    def test_llm_classes_read_every_file_and_the_newest_wins(self):
        self.write_classes("task-classes-llm-2026-09-29.jsonl", [
            {"run_id": "r1", "task_class": "probe", "grade": "low", "area": "hydra", "kind": "probe"},
            {"run_id": "r2", "task_class": "probe", "grade": "low", "unsure": True},
        ])
        self.write_classes("task-classes-llm-2026-10-01.jsonl", [
            {"run_id": "r1", "task_class": "security_review", "grade": "high", "source": "codex"},
        ])
        (self.home / "other.jsonl").write_text('{"run_id": "r3", "task_class": "x", "grade": "y"}\n')
        got = labels.collect_llm(self.home)
        self.assertEqual(sorted(got), ["r1", "r2"])
        self.assertEqual(got["r1"]["task_class"], "security_review")
        self.assertEqual(got["r1"]["source"], "codex")
        self.assertIsNone(got["r1"].get("area"))
        self.assertTrue(got["r2"]["unsure"])


    def test_the_helper_is_an_absolute_path_and_never_a_search_of_path(self):
        on_path = self.stub("echo '{}'\n")
        self.use(PATH=f"{on_path.parent}:{os.environ['PATH']}")
        release = labels.CHECKOUT / "target" / "release" / "hydra-puppets"

        self.use(HYDRA_PUPPETS_BIN="/opt/x/hydra-puppets", HYDRA_BIN="/opt/y/hydra")
        self.assertEqual(labels.helper_bin(), Path("/opt/x/hydra-puppets"))
        self.use(HYDRA_PUPPETS_BIN=None)
        self.assertEqual(labels.helper_bin(), Path("/opt/y/hydra-puppets"))
        # A bare `hydra` is found on PATH by whoever runs it. The helper is not looked for there.
        self.use(HYDRA_BIN="hydra")
        self.assertEqual(labels.helper_bin(), release)
        self.use(HYDRA_BIN=None)
        self.assertEqual(labels.helper_bin(), release)
        self.assertTrue(release.is_absolute())
        self.use(HYDRA_PUPPETS_BIN="hydra-puppets")
        with self.assertRaises(labels.LabelsError):
            labels.helper_bin()

    def test_a_missing_helper_raises_and_no_label_comes_back(self):
        self.write_goal("g1", [GOAL, {"event": "plan_proposed", "provider": "codex", "run_id": "r"}])
        self.use(HYDRA_PUPPETS_BIN=str(self.home / "absent" / "hydra-puppets"))
        with self.assertRaisesRegex(labels.LabelsError, "cannot run"):
            labels.collect(self.home)

    def test_a_missing_helper_beside_hydra_bin_is_not_replaced_by_another(self):
        self.write_goal("g1", [GOAL])
        self.use(HYDRA_PUPPETS_BIN=None, HYDRA_BIN=str(self.home / "absent" / "hydra"))
        with self.assertRaisesRegex(labels.LabelsError, "absent/hydra-puppets"):
            labels.collect(self.home)

    def test_a_damaged_log_raises_even_when_another_goal_is_sound(self):
        self.write_goal("a", [GOAL, {"event": "plan_proposed", "provider": "codex", "run_id": "r"}])
        (self.home / "b.jsonl").write_text(json.dumps(GOAL) + "\nnot json\n" + json.dumps(GOAL) + "\n")
        with self.assertRaisesRegex(labels.LabelsError, "exited 1"):
            labels.collect(self.home)

    def test_a_bad_last_line_is_damage_and_no_longer_passed_over(self):
        (self.home / "g1.jsonl").write_text(json.dumps(GOAL) + "\n{\"event\":\n")
        with self.assertRaises(labels.LabelsError):
            labels.collect(self.home)

    def test_output_of_another_shape_raises(self):
        self.write_goal("g1", [GOAL])
        task = {"id": "t", "class": "c", "grade": "low", "text": "x", "run_id": "r", "human_result": None}
        short = {key: value for key, value in task.items() if key != "human_result"}
        for printed in [
            "not json",
            "[]",
            "{}",
            json.dumps({"goal": None, "tasks": {}}),
            json.dumps({"goal": "text", "tasks": {}, "plans": []}),
            json.dumps({"goal": {"repo": "/r"}, "tasks": {}, "plans": []}),
            json.dumps({"goal": None, "tasks": [task], "plans": []}),
            json.dumps({"goal": None, "tasks": {"t": short}, "plans": []}),
            json.dumps({"goal": None, "tasks": {"t": "x"}, "plans": []}),
            json.dumps({"goal": None, "tasks": {}, "plans": {}}),
            json.dumps({"goal": None, "tasks": {}, "plans": [{"provider": "codex"}]}),
        ]:
            (self.home / "printed").write_text(printed)
            self.use(HYDRA_PUPPETS_BIN=str(self.stub(f"cat '{self.home / 'printed'}'\n")))
            with self.assertRaises(labels.LabelsError, msg=printed):
                labels.collect(self.home)
        # The control: the same stub printing the shape `show` prints is read.
        (self.home / "printed").write_text(
            json.dumps({"goal": None, "tasks": {"t": task}, "plans": [{"run_id": "p"}]})
        )
        self.assertEqual(sorted(labels.collect(self.home)), ["p", "r"])

    def test_the_helper_is_asked_for_the_home_it_was_given(self):
        seen = self.home / "seen"
        self.write_goal("-g", [GOAL])
        self.use(
            HYDRA_PUPPETS_HOME="/elsewhere",
            HYDRA_PUPPETS_BIN=str(
                self.stub(
                    f"echo \"$HYDRA_PUPPETS_HOME $*\" > '{seen}'\n"
                    "echo '{\"goal\": null, \"tasks\": {}, \"plans\": []}'\n"
                )
            ),
        )
        self.assertEqual(labels.collect(self.home), {})
        self.assertEqual(seen.read_text(), f"{self.home} show -- -g\n")

    def test_two_tasks_whose_ids_print_alike_are_both_labelled(self):
        self.write_goal(
            "g1",
            [
                GOAL,
                {"event": "task", "id": 3, "text": "number", "class": "docs", "grade": "low"},
                {"event": "task", "id": "3", "text": "string", "class": "docs", "grade": "high"},
                {"event": "run", "task": 3, "attempt": 1, "run_id": "r-number"},
                {"event": "run", "task": "3", "attempt": 1, "run_id": "r-string"},
                {"event": "review", "task": 3, "judge_run_ids": ["j"], "verdict": "pass"},
            ],
        )
        collected = labels.collect(self.home)
        self.assertEqual(collected["r-number"]["text"], "number")
        self.assertEqual(collected["r-string"]["text"], "string")
        self.assertEqual(collected["j"]["grade"], "low")

if __name__ == "__main__":
    unittest.main()
