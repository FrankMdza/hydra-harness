"""The local judge, against synthetic rows, a stub `hydra show` and a stub Ollama."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from unittest import mock

GRADER = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("grader_judge", GRADER / "judge.py")
judge = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = judge
spec.loader.exec_module(judge)

GOOD = {"needs_correction": True, "failure_class": "agent", "reason": "The diff misses the test."}


class Ollama:
    """Serves queued replies to /api/generate and keeps every request body."""

    def __init__(self) -> None:
        self.replies: list[tuple[int, str]] = []
        self.requests: list[dict] = []
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers["Content-Length"]))
                owner.requests.append({"path": self.path, "body": json.loads(body)})
                code, text = owner.replies.pop(0) if owner.replies else (200, owner.wrap(GOOD))
                self.send_response(code)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(text.encode())

            def log_message(self, *args):
                pass

        self.server = HTTPServer(("127.0.0.1", 0), Handler)
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = f"http://127.0.0.1:{self.server.server_port}/api/generate"

    @staticmethod
    def wrap(answer) -> str:
        text = answer if isinstance(answer, str) else json.dumps(answer)
        return json.dumps({"response": text, "done": True})

    def reply(self, answer, code=200) -> None:
        self.replies.append((code, self.wrap(answer) if code == 200 else json.dumps({"error": "x"})))

    def close(self) -> None:
        self.server.shutdown()
        self.server.server_close()


class JudgeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "runs").mkdir()
        self.home = self.root / "grader"
        self.home.mkdir()
        self.shows: dict[str, dict] = {}
        self.stub = self.root / "hydra"
        self.stub.write_text(
            "#!" + sys.executable + "\n"
            + """import json, pathlib, sys
table = json.loads((pathlib.Path(__file__).parent / 'shows.json').read_text())
assert sys.argv[1] == 'show'
if sys.argv[2] not in table:
    sys.exit(1)
print(json.dumps({'projection': table[sys.argv[2]]}))
"""
        )
        self.stub.chmod(0o755)
        self.ollama = Ollama()
        self.addCleanup(self.ollama.close)
        self.env = {
            "HYDRA_ROOT": str(self.root),
            "HYDRA_GRADER_HOME": str(self.home),
            "HYDRA_BIN": str(self.stub),
            "HYDRA_LOCAL_LLM_URL": self.ollama.url,
            "HYDRA_LOCAL_MODEL": "stub-model",
        }
        self.rows: list[dict] = []

    def row(self, ident, task="fix it", outcome="completed", verdict=None, result="done", stat=None):
        self.rows.append({
            "run_id": ident, "task_text": task, "outcome": outcome, "human_verdict": verdict,
        })
        self.shows[ident] = {"result_text": result, "diff_stat": stat}
        (self.home / "dataset.jsonl").write_text("".join(json.dumps(r) + "\n" for r in self.rows))
        (self.root / "shows.json").write_text(json.dumps(self.shows))

    def run_main(self, *argv, env=None):
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.dict(os.environ, env or self.env), \
                contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = judge.main(list(argv))
            except SystemExit as exit_:
                code = exit_.code
        return code, out.getvalue(), err.getvalue()

    def labels(self) -> list[dict]:
        path = self.home / "judge-labels.jsonl"
        return [json.loads(l) for l in path.read_text().splitlines()] if path.exists() else []

    # item 1
    def test_prompt_carries_task_result_and_diff_stat(self):
        self.row("a", task="rename the flag", result="renamed it", stat={"files": 2, "insertions": 5})
        self.run_main("label")
        prompt = self.ollama.requests[0]["body"]["prompt"]
        self.assertIn("rename the flag", prompt)
        self.assertIn("renamed it", prompt)
        self.assertIn('"insertions": 5', prompt)

    def test_texts_are_redacted_and_cut_with_a_marker(self):
        self.row("a", task="t" * 3500 + " sk-ant-planted", result="r" * 5000 + " ghp_plantedtoken")
        self.row("b", task="use sk-ant-planted", result="key ghp_plantedtoken")
        self.run_main("label")
        long_prompt = self.ollama.requests[0]["body"]["prompt"]
        self.assertIn("t" * 3000, long_prompt)
        self.assertNotIn("t" * 3001, long_prompt)
        self.assertIn("r" * 4000, long_prompt)
        self.assertNotIn("r" * 4001, long_prompt)
        self.assertEqual(long_prompt.count("[cut:"), 2)
        short_prompt = self.ollama.requests[1]["body"]["prompt"]
        self.assertNotIn("[cut:", short_prompt)
        self.assertNotIn("sk-ant-planted", short_prompt)
        self.assertNotIn("ghp_plantedtoken", short_prompt)
        self.assertIn("[REDACTED:", short_prompt)

    # item 2
    def test_request_shape_and_environment(self):
        self.row("a")
        self.run_main("label")
        request = self.ollama.requests[0]
        self.assertEqual(request["path"], "/api/generate")
        body = request["body"]
        self.assertEqual(body["model"], "stub-model")
        self.assertEqual(body["format"], "json")
        self.assertIs(body["stream"], False)
        self.assertEqual(body["options"]["temperature"], 0)
        self.assertIsInstance(body["options"]["num_predict"], int)

    def test_defaults_match_the_local_llm_integration(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            self.assertEqual(judge.ollama_url(), "http://127.0.0.1:11434/api/generate")
            self.assertEqual(judge.ollama_model(), "gemma3:1b-it-qat")

    # item 3
    def test_one_line_per_run_and_a_second_pass_appends_nothing(self):
        self.row("a")
        self.row("b")
        code, _, _ = self.run_main("label")
        self.assertEqual(code, 0)
        lines = self.labels()
        self.assertEqual([l["run_id"] for l in lines], ["a", "b"])
        line = lines[0]
        self.assertEqual(set(line), {
            "run_id", "needs_correction", "failure_class", "reason", "judge_model",
            "prompt_sha256", "judged_at",
        })
        self.assertIs(line["needs_correction"], True)
        self.assertEqual(line["failure_class"], "agent")
        self.assertEqual(line["judge_model"], "stub-model")
        self.assertEqual(len(line["prompt_sha256"]), 64)
        before = (self.home / "judge-labels.jsonl").read_text()
        calls = len(self.ollama.requests)
        self.run_main("label")
        self.assertEqual((self.home / "judge-labels.jsonl").read_text(), before)
        self.assertEqual(len(self.ollama.requests), calls)

    def test_a_new_model_or_prompt_labels_again(self):
        self.row("a")
        self.run_main("label")
        self.run_main("label", env={**self.env, "HYDRA_LOCAL_MODEL": "other"})
        self.assertEqual([l["judge_model"] for l in self.labels()], ["stub-model", "other"])
        self.row("a", task="a different task")
        self.run_main("label")
        self.assertEqual(len(self.labels()), 3)

    def test_unparsed_answers_are_kept_cut_and_counted(self):
        for ident in "abcd":
            self.row(ident)
        self.ollama.reply("not json at all " + "x" * 600)
        self.ollama.reply({"needs_correction": "yes", "failure_class": "agent", "reason": "r"})
        self.ollama.reply({"needs_correction": True, "failure_class": "weather", "reason": "r"})
        self.ollama.reply(GOOD)
        _, out, _ = self.run_main("label")
        lines = self.labels()
        self.assertEqual([l["needs_correction"] for l in lines], [None, None, None, True])
        self.assertEqual(len(lines[0]["unparsed"]), 500)
        self.assertTrue(lines[0]["unparsed"].startswith("not json at all"))
        self.assertNotIn("unparsed", lines[3])
        self.assertIn("unparsed 3", out)

    # item 4
    def test_rate_limited_and_gated_rows_are_not_sent(self):
        self.row("a", outcome="rate_limited")
        self.row("b", outcome="gated")
        self.run_main("label")
        self.assertEqual(self.ollama.requests, [])
        first, second = self.labels()
        self.assertEqual((first["failure_class"], second["failure_class"]), ("quota", "gate"))
        for line in (first, second):
            self.assertIsNone(line["needs_correction"])
            self.assertEqual(line["judge_model"], "rule")
        self.run_main("label")
        self.assertEqual(len(self.labels()), 2)

    # item 5
    def test_ollama_down_stops_with_one_sentence_and_exit_one(self):
        self.row("a")
        self.ollama.close()
        code, _, err = self.run_main("label")
        self.assertEqual(code, 1)
        self.assertEqual(len(err.strip().splitlines()), 1)
        self.assertIn("Ollama", err)
        self.assertEqual(self.labels(), [])

    def test_model_not_pulled_keeps_the_lines_already_written(self):
        self.row("a")
        self.row("b")
        self.ollama.reply(GOOD)
        self.ollama.reply("", code=404)
        code, _, err = self.run_main("label")
        self.assertEqual(code, 1)
        self.assertEqual(len(err.strip().splitlines()), 1, err)
        self.assertIn("stub-model", err)
        self.assertEqual([l["run_id"] for l in self.labels()], ["a"])

    # item 1 and 3, the flags
    def test_limit_and_only_unlabelled(self):
        self.row("a", verdict="fixed")
        self.row("b")
        self.row("c")
        self.row("d")
        self.run_main("label", "--only-unlabelled", "--limit", "2")
        self.assertEqual([l["run_id"] for l in self.labels()], ["b", "c"])
        self.run_main("label", "--only-unlabelled")
        self.assertEqual([l["run_id"] for l in self.labels()], ["b", "c", "d"])

    def test_a_run_whose_show_fails_is_still_judged_on_its_task(self):
        self.row("a")
        del self.shows["a"]
        (self.root / "shows.json").write_text(json.dumps(self.shows))
        code, _, _ = self.run_main("label")
        self.assertEqual(code, 0)
        self.assertEqual(len(self.labels()), 1)

    # item 6
    def test_agree_counts_and_table(self):
        plan = [
            ("a", "fixed", True), ("b", "fixed", False), ("c", "applied", False),
            ("d", "applied", False), ("e", "applied", True), ("f", None, True),
            ("g", "fixed", None),
        ]
        for ident, verdict, _ in plan:
            self.row(ident, verdict=verdict)
        with (self.home / "judge-labels.jsonl").open("w") as handle:
            for ident, _, need in plan:
                line = {"run_id": ident, "needs_correction": need, "failure_class": "agent",
                        "reason": "r", "judge_model": "m", "prompt_sha256": "0" * 64,
                        "judged_at": "2026-09-29T00:00:00Z"}
                if need is None:
                    line["unparsed"] = "junk"
                handle.write(json.dumps(line) + "\n")
        code, out, _ = self.run_main("agree")
        self.assertEqual(code, 0)
        self.assertIn("runs with both: 5", out)
        self.assertIn("agree: 3", out)
        self.assertIn("agreement: 3/5 = 0.60", out)
        self.assertIn("fewer than 50", out)
        self.assertIn("unparsed answers: 1", out)
        self.assertIn("human fixed    judge true 1    judge false 1", out)
        self.assertIn("human applied  judge true 1    judge false 2", out)

    def test_agree_does_not_call_the_loose_number_firm_at_fifty(self):
        for n in range(50):
            self.row(f"r{n}", verdict="fixed")
        with (self.home / "judge-labels.jsonl").open("w") as handle:
            for n in range(50):
                handle.write(json.dumps({"run_id": f"r{n}", "needs_correction": True,
                                         "judge_model": "m"}) + "\n")
        _, out, _ = self.run_main("agree")
        self.assertIn("agreement: 50/50 = 1.00", out)
        self.assertNotIn("fewer than 50", out)

    def test_help_prints_usage(self):
        for sub in ("label", "agree"):
            code, out, _ = self.run_main(sub, "--help")
            self.assertEqual(code, 0)
            self.assertIn("usage:", out)


if __name__ == "__main__":
    unittest.main()
