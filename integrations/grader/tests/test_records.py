import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

MODULE = Path(__file__).resolve().parents[1] / "records.py"
spec = importlib.util.spec_from_file_location("records", MODULE)
records = importlib.util.module_from_spec(spec)
spec.loader.exec_module(records)


class RecordsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runs = self.root / "runs"
        self.runs.mkdir()
        self.home = self.root / "grader"
        self.table = {}
        self.stub = self.root / "hydra"
        self.stub.write_text(
            "#!" + sys.executable + "\n"
            + """import json, os, pathlib, sys
root = pathlib.Path(__file__).parent
assert sys.argv[1] == 'show' and sys.argv[3] == '--events'
assert (pathlib.Path(os.environ['HYDRA_ROOT']) / 'runs' / (sys.argv[2] + '.jsonl')).exists()
with (root / 'calls').open('a') as handle:
    handle.write(sys.argv[2] + '\\n')
print(json.dumps(json.loads((root / 'table.json').read_text())[sys.argv[2]]))
"""
        )
        self.stub.chmod(0o755)

    def record(
        self,
        ident,
        provider="claude",
        model="sonnet",
        parent=None,
        command="run",
        policy="standard",
        task="do the thing",
        ended=True,
        partial=False,
    ):
        envelope = dict(
            run_id=ident,
            parent_run_id=parent,
            provider=provider,
            model=model,
            ts="2026-09-24T00:00:00Z",
        )
        rows = [
            dict(
                envelope,
                hydra=dict(
                    type="run_start",
                    command=command,
                    policy=policy,
                    task=task,
                    subscription="test",
                    provider_version="1",
                    launch_mode="recorded",
                ),
            )
        ]
        if ended:
            rows.append(
                dict(
                    envelope,
                    hydra=dict(type="run_end", ended_at="2026-09-24T00:00:10Z", exit_code=0),
                )
            )
        text = "".join(json.dumps(row) + ("\n" if not partial or index < len(rows) - 1 else "") for index, row in enumerate(rows))
        (self.runs / (ident + ".jsonl")).write_text(text)
        self.table[ident] = {
            "projection": {
                "outcome": "completed" if ended else None,
                "usage": {
                    "usage_source": "provider",
                    "counts": {"raw_input": 100, "cache_read_tokens": 80, "output_tokens": 20},
                },
            },
            "events": [],
        }
        (self.root / "table.json").write_text(json.dumps(self.table))

    def walk(self):
        return list(records.iter_records(self.runs, self.home, str(self.stub)))

    def test_yields_run_fields_and_skips_excluded_records(self):
        self.record("a", model="Sonnet 5")
        self.record("b", parent="a", command="resume")
        self.record("c", ended=False)
        self.record("d", partial=True)
        (self.runs / "evolve.jsonl").write_text(
            json.dumps(
                {
                    "run_id": "evolve",
                    "provider": "claude",
                    "ts": "2026-09-24T00:00:00Z",
                    "hydra": {"type": "run_start", "command": "evolve"},
                }
            )
            + "\n"
        )
        rows = self.walk()
        self.assertEqual([row["run_id"] for row in rows], ["a", "b", "c", "d"])
        row = rows[0]
        self.assertEqual(row["provider"], "claude")
        self.assertEqual(row["recorded_model"], "Sonnet 5")
        self.assertEqual(row["canonical_provider"], "claude")
        self.assertEqual(row["canonical_model"], "sonnet")
        self.assertEqual(row["policy"], "standard")
        self.assertEqual(row["task"], "do the thing")
        self.assertEqual(row["outcome"], "completed")
        self.assertEqual(row["tokens"]["output_tokens"], 20)
        self.assertEqual(row["duration_seconds"], 10.0)
        self.assertIsNone(rows[2]["outcome"])

    def test_show_cache_hits_once_per_fingerprint(self):
        self.record("a")
        self.record("b")
        self.walk()
        self.assertEqual(len((self.root / "calls").read_text().splitlines()), 2)
        cache = json.loads((self.home / "show-cache.json").read_text())
        self.assertIn("a", cache)
        self.assertIn("fingerprint", cache["a"])
        self.walk()
        self.assertEqual(len((self.root / "calls").read_text().splitlines()), 2)
        self.record("a", model="sonnet", task="changed task")
        self.walk()
        self.assertEqual(len((self.root / "calls").read_text().splitlines()), 3)


class RecordsCliTests(unittest.TestCase):
    def test_defaults_follow_hydra_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            runs = root / "runs"
            runs.mkdir()
            (runs / "a.jsonl").write_text(
                json.dumps(
                    {
                        "run_id": "a",
                        "provider": "claude",
                        "model": "sonnet",
                        "ts": "2026-09-24T00:00:00Z",
                        "hydra": {
                            "type": "run_start",
                            "command": "run",
                            "policy": "standard",
                            "task": "x",
                        },
                    }
                )
                + "\n"
            )
            home = root / "grader"
            stub = root / "hydra"
            stub.write_text(
                "#!"
                + sys.executable
                + "\nimport json,sys; print(json.dumps({'projection':{'outcome':'completed','usage':{'counts':{}}},'events':[]}))\n"
            )
            stub.chmod(0o755)
            env = dict(os.environ, HYDRA_ROOT=str(root), HYDRA_GRADER_HOME=str(home), HYDRA_BIN=str(stub))
            with mock.patch.dict(os.environ, env, clear=False):
                rows = list(records.iter_records())
            self.assertEqual(rows[0]["run_id"], "a")


if __name__ == "__main__":
    unittest.main()
