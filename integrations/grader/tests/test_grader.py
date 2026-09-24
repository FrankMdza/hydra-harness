import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

GRADER = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("grader", GRADER / "grader.py")
grader = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = grader
spec.loader.exec_module(grader)

records_spec = importlib.util.spec_from_file_location("records", GRADER / "records.py")
records = importlib.util.module_from_spec(records_spec)
records_spec.loader.exec_module(records)


class GraderTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runs = self.root / "runs"
        self.runs.mkdir()
        self.home = self.root / "grader"
        self.puppets = self.root / "puppets"
        self.puppets.mkdir()
        (self.puppets / "g1.jsonl").write_text(
            json.dumps(
                {
                    "event": "goal",
                    "text": "ship it",
                    "repo": "/repo",
                    "base_sha": "abc",
                }
            )
            + "\n"
            + json.dumps(
                {
                    "event": "task",
                    "id": "t1",
                    "text": "edit records.py",
                    "class": "rust_launch_path",
                    "grade": "medium",
                }
            )
            + "\n"
            + json.dumps({"event": "run", "task": "t1", "attempt": 1, "run_id": "a"})
            + "\n"
            + json.dumps(
                {
                    "event": "human_verdict",
                    "task": "t1",
                    "run_id": "a",
                    "result": "applied",
                }
            )
            + "\n"
        )
        self.table = {}
        self.stub = self.root / "hydra"
        self.stub.write_text(
            "#!" + sys.executable + "\n"
            + """import json, os, pathlib, sys
root = pathlib.Path(__file__).parent
assert sys.argv[1] == 'show' and sys.argv[3] == '--events'
with (root / 'calls').open('a') as handle:
    handle.write(sys.argv[2] + '\\n')
print(json.dumps(json.loads((root / 'table.json').read_text())[sys.argv[2]]))
"""
        )
        self.stub.chmod(0o755)

    def record(self, ident, task="do the thing", model="sonnet"):
        envelope = {
            "run_id": ident,
            "provider": "claude",
            "model": model,
            "ts": "2026-09-24T00:00:00Z",
        }
        rows = [
            {
                **envelope,
                "hydra": {
                    "type": "run_start",
                    "command": "run",
                    "policy": "standard",
                    "task": task,
                },
            },
            {
                **envelope,
                "hydra": {
                    "type": "run_end",
                    "ended_at": "2026-09-24T00:00:10Z",
                    "exit_code": 0,
                },
            },
        ]
        (self.runs / (ident + ".jsonl")).write_text(
            "".join(json.dumps(row) + "\n" for row in rows)
        )
        self.table[ident] = {
            "projection": {
                "outcome": "completed",
                "usage": {"counts": {"raw_input": 1, "output_tokens": 2}},
            },
            "events": [],
        }
        (self.root / "table.json").write_text(json.dumps(self.table))

    def collect(self, **kwargs):
        env = dict(
            os.environ,
            HYDRA_ROOT=str(self.root),
            HYDRA_GRADER_HOME=str(self.home),
            HYDRA_BIN=str(self.stub),
            HYDRA_PUPPETS_HOME=str(self.puppets),
        )
        with mock.patch.dict(os.environ, env, clear=False):
            return grader.collect(runs=self.runs, home=self.home, hydra=str(self.stub), **kwargs)

    def test_once_writes_dataset_cursor_and_schema(self):
        self.record("a")
        self.record("b", task="token sk-ant-secret")
        count = self.collect(jobs=2)
        self.assertEqual(count, 2)
        rows = {row["run_id"]: row for row in records.read_jsonl(self.home / "dataset.jsonl")}
        labeled = rows["a"]
        self.assertEqual(labeled["task_class"], "rust_launch_path")
        self.assertEqual(labeled["labels_source"], "puppets")
        self.assertEqual(labeled["task_text"], "edit records.py")
        redacted = rows["b"]
        self.assertIsNone(redacted["labels_source"])
        self.assertNotIn("sk-ant-", redacted["task_text"])
        self.assertIn("[REDACTED:anthropic_api_key]", redacted["task_text"])
        self.assertEqual(redacted["duration_s"], 10.0)
        cursor = records.read_json(self.home / "cursor.json")
        self.assertEqual(len(cursor), 2)

    def test_once_returns_zero_when_cursor_matches(self):
        self.record("a")
        self.assertEqual(self.collect(), 1)
        self.assertEqual(self.collect(), 0)
        self.assertEqual(len((self.root / "calls").read_text().splitlines()), 1)

    def test_rebuild_replaces_every_line(self):
        self.record("b", task="first")
        self.collect()
        self.record("b", task="changed")
        self.assertEqual(self.collect(rebuild=True), 1)
        row = records.read_jsonl(self.home / "dataset.jsonl")[0]
        self.assertEqual(row["task_text"], "changed")

    def test_cli_once_prints_count(self):
        self.record("a")
        env = dict(
            os.environ,
            HYDRA_ROOT=str(self.root),
            HYDRA_GRADER_HOME=str(self.home),
            HYDRA_BIN=str(self.stub),
            HYDRA_PUPPETS_HOME=str(self.puppets),
        )
        with mock.patch.dict(os.environ, env, clear=False):
            with mock.patch("sys.stdout", new_callable=io.StringIO) as out:
                code = grader.main(["once", "--jobs", "1"])
        self.assertEqual(code, 0)
        self.assertEqual(out.getvalue().strip(), "1")


if __name__ == "__main__":
    unittest.main()
