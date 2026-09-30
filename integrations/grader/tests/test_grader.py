import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
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
        (self.puppets / "g2.jsonl").write_text(
            json.dumps(
                {
                    "event": "goal",
                    "text": "audit",
                    "repo": "/repo",
                    "base_sha": "def",
                }
            )
            + "\n"
            + json.dumps(
                {
                    "event": "task",
                    "id": "t2",
                    "text": "audit the gate",
                    "class": "security_review",
                    "grade": "high",
                }
            )
            + "\n"
            + json.dumps({"event": "run", "task": "t2", "attempt": 1, "run_id": "c"})
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

    def env(self):
        return dict(
            os.environ,
            HYDRA_ROOT=str(self.root),
            HYDRA_GRADER_HOME=str(self.home),
            HYDRA_BIN=str(self.stub),
            HYDRA_PUPPETS_HOME=str(self.puppets),
        )

    def collect(self, **kwargs):
        with mock.patch.dict(os.environ, self.env(), clear=False):
            return grader.collect(runs=self.runs, home=self.home, hydra=str(self.stub), **kwargs)

    def test_once_builds_one_line_per_run_with_labels_redaction_and_models(self):
        self.record("a", model="Sonnet 5")
        self.record("b", task="token sk-ant-secret")
        self.record("c")
        count = self.collect(jobs=2)
        self.assertEqual(count, 3)
        lines = records.read_jsonl(self.home / "dataset.jsonl")
        self.assertEqual(len(lines), 3)
        self.assertEqual(len({row["run_id"] for row in lines}), 3)
        by_id = {row["run_id"]: row for row in lines}

        labeled = by_id["a"]
        self.assertEqual(labeled["task_class"], "rust_launch_path")
        self.assertEqual(labeled["grade"], "medium")
        self.assertEqual(labeled["human_verdict"], "applied")
        self.assertEqual(labeled["task_text"], "edit records.py")
        self.assertEqual(labeled["labels_source"], "puppets")
        self.assertEqual(labeled["recorded_model"], "Sonnet 5")
        self.assertEqual(labeled["model"], "sonnet")

        redacted = by_id["b"]
        self.assertIsNone(redacted["labels_source"])
        self.assertNotIn("sk-ant-", redacted["task_text"])
        self.assertIn("[REDACTED:anthropic_api_key]", redacted["task_text"])
        self.assertEqual(redacted["duration_s"], 10.0)

        joined = by_id["c"]
        self.assertEqual(joined["task_class"], "security_review")
        self.assertEqual(joined["grade"], "high")
        self.assertEqual(joined["task_text"], "audit the gate")
        self.assertEqual(joined["labels_source"], "puppets")

        cursor = records.read_json(self.home / "cursor.json")
        self.assertEqual(len(cursor), 3)

    def test_second_once_writes_nothing(self):
        self.record("a")
        self.assertEqual(self.collect(), 1)
        self.assertEqual(self.collect(), 0)
        self.assertEqual(len((self.root / "calls").read_text().splitlines()), 1)

    def test_once_replaces_line_when_record_changes(self):
        self.record("b", task="first")
        self.collect()
        self.record("b", task="changed")
        self.assertEqual(self.collect(), 1)
        rows = records.read_jsonl(self.home / "dataset.jsonl")
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["task_text"], "changed")

    def test_rebuild_rewrites_every_line(self):
        self.record("a")
        self.record("b", task="first")
        self.collect()
        self.record("b", task="changed")
        self.assertEqual(self.collect(rebuild=True), 2)
        rows = {row["run_id"]: row for row in records.read_jsonl(self.home / "dataset.jsonl")}
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows["b"]["task_text"], "changed")

    def test_watch_picks_up_record_appended_while_running(self):
        self.record("a")
        proc = subprocess.Popen(
            [sys.executable, str(GRADER / "grader.py"), "watch", "--interval", "1", "--jobs", "1"],
            env=self.env(),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            deadline = time.time() + 5
            while time.time() < deadline:
                if (self.home / "dataset.jsonl").is_file():
                    rows = records.read_jsonl(self.home / "dataset.jsonl")
                    if rows and rows[0]["run_id"] == "a":
                        break
                time.sleep(0.1)
            else:
                self.fail("watch did not collect the initial record")

            self.record("b")
            deadline = time.time() + 5
            while time.time() < deadline:
                rows = records.read_jsonl(self.home / "dataset.jsonl")
                if len(rows) >= 2 and {row["run_id"] for row in rows} >= {"a", "b"}:
                    break
                time.sleep(0.1)
            else:
                self.fail("watch did not collect a record appended during the loop")
            self.assertEqual(len(rows), 2)
        finally:
            proc.terminate()
            proc.wait(timeout=5)

    def plant_model(self, label="outcome", version="baseline-outcome-v1"):
        # A model that has seen only a bias: every run scores sigmoid(0) = 0.5.
        records.write_json(self.home / "models" / f"{label}.json", {
            "label": label, "positive": "x", "vocab": [], "scale": {}, "weights": [],
            "bias": 0.0, "test_from": "2026-09-23T00:00:00+00:00", "version": version,
        })

    def scores(self):
        return records.read_jsonl(self.home / "scores.jsonl")

    def test_collect_scores_each_new_run_with_every_model_once(self):
        self.plant_model("outcome", "baseline-outcome-v1")
        self.plant_model("verdict", "baseline-verdict-v1")
        self.record("a")
        self.record("b")
        self.collect()
        got = self.scores()
        self.assertEqual(sorted((s["run_id"], s["label"]) for s in got), [
            ("a", "outcome"), ("a", "verdict"), ("b", "outcome"), ("b", "verdict"),
        ])
        for score in got:
            self.assertEqual(set(score), {"run_id", "label", "p", "model_version", "scored_at"})
            self.assertAlmostEqual(score["p"], 0.5)
        self.assertEqual({s["model_version"] for s in got},
                         {"baseline-outcome-v1", "baseline-verdict-v1"})

        self.collect()
        self.assertEqual(len(self.scores()), 4)
        # A changed record is collected again, but the same model version scores it once.
        self.record("b", task="changed")
        self.assertEqual(self.collect(), 1)
        self.assertEqual(len(self.scores()), 4)
        # A new model version scores the next run it collects.
        self.plant_model("outcome", "baseline-outcome-v2")
        self.record("c")
        self.collect()
        self.assertEqual([s["run_id"] for s in self.scores()[4:]], ["c", "c"])

    def test_a_ticket_verdict_survives_a_rewritten_line(self):
        self.record("b", task="first")
        self.collect()
        records.write_json(self.home / "ticket-verdicts.json", {"b": "fixed"})
        self.record("b", task="changed")
        self.collect()
        rows = records.read_jsonl(self.home / "dataset.jsonl")
        self.assertEqual(rows[0]["human_verdict"], "fixed")

    def test_collect_without_a_model_writes_no_scores(self):
        self.record("a")
        self.assertEqual(self.collect(), 1)
        self.assertFalse((self.home / "scores.jsonl").exists())

    def plant_dataset(self, *ids):
        self.home.mkdir(parents=True, exist_ok=True)
        (self.home / "dataset.jsonl").write_text(
            "".join(json.dumps({"run_id": i, "provider": "claude", "model": "sonnet",
                                "policy": "standard", "task": "t"}) + "\n" for i in ids)
        )

    def backfill(self):
        with mock.patch.dict(os.environ, self.env(), clear=False):
            with mock.patch("sys.stdout", new_callable=io.StringIO) as out:
                code = grader.main(["backfill"])
        return code, out.getvalue().strip()

    def test_backfill_scores_every_row_and_touches_nothing_else(self):
        self.plant_model("outcome", "baseline-outcome-v1")
        self.plant_dataset("a", "b", "c")
        dataset = (self.home / "dataset.jsonl").read_bytes()
        code, printed = self.backfill()
        self.assertEqual((code, printed), (0, "3"))
        self.assertEqual(sorted(s["run_id"] for s in self.scores()), ["a", "b", "c"])
        self.assertEqual((self.home / "dataset.jsonl").read_bytes(), dataset)
        self.assertFalse((self.home / "cursor.json").exists())
        self.assertFalse((self.root / "calls").exists())

    def test_a_second_backfill_appends_nothing(self):
        self.plant_model()
        self.plant_dataset("a", "b")
        self.assertEqual(self.backfill(), (0, "2"))
        self.assertEqual(self.backfill(), (0, "0"))
        self.assertEqual(len(self.scores()), 2)

    def test_backfill_without_a_model_or_a_dataset_prints_zero(self):
        self.assertEqual(self.backfill(), (0, "0"))
        self.plant_dataset("a")
        self.assertEqual(self.backfill(), (0, "0"))
        self.assertFalse((self.home / "scores.jsonl").exists())
        (self.home / "dataset.jsonl").unlink()
        self.plant_model()
        self.assertEqual(self.backfill(), (0, "0"))
        self.assertFalse((self.home / "scores.jsonl").exists())

    def test_cli_once_prints_count(self):
        self.record("a")
        with mock.patch.dict(os.environ, self.env(), clear=False):
            with mock.patch("sys.stdout", new_callable=io.StringIO) as out:
                code = grader.main(["once", "--jobs", "1"])
        self.assertEqual(code, 0)
        self.assertEqual(out.getvalue().strip(), "1")

    def write_classes(self, name, lines):
        self.home.mkdir(parents=True, exist_ok=True)
        (self.home / name).write_text("".join(json.dumps(x) + "\n" for x in lines))

    def rows_by_id(self):
        return {r["run_id"]: r for r in records.read_jsonl(self.home / "dataset.jsonl")}

    def test_a_model_written_class_fills_an_unlabelled_row_and_puppets_wins(self):
        self.record("a")
        self.record("b")
        self.write_classes("task-classes-llm-2026-09-29.jsonl", [
            {"run_id": "a", "task_class": "probe", "grade": "low", "area": "hydra", "kind": "probe"},
            {"run_id": "b", "task_class": "docs", "grade": "medium", "area": "docs",
             "kind": "edit", "unsure": True},
        ])
        self.collect()
        rows = self.rows_by_id()
        self.assertEqual(rows["a"]["task_class"], "rust_launch_path")
        self.assertEqual(rows["a"]["labels_source"], "puppets")
        self.assertIsNone(rows["a"]["area"])
        self.assertIsNone(rows["a"]["kind"])
        self.assertNotIn("label_unsure", rows["a"])
        self.assertEqual(rows["b"]["task_class"], "docs")
        self.assertEqual(rows["b"]["grade"], "medium")
        self.assertEqual(rows["b"]["labels_source"], "llm")
        self.assertEqual(rows["b"]["area"], "docs")
        self.assertEqual(rows["b"]["kind"], "edit")
        self.assertTrue(rows["b"]["label_unsure"])

    def test_a_later_once_rewrites_the_rows_a_new_class_file_touches(self):
        self.record("b")
        self.record("d")
        self.collect()
        self.assertIsNone(self.rows_by_id()["b"]["labels_source"])
        self.write_classes("task-classes-llm-2026-09-29.jsonl", [
            {"run_id": "b", "task_class": "docs", "grade": "low"},
        ])
        self.assertEqual(self.collect(), 1)
        self.assertEqual(self.rows_by_id()["b"]["labels_source"], "llm")
        self.assertEqual(self.collect(), 0)
        # Only the label changed, so `hydra show` is not asked again.
        self.assertEqual(sorted((self.root / "calls").read_text().split()), ["b", "d"])
        self.write_classes("task-classes-llm-2026-09-30.jsonl", [
            {"run_id": "b", "task_class": "probe", "grade": "high"},
        ])
        self.assertEqual(self.collect(), 1)
        self.assertEqual(self.rows_by_id()["b"]["task_class"], "probe")
        self.assertEqual(self.rows_by_id()["d"]["labels_source"], None)

    def test_rebuild_fills_rows_from_the_class_file(self):
        self.record("b")
        self.write_classes("task-classes-llm-2026-09-29.jsonl", [
            {"run_id": "b", "task_class": "docs", "grade": "low", "kind": "edit"},
        ])
        self.collect(rebuild=True)
        row = self.rows_by_id()["b"]
        self.assertEqual((row["labels_source"], row["kind"]), ("llm", "edit"))

    def test_every_row_carries_area_and_kind_even_when_unlabelled(self):
        self.record("b")
        self.collect()
        row = self.rows_by_id()["b"]
        self.assertIn("area", row)
        self.assertIn("kind", row)


if __name__ == "__main__":
    unittest.main()


class ReviewFindingsTests(unittest.TestCase):
    """The security review of goal ea69da1f: a garbled record must not end the pass, and the
    show cache must hold nothing but what a row reads."""

    def setUp(self):
        import tempfile, textwrap
        self.root = Path(tempfile.mkdtemp())
        self.runs = self.root / "runs"
        self.runs.mkdir()
        self.home = self.root / "grader"
        self.stub = self.root / "hydra"
        self.stub.write_text(textwrap.dedent("""\
            #!/bin/bash
            if [ "$1" = "show" ]; then
              echo '{"projection":{"outcome":"completed","usage":{"counts":{"total_tokens":7}},"result_text":"secret body"},"events":[{"type":"tool_call","name":"Bash"}]}'
              exit 0
            fi
            exit 1
            """))
        self.stub.chmod(0o755)
        good = {"provider": "claude", "model": "sonnet", "run_id": "r-good", "ts": "2026-09-24T10:00:00Z",
                "hydra": {"type": "run_start", "command": "run", "policy": "standard", "task": "hello"}}
        (self.runs / "r-good.jsonl").write_text(json.dumps(good) + "\n")
        (self.runs / "r-bad.jsonl").write_text('{"provider": "claude", "run_id": "r-bad", "ts": "x", "hydra": {"command": "run"}}\n{not json}\n')

    def tearDown(self):
        import shutil
        shutil.rmtree(self.root)

    def test_a_garbled_record_is_skipped_and_the_rest_collected(self):
        import io, contextlib
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            n = grader.collect(runs=self.runs, home=self.home, hydra=str(self.stub), jobs=1)
        self.assertEqual(n, 1)
        rows = [json.loads(l) for l in (self.home / "dataset.jsonl").read_text().splitlines()]
        self.assertEqual([r["run_id"] for r in rows], ["r-good"])
        self.assertIn("r-bad", err.getvalue())

    def test_the_cache_holds_only_outcome_and_counts(self):
        grader.collect(runs=self.runs, home=self.home, hydra=str(self.stub), jobs=1)
        cache = json.loads((self.home / "show-cache.json").read_text())
        answer = cache["r-good"]["answer"]
        self.assertEqual(answer, {"projection": {"outcome": "completed", "usage": {"counts": {"total_tokens": 7}}}})
        self.assertNotIn("secret body", (self.home / "show-cache.json").read_text())
