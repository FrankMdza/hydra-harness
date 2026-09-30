"""The review queue reads verdict scores. Scratch homes only."""

import contextlib
import io
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import puppets
import test_puppets


def attempt_events(task_id, run_id, provider, model, attempt, *, ended=True, verdict=None):
    """One launch. A verdict is recorded before `run_end`, which is what the log looks like
    when the human answers and `wait` lands afterwards."""
    events = [
        {
            "event": "launch_intent",
            "task": task_id,
            "attempt": attempt,
            "provider": provider,
            "model": model,
            "policy": "standard",
            "argv_sha256": "x",
        },
        {"event": "run", "task": task_id, "attempt": attempt, "run_id": run_id},
    ]
    if verdict:
        events.append(
            {
                "event": "human_verdict",
                "task": task_id,
                "run_id": run_id,
                "result": verdict,
                "note": "",
            }
        )
    if ended:
        events.append(
            {
                "event": "run_end",
                "task": task_id,
                "run_id": run_id,
                "outcome": "completed",
                "total_tokens": 1,
            }
        )
    return events


def task_events(task_id, run_id, provider, model, attempt=1, *, ended=True, verdict=None):
    return [
        {
            "event": "task",
            "id": task_id,
            "text": task_id,
            "class": "docs",
            "grade": "low",
            "deps": [],
            "paths": [],
        },
        {
            "event": "assignment",
            "task": task_id,
            "provider": provider,
            "model": model,
            "policy": "standard",
            "source": "default",
        },
        *attempt_events(
            task_id, run_id, provider, model, attempt, ended=ended, verdict=verdict
        ),
    ]


def score(run_id, p, version, scored_at, label="verdict"):
    return {
        "run_id": run_id,
        "label": label,
        "p": p,
        "model_version": version,
        "scored_at": scored_at,
    }


class QueueTests(test_puppets.PuppetsTestCase):
    def queue(self, goal="g1"):
        out = io.StringIO()
        err = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = puppets.main(["queue", goal])
        return code, out.getvalue(), err.getvalue()

    def task_lines(self, text):
        header, *rest = text.splitlines()
        rows = [line for line in rest if not line.startswith("task ")]
        return header, rows

    def write_scores(self, rows):
        path = self.grader_home / "scores.jsonl"
        path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
        return path

    def test_queue_orders_by_newest_verdict_model_and_marks_the_unscored(self):
        events = [{"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}]
        events += task_events("high", "run-high", "cursor-agent", "composer-2.5", 1)
        events += task_events("mid", "run-mid", "claude", "sonnet", 3)
        events += task_events("oldonly", "run-old", "codex", "", 1)
        events += task_events("bare", "run-bare", "agy", "gemini", 2)
        events += task_events("done", "run-done", "cursor-agent", "composer-2.5", 1, verdict="applied")
        events += task_events("live", "run-live", "cursor-agent", "composer-2.5", 1, ended=False)
        self.write_events("g1", events)
        path = self.write_scores(
            [
                score("run-high", 0.99, "baseline-verdict-zzz", "2026-09-01T00:00:00Z"),
                score("run-high", 0.10, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("run-high", 0.01, "baseline-outcome-aaa", "2026-09-30T00:00:00Z", label="outcome"),
                score("run-high", 0.91, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("run-mid", 0.40, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("run-mid", 0.99, "baseline-outcome-aaa", "2026-09-30T00:00:00Z", label="outcome"),
                score("run-old", 0.95, "baseline-verdict-zzz", "2026-09-01T00:00:00Z"),
                score("run-done", 0.99, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
            ]
        )
        before = path.read_bytes()
        names_before = sorted(item.name for item in self.grader_home.iterdir())

        code, text, err = self.queue()

        self.assertEqual(code, 0, err)
        self.assertEqual(path.read_bytes(), before)
        self.assertEqual(sorted(item.name for item in self.grader_home.iterdir()), names_before)
        header, rows = self.task_lines(text)
        self.assertEqual(header, "verdict baseline-verdict-aaa  2 tasks with no score")
        self.assertEqual([row.split()[0] for row in rows], ["high", "mid", "bare", "oldonly"])
        high, mid, bare, oldonly = rows
        self.assertIn("cursor-agent", high)
        self.assertIn("composer-2.5", high)
        self.assertIn("run-high", high)
        self.assertRegex(high, r"0\.91\s+1$")
        self.assertIn("claude", mid)
        self.assertIn("sonnet", mid)
        self.assertIn("run-mid", mid)
        self.assertRegex(mid, r"0\.40\s+3$")
        self.assertIn("agy", bare)
        self.assertIn("gemini", bare)
        self.assertIn("run-bare", bare)
        self.assertRegex(bare, r"no score\s+2$")
        self.assertIn("codex", oldonly)
        self.assertIn("run-old", oldonly)
        self.assertRegex(oldonly, r"no score\s+1$")
        self.assertNotIn("done", text.split("attempt", 1)[-1])
        self.assertNotIn("live", text)
        self.assertNotIn("run-done", text)
        self.assertNotIn("0.99", text)
        self.assertNotIn("0.95", text)
        self.assertNotIn("0.10", text)

    def test_a_new_attempt_is_queued_when_only_the_previous_run_has_a_verdict(self):
        events = [{"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}]
        events += task_events("t1", "r1", "claude", "sonnet", 1, verdict="rejected")
        events += attempt_events("t1", "r2", "claude", "sonnet", 2)
        self.write_events("g1", events)
        self.write_scores(
            [
                score("r1", 0.99, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("r2", 0.55, "baseline-verdict-aaa", "2026-09-29T00:00:01Z"),
            ]
        )
        code, text, err = self.queue()
        self.assertEqual(code, 0, err)
        header, rows = self.task_lines(text)
        self.assertEqual(header, "verdict baseline-verdict-aaa  0 tasks with no score")
        self.assertEqual(len(rows), 1)
        self.assertIn("r2", rows[0])
        self.assertNotIn("r1", rows[0])
        self.assertRegex(rows[0], r"0\.55\s+2$")

    def test_a_verdict_recorded_before_run_end_is_not_waiting(self):
        events = [{"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}]
        events += task_events("t1", "r1", "claude", "sonnet", 1, verdict="applied")
        self.write_events("g1", events)
        code, text, err = self.queue()
        self.assertEqual(code, 0, err)
        header, rows = self.task_lines(text)
        self.assertEqual(header, "verdict none  0 tasks with no score")
        self.assertEqual(rows, [])

    def test_missing_scores_file_marks_every_waiting_task(self):
        events = [{"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}]
        events += task_events("only", "run-only", "codex", "", 4)
        self.write_events("g1", events)
        code, text, err = self.queue()
        self.assertEqual(code, 0, err)
        header, rows = self.task_lines(text)
        self.assertEqual(header, "verdict none  1 task with no score")
        self.assertEqual(len(rows), 1)
        self.assertRegex(rows[0], r"no score\s+4$")
        self.assertFalse((self.grader_home / "scores.jsonl").exists())

    def test_equal_scores_sort_by_task_id(self):
        events = [{"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}]
        events += task_events("b-task", "run-b", "claude", "sonnet", 1)
        events += task_events("a-task", "run-a", "claude", "sonnet", 1)
        self.write_events("g1", events)
        self.write_scores(
            [
                score("run-b", 0.50, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("run-a", 0.50, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
            ]
        )
        code, text, err = self.queue()
        self.assertEqual(code, 0, err)
        _, rows = self.task_lines(text)
        self.assertEqual([row.split()[0] for row in rows], ["a-task", "b-task"])

    def test_unknown_goal_is_refused(self):
        code, text, err = self.queue("missing")
        self.assertEqual(code, 1)
        self.assertIn("unknown goal missing", err)
        self.assertEqual(text, "")

    def test_board_shows_p_only_beside_a_task_waiting_for_review(self):
        events = [{"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}]
        events += task_events("wait", "run-wait", "cursor-agent", "composer-2.5", 1)
        events += task_events("bare", "run-bare", "agy", "gemini", 1)
        events += task_events("done", "run-done", "claude", "sonnet", 1, verdict="applied")
        events += task_events("live", "run-live", "codex", "", 1, ended=False)
        self.write_events("g1", events)
        self.write_scores(
            [
                score("run-wait", 0.25, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("run-done", 0.99, "baseline-verdict-aaa", "2026-09-29T00:00:00Z"),
                score("run-wait", 0.80, "baseline-verdict-zzz", "2026-09-01T00:00:00Z"),
            ]
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(puppets.main(["board", "g1"]), 0)
        lines = {line.split()[0]: line for line in out.getvalue().splitlines() if line.startswith("  ")}
        self.assertIn("0.25", lines["wait"])
        self.assertNotIn("0.80", lines["wait"])
        self.assertIn("no score", lines["bare"])
        self.assertNotIn("0.99", lines["done"])
        self.assertNotIn("no score", lines["done"])
        self.assertNotIn("no score", lines["live"])
        header = next(line for line in out.getvalue().splitlines() if "status" in line.split())
        self.assertIn("p", header.split())


def load_tests(loader, standard_tests, pattern):
    """Collect only the tests defined here. QueueTests subclasses the puppets fixture, and
    the default loader would otherwise run that parent's tests a second time."""
    suite = unittest.TestSuite()
    for name, value in QueueTests.__dict__.items():
        if name.startswith("test_") and callable(value):
            suite.addTest(QueueTests(name))
    return suite
