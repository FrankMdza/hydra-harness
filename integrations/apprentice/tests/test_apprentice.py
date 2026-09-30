"""The apprentice, against the fixture records, generated records and a stub Ollama."""

from __future__ import annotations

import contextlib
import hashlib
import importlib.util
import io
import json
import os
import plistlib
import shutil
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from unittest import mock

APPRENTICE = Path(__file__).resolve().parents[1]
REPO = APPRENTICE.parents[1]
FIXTURES = REPO / "tests" / "fixtures" / "records"
spec = importlib.util.spec_from_file_location("apprentice_main", APPRENTICE / "apprentice.py")
apprentice = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = apprentice
spec.loader.exec_module(apprentice)

ID_INTERACTIVE = "00000000-0000-7000-8000-000000000101"
ID_SECRET = "00000000-0000-7000-8000-000000000102"
SECRET = "sk-ant-" + "a1B2c3D4" * 5


def fixture_id(n: int) -> str:
    return f"00000000-0000-7000-8000-{n:012d}"


class Ollama:
    """Serves queued replies to /api/generate and keeps every request body."""

    def __init__(self) -> None:
        self.replies: list = []
        self.requests: list[dict] = []
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                body = self.rfile.read(int(self.headers["Content-Length"]))
                owner.requests.append(json.loads(body))
                reply = owner.replies.pop(0) if owner.replies else {"reason": "r", "answer": "x"}
                text = reply if isinstance(reply, str) else json.dumps(reply)
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(json.dumps({"response": text, "done": True}).encode())

            def log_message(self, *args):
                pass

        self.server = HTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.server.server_port}/api/generate"

    def close(self) -> None:
        self.server.shutdown()
        self.server.server_close()


def write_record(runs: Path, run_id: str, lines: list[dict]) -> None:
    (runs / f"{run_id}.jsonl").write_text("".join(json.dumps(l) + "\n" for l in lines))


def start(run_id: str, **fields) -> dict:
    hydra = {"type": "run_start", "command": "run", "policy": "standard", "launch_mode": "recorded",
             "task": "probe"}
    hydra.update(fields)
    return {"hydra": hydra, "model": None, "provider": "claude", "run_id": run_id, "seq": 0,
            "ts": "2026-09-05T20:51:39.370Z"}


def end(run_id: str, seq: int, **fields) -> dict:
    hydra = {"type": "run_end", "exit_code": 0, "refusal": None, "error": None}
    hydra.update(fields)
    return {"hydra": hydra, "provider": "claude", "run_id": run_id, "seq": seq,
            "ts": "2026-09-05T20:52:01.905Z"}


class Base(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.runs = self.root / "runs"
        self.runs.mkdir()
        for path in FIXTURES.glob("*/*.jsonl"):
            shutil.copy(path, self.runs / path.name)
        write_record(self.runs, ID_INTERACTIVE, [
            start(ID_INTERACTIVE, launch_mode="interactive", task=None),
            {"hydra": {"type": "seat_request", "request_id": "r1", "shape": "review", "seats": 3,
                       "task": "look", "decision": "launched", "reason": None,
                       "runs": ["a", "b", "c"]},
             "provider": "claude", "run_id": ID_INTERACTIVE, "seq": 1, "ts": "2026-09-05T20:51:40Z"},
            {"hydra": {"type": "seat_request", "request_id": "r2", "shape": "x", "seats": 5,
                       "task": "no", "decision": "refused", "reason": "no shape", "runs": []},
             "provider": "claude", "run_id": ID_INTERACTIVE, "seq": 2, "ts": "2026-09-05T20:51:41Z"},
            end(ID_INTERACTIVE, 3),
        ])
        write_record(self.runs, ID_SECRET, [
            start(ID_SECRET, task=f"use the key {SECRET} to deploy"), end(ID_SECRET, 1)])
        self.home = self.root / "grader"
        env = {"HYDRA_ROOT": str(self.root), "HYDRA_GRADER_HOME": str(self.home)}
        patcher = mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.apprentice_home = self.home / "apprentice"

    def run_main(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = apprentice.main(list(argv))
        return code, out.getvalue(), err.getvalue()

    def bank(self, n: int = 20) -> list[dict]:
        code, _, err = self.run_main("bank", "build", "--n", str(n))
        self.assertEqual(code, 0, err)
        return [json.loads(l) for l in (self.apprentice_home / "bank.jsonl").read_text().splitlines()]

    def task(self, template: str, run_id: str) -> dict:
        found = [t for t in self.bank(500) if t["template"] == template and t["run_id"] == run_id]
        self.assertEqual(len(found), 1, f"{template} for {run_id}")
        return found[0]


class BankTests(Base):
    def test_same_records_give_the_same_bank_whatever_the_listing_order(self):
        first = self.bank()
        text = (self.apprentice_home / "bank.jsonl").read_bytes()
        again = self.bank()
        self.assertEqual(first, again)
        self.assertEqual(text, (self.apprentice_home / "bank.jsonl").read_bytes())
        real = apprentice.list_records
        with mock.patch.object(apprentice, "list_records", lambda runs: list(reversed(real(runs)))):
            shuffled = apprentice.build_bank(self.runs, 20)
        self.assertEqual(first, shuffled)

    def test_n_caps_each_class_and_both_classes_are_present(self):
        bank = self.bank(4)
        classes = [t["class"] for t in bank]
        self.assertEqual(classes.count("verifiable"), 4)
        self.assertEqual(classes.count("open"), 4)

    def test_a_bigger_n_than_the_records_allow_gives_fewer_tasks_not_an_error(self):
        bank = self.bank(500)
        self.assertGreater(len(bank), 8)
        self.assertLess(len(bank), 1000)

    def test_at_least_six_verifiable_and_four_open_templates_exist(self):
        bank = self.bank(200)
        self.assertGreaterEqual(len({t["template"] for t in bank if t["class"] == "verifiable"}), 6)
        self.assertGreaterEqual(len({t["template"] for t in bank if t["class"] == "open"}), 4)

    def test_verifiable_task_shape(self):
        for t in self.bank(200):
            self.assertTrue({"id", "template", "class", "prompt", "run_id", "harness"} <= set(t))
            if t["class"] == "verifiable":
                self.assertIn(t["check"], ("exact", "number", "one_of"))
                self.assertIn("answer", t)
            else:
                self.assertNotIn("answer", t)
            self.assertLessEqual(len(t["prompt"]), 3000 + 1500)

    def test_outcome(self):
        self.assertEqual(self.task("outcome", fixture_id(1))["answer"], "success")
        self.assertEqual(self.task("outcome", fixture_id(5))["answer"], "refused")

    def test_provider_model(self):
        t = self.task("provider_model", fixture_id(1))
        self.assertEqual(t["check"], "one_of")
        self.assertIn("claude/claude-opus-5[1m]", t["answer"])

    def test_refused_count_and_first_refused_tool(self):
        self.assertEqual(self.task("refused_count", fixture_id(1))["answer"], 1)
        self.assertEqual(self.task("refused_count", fixture_id(1))["check"], "number")
        self.assertEqual(self.task("first_refused_tool", fixture_id(1))["answer"], "Write")
        bank = self.bank(500)
        first = {t["run_id"] for t in bank if t["template"] == "first_refused_tool"}
        counted = {t["run_id"] for t in bank if t["template"] == "refused_count" and t["answer"] > 0}
        self.assertEqual(first, counted, "only a run with a refusal has a first refused tool")
        self.assertTrue(any(t["answer"] == 0 for t in bank if t["template"] == "refused_count"))

    def test_interactive(self):
        self.assertEqual(self.task("interactive", ID_INTERACTIVE)["answer"], "yes")
        self.assertEqual(self.task("interactive", fixture_id(1))["answer"], "no")

    def test_seats_launched_counts_only_launched_requests(self):
        self.assertEqual(self.task("seats_launched", ID_INTERACTIVE)["answer"], 3)

    def test_policy(self):
        self.assertEqual(self.task("policy", fixture_id(1))["answer"], "read-only")
        self.assertEqual(self.task("policy", fixture_id(3))["answer"], "none")

    def test_open_templates(self):
        templates = {t["template"] for t in self.bank(200) if t["class"] == "open"}
        self.assertEqual(templates, {"run_line", "provider_policy", "summarize", "refusal_meaning"})

    def test_prompts_are_redacted_and_cut(self):
        for t in self.bank(200):
            self.assertNotIn(SECRET, t["prompt"])
        run_line = self.task("run_line", ID_SECRET)
        self.assertIn("[REDACTED", run_line["prompt"])
        long_id = "00000000-0000-7000-8000-000000000103"
        write_record(self.runs, long_id, [start(long_id, task="word " * 2000), end(long_id, 1)])
        big = self.task("run_line", long_id)
        self.assertIn("[cut:", big["prompt"])
        self.assertLess(len(big["prompt"]), 3000 + 1500)

    def test_bank_lives_under_the_grader_home(self):
        self.bank()
        self.assertTrue((self.apprentice_home / "bank.jsonl").is_file())


class AttemptTests(Base):
    def setUp(self):
        super().setUp()
        self.stub = Ollama()
        self.addCleanup(self.stub.close)
        patcher = mock.patch.dict(os.environ, {"HYDRA_LOCAL_LLM_URL": self.stub.url,
                                               "HYDRA_LOCAL_MODEL": "stub-model"})
        patcher.start()
        self.addCleanup(patcher.stop)
        self.bank(2)
        self.tasks = {}
        for line in (self.apprentice_home / "bank.jsonl").read_text().splitlines():
            t = json.loads(line)
            self.tasks[t["id"]] = t

    def attempts(self) -> list[dict]:
        path = self.apprentice_home / "attempts.jsonl"
        return [json.loads(l) for l in path.read_text().splitlines()] if path.exists() else []

    def test_attempt_writes_one_line_per_task_and_grades_verifiable_ones(self):
        first = next(t for t in self.tasks.values() if t["class"] == "verifiable")
        answer = first["answer"] if isinstance(first["answer"], (str, int)) else first["answer"][0]
        wrong = {"reason": "guess", "answer": "zzz-wrong"}
        # the bank is sorted by id; give the first verifiable task the right answer, the rest wrong.
        order = sorted(self.tasks)
        self.stub.replies = [
            {"reason": "read it", "answer": answer} if tid == first["id"] else wrong for tid in order
        ]
        code, out, err = self.run_main("attempt")
        self.assertEqual(code, 0, err)
        lines = self.attempts()
        self.assertEqual(len(lines), len(self.tasks))
        by_id = {l["task_id"]: l for l in lines}
        self.assertIs(by_id[first["id"]]["correct"], True)
        for l in lines:
            self.assertEqual(l["model"], "stub-model")
            self.assertEqual(len(l["prompt_sha"]), 64)
            self.assertIn("seconds", l)
            self.assertIn("reason", l)
            if self.tasks[l["task_id"]]["class"] == "open":
                self.assertNotIn("correct", l)
            elif l["task_id"] != first["id"]:
                self.assertIs(l["correct"], False)

    def test_request_is_reason_first_json_at_temperature_zero_with_a_cap(self):
        self.run_main("attempt", "--limit", "1")
        body = self.stub.requests[0]
        self.assertEqual(body["model"], "stub-model")
        self.assertEqual(body["format"], "json")
        self.assertEqual(body["options"]["temperature"], 0)
        self.assertGreater(body["options"]["num_predict"], 0)
        self.assertLess(body["options"]["num_predict"], 1000)
        self.assertLess(body["prompt"].index('"reason"'), body["prompt"].index('"answer"'))

    def test_limit_and_resume_skip_tasks_already_attempted_at_this_sha(self):
        self.run_main("attempt", "--limit", "2")
        self.assertEqual(len(self.attempts()), 2)
        self.run_main("attempt")
        lines = self.attempts()
        self.assertEqual(len(lines), len(self.tasks))
        self.assertEqual(len({l["task_id"] for l in lines}), len(self.tasks))
        self.run_main("attempt")
        self.assertEqual(len(self.attempts()), len(self.tasks))

    def test_a_changed_prompt_asks_every_task_again(self):
        self.run_main("attempt")
        with mock.patch.object(apprentice, "INSTRUCTIONS", apprentice.INSTRUCTIONS + "\nBe brief."):
            self.run_main("attempt")
        lines = self.attempts()
        self.assertEqual(len(lines), 2 * len(self.tasks))
        self.assertEqual(len({l["prompt_sha"] for l in lines}), 2)

    def test_the_minute_budget_stops_the_pass_and_keeps_what_it_wrote(self):
        ticks = iter([0.0, 0.0, 10.0, 20.0, 61.0, 61.0, 61.0, 61.0, 61.0, 61.0, 61.0, 61.0, 61.0, 61.0])
        with mock.patch.object(apprentice, "clock", lambda: next(ticks)):
            code, out, err = self.run_main("attempt", "--minutes", "1")
        self.assertEqual(code, 0, err)
        written = len(self.attempts())
        self.assertGreaterEqual(written, 1)
        self.assertLess(written, len(self.tasks))
        self.assertIn("minute", out + err)

    def test_ollama_down_stops_with_one_line_exit_one_and_keeps_earlier_lines(self):
        self.run_main("attempt", "--limit", "1")
        self.assertEqual(len(self.attempts()), 1)
        self.stub.close()
        code, out, err = self.run_main("attempt")
        self.assertEqual(code, 1)
        self.assertEqual(len(err.strip().splitlines()), 1, err)
        self.assertIn("Ollama", err)
        self.assertEqual(len(self.attempts()), 1)

    def test_an_answer_that_is_not_json_is_kept_and_counted_wrong(self):
        self.stub.replies = ["not json at all"] * len(self.tasks)
        self.run_main("attempt")
        for l in self.attempts():
            self.assertEqual(l["answer"], "")
            if self.tasks[l["task_id"]]["class"] == "verifiable":
                self.assertIs(l["correct"], False)

    def test_only_the_local_url_is_called(self):
        with mock.patch("urllib.request.urlopen", wraps=__import__("urllib.request").request.urlopen) as spy:
            self.run_main("attempt", "--limit", "2")
        for call in spy.call_args_list:
            self.assertEqual(call.args[0].full_url, self.stub.url)


class CheckTests(unittest.TestCase):
    def test_checks(self):
        g = apprentice.grade
        self.assertTrue(g({"check": "exact", "answer": "success"}, " Success. "))
        self.assertFalse(g({"check": "exact", "answer": "success"}, "failed"))
        self.assertTrue(g({"check": "number", "answer": 3}, "3"))
        self.assertTrue(g({"check": "number", "answer": 3}, 3))
        self.assertFalse(g({"check": "number", "answer": 3}, "13"))
        self.assertTrue(g({"check": "one_of", "answer": ["a/b", "a"]}, "A"))
        self.assertFalse(g({"check": "one_of", "answer": ["a/b", "a"]}, "b"))


class ScoreTests(Base):
    def put(self, rows: list[dict]) -> None:
        self.apprentice_home.mkdir(parents=True, exist_ok=True)
        (self.apprentice_home / "bank.jsonl").write_text("".join(json.dumps(r) + "\n" for r in [
            {"id": "outcome:a", "template": "outcome", "class": "verifiable"},
            {"id": "outcome:b", "template": "outcome", "class": "verifiable"},
            {"id": "policy:a", "template": "policy", "class": "verifiable"},
            {"id": "summarize:a", "template": "summarize", "class": "open"},
        ]))
        (self.apprentice_home / "attempts.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))

    @staticmethod
    def row(task, sha, correct):
        r = {"task_id": task, "prompt_sha": sha, "answer": "x", "reason": "", "model": "m", "seconds": 1.0}
        if correct is not None:
            r["correct"] = correct
        return r

    def test_score_per_template_overall_and_delta_against_the_previous_sha(self):
        self.put([
            self.row("outcome:a", "old", False), self.row("outcome:b", "old", False),
            self.row("policy:a", "old", True), self.row("summarize:a", "old", None),
            self.row("outcome:a", "new", True), self.row("outcome:b", "new", False),
            self.row("policy:a", "new", True),
        ])
        code, out, err = self.run_main("score")
        self.assertEqual(code, 0, err)
        self.assertIn("overall", out)
        lines = {l.split()[0]: l for l in out.splitlines() if l.split()}
        self.assertIn("2/3", lines["overall"])
        self.assertIn("1/3", out)
        self.assertIn("outcome", lines)
        self.assertIn("1/2", lines["outcome"])
        self.assertIn("+", lines["outcome"])
        self.assertIn("policy", lines)
        self.assertNotIn("summarize", lines, "open tasks are not scored")

    def test_the_latest_attempt_per_task_counts(self):
        self.put([self.row("outcome:a", "s", False), self.row("outcome:a", "s", True),
                  self.row("outcome:b", "s", False), self.row("policy:a", "s", False)])
        code, out, _ = self.run_main("score")
        lines = {l.split()[0]: l for l in out.splitlines() if l.split()}
        self.assertIn("1/3", lines["overall"])

    def test_one_sha_prints_no_delta_and_no_attempts_says_so(self):
        self.put([self.row("outcome:a", "s", True)])
        _, out, _ = self.run_main("score")
        self.assertNotIn("previous", out)
        (self.apprentice_home / "attempts.jsonl").unlink()
        code, out, _ = self.run_main("score")
        self.assertEqual(code, 0)
        self.assertIn("no attempts", out)


class PairsTests(Base):
    def setUp(self):
        super().setUp()
        self.bank(4)
        self.open = [t for t in json.loads("[" + ",".join(
            (self.apprentice_home / "bank.jsonl").read_text().splitlines()) + "]") if t["class"] == "open"]

    def graded(self, rows: list[dict]) -> Path:
        path = self.root / "graded.jsonl"
        path.write_text("".join(json.dumps(r) + "\n" for r in rows))
        return path

    def line(self, i: int, verdict: str) -> dict:
        return {"task_id": self.open[i]["id"], "local_answer": f"local {i}",
                "better_answer": f"better {i}", "verdict": verdict, "graded_by": "panel-a"}

    def test_only_better_and_worse_become_pairs(self):
        path = self.graded([self.line(0, "better"), self.line(1, "same"), self.line(2, "worse")])
        code, out, err = self.run_main("pairs", str(path))
        self.assertEqual(code, 0, err)
        pairs = [json.loads(l) for l in (self.apprentice_home / "pairs.jsonl").read_text().splitlines()]
        self.assertEqual(len(pairs), 2)
        by_prompt = {p["prompt"][0]["content"]: p for p in pairs}
        better = by_prompt[self.open[0]["prompt"]]
        worse = by_prompt[self.open[2]["prompt"]]
        # the verdict is on the local answer: better keeps it, worse swaps it for the panel's.
        self.assertEqual(better["chosen"][0]["content"], "local 0")
        self.assertEqual(better["rejected"][0]["content"], "better 0")
        self.assertEqual(worse["chosen"][0]["content"], "better 2")
        self.assertEqual(worse["rejected"][0]["content"], "local 2")

    def test_pair_carries_harness_grader_and_time(self):
        self.run_main("pairs", str(self.graded([self.line(0, "worse")])))
        pair = json.loads((self.apprentice_home / "pairs.jsonl").read_text().splitlines()[0])
        self.assertEqual(pair["graded_by"], "panel-a")
        self.assertEqual(pair["harness"], self.open[0]["harness"])
        self.assertRegex(pair["graded_at"], r"^\d{4}-\d\d-\d\dT")
        self.assertEqual(pair["prompt"][0]["role"], "user")

    def test_unknown_task_and_bad_verdict_are_named_not_written(self):
        bad = self.line(0, "great")
        ghost = dict(self.line(1, "better"), task_id="nope:none")
        code, out, err = self.run_main("pairs", str(self.graded([bad, ghost])))
        self.assertEqual(code, 0)
        text = out + err
        self.assertIn("nope:none", text)
        self.assertIn("great", text)
        path = self.apprentice_home / "pairs.jsonl"
        self.assertEqual(path.read_text() if path.exists() else "", "")

    def test_pairs_are_redacted_and_rerunning_does_not_duplicate(self):
        row = dict(self.line(0, "worse"), better_answer=f"use {SECRET}")
        path = self.graded([row])
        self.run_main("pairs", str(path))
        self.run_main("pairs", str(path))
        text = (self.apprentice_home / "pairs.jsonl").read_text()
        self.assertNotIn(SECRET, text)
        self.assertEqual(len(text.splitlines()), 1)


class LaunchdTests(unittest.TestCase):
    def test_template_runs_attempt_at_four_for_thirty_minutes_and_logs_under_the_home(self):
        with (APPRENTICE / "launchd" / "co.hephaestuslab.hydra-apprentice.plist").open("rb") as f:
            job = plistlib.load(f)
        self.assertEqual(job["Label"], "co.hephaestuslab.hydra-apprentice")
        self.assertEqual(job["StartCalendarInterval"], {"Hour": 4, "Minute": 0})
        args = job["ProgramArguments"]
        self.assertTrue(args[1].endswith("integrations/apprentice/apprentice.py"))
        self.assertEqual(args[2:], ["attempt", "--minutes", "30"])
        home = job["EnvironmentVariables"]["HYDRA_GRADER_HOME"]
        self.assertTrue(job["StandardOutPath"].startswith(home + "/apprentice/"))
        self.assertNotIn("RunAtLoad", job)


if __name__ == "__main__":
    unittest.main()
