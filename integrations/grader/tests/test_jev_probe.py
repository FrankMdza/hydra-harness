import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import threading
import unittest
import unittest.mock
from http.server import BaseHTTPRequestHandler, HTTPServer
from io import StringIO
from pathlib import Path

MODULE = Path(__file__).resolve().parents[1] / "jev_probe.py"
spec = importlib.util.spec_from_file_location("jev_probe", MODULE)
jev_probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(jev_probe)


class JevProbeBuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.build = self.root / ".scratch/hydra/build"
        self.build.mkdir(parents=True)
        (self.root / "src").mkdir()
        self._prior = os.getcwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, self._prior)

    def git(self, *args: str) -> None:
        subprocess.run(["git", *args], check=True, capture_output=True, text=True)

    def init_repo(self) -> None:
        self.git("init", "-b", "main")
        self.git("config", "user.email", "test@example.com")
        self.git("config", "user.name", "test")

    def write_ticket(self, num: int, slug: str, body: str) -> None:
        (self.build / f"{num:03d}-{slug}.md").write_text(body, encoding="utf-8")

    def test_build_labels_and_skips(self):
        self.init_repo()
        (self.root / "src/a.rs").write_text("fn a() {}\n", encoding="utf-8")
        self.write_ticket(
            200,
            "changed-one",
            textwrap.dedent(
                """\
                Status: resolved

                ## Outcome

                Built on run 01234567-abcd under standard. The session changed the patch.
                """
            ),
        )
        self.write_ticket(
            201,
            "as-written",
            textwrap.dedent(
                """\
                Status: resolved

                ## Outcome

                Built on run 89abcdef-1234. The session applied the diff as written.
                """
            ),
        )
        self.write_ticket(
            202,
            "no-outcome",
            textwrap.dedent(
                """\
                Status: resolved

                ## Outcome

                Landed without naming a run.
                """
            ),
        )
        self.git("add", ".")
        self.git("commit", "-m", "seed")

        (self.root / "src/a.rs").write_text("fn a() { 1 }\n", encoding="utf-8")
        self.git("add", "src/a.rs")
        self.git("commit", "-m", "Build 200: touch a")

        (self.root / "src/b.rs").write_text("fn b() {}\n", encoding="utf-8")
        self.git("add", "src/b.rs")
        self.git("commit", "-m", "Build 201: touch b")

        out = self.root / "rows.jsonl"
        buf = StringIO()
        old_stdout = sys.stdout
        sys.stdout = buf
        try:
            rc = jev_probe.cmd_build(out)
        finally:
            sys.stdout = old_stdout
        self.assertEqual(rc, 0)
        self.assertIn("changed: 1", buf.getvalue())
        self.assertIn("as_written: 1", buf.getvalue())
        self.assertIn("skip: 202:", buf.getvalue())

        rows = [json.loads(line) for line in out.read_text(encoding="utf-8").splitlines()]
        by_ticket = {row["ticket"]: row for row in rows}
        self.assertEqual(by_ticket[200]["label"], "changed")
        self.assertIn("fn a() { 1 }", by_ticket[200]["diff"])
        self.assertEqual(by_ticket[201]["label"], "as_written")
        self.assertIn("fn b()", by_ticket[201]["diff"])


class StubHandler(BaseHTTPRequestHandler):
    last_payload: dict | None = None

    def log_message(self, *_args) -> None:
        return

    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length)
        StubHandler.last_payload = json.loads(raw.decode("utf-8"))
        body = json.dumps(
            {
                "model": "jev-stub-1.0",
                "answers": {
                    key: {"type": "noul", "noul": 0.25}
                    for key in jev_probe.QUESTIONS
                },
            }
        ).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class JevProbeAskTests(unittest.TestCase):
    def test_ask_hits_stub_and_writes_answers(self):
        server = HTTPServer(("127.0.0.1", 0), StubHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        port = server.server_address[1]
        inp = tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8")
        out = tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8")
        try:
            inp.write(
                json.dumps(
                    {
                        "ticket": 127,
                        "label": "as_written",
                        "diff": "diff --git a/src/x.rs\n+fn x() {}\n",
                    }
                )
                + "\n"
            )
            inp.close()
            out.close()
            env = os.environ.copy()
            env["TYPESAFE_API_KEY"] = "test-key"
            env["TYPESAFE_API_BASE"] = f"http://127.0.0.1:{port}"
            old = os.environ
            os.environ = env
            try:
                rc = jev_probe.cmd_ask(Path(inp.name), Path(out.name))
            finally:
                os.environ = old
            self.assertEqual(rc, 0)
            payload = StubHandler.last_payload
            self.assertIsNotNone(payload)
            self.assertEqual(payload["model"], jev_probe.MODEL)
            self.assertEqual(payload["state"], "diff --git a/src/x.rs\n+fn x() {}\n")
            self.assertEqual(set(payload["questions"]), set(jev_probe.QUESTIONS))
            for entry in payload["questions"].values():
                self.assertEqual(entry["type"], "noul")
                self.assertIn("instructions", entry)
            row = json.loads(Path(out.name).read_text(encoding="utf-8").strip())
            self.assertEqual(row["model"], "jev-stub-1.0")
            self.assertEqual(set(row["answers"]), set(jev_probe.QUESTIONS))
        finally:
            server.shutdown()
            server.server_close()
            Path(inp.name).unlink(missing_ok=True)
            Path(out.name).unlink(missing_ok=True)


class JevProbeScoreTests(unittest.TestCase):
    def test_score_prints_expected_brier(self):
        answers = tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8")
        try:
            rows = [
                {
                    "ticket": 1,
                    "label": "changed",
                    "answers": {"needs_correction": 0.8},
                },
                {
                    "ticket": 2,
                    "label": "as_written",
                    "answers": {"needs_correction": 0.2},
                },
                {
                    "ticket": 3,
                    "label": "changed",
                    "answers": {"needs_correction": 0.6},
                },
            ]
            answers.writelines(json.dumps(r) + "\n" for r in rows)
            answers.close()
            buf = StringIO()
            old_stdout = sys.stdout
            sys.stdout = buf
            try:
                rc = jev_probe.cmd_score(Path(answers.name))
            finally:
                sys.stdout = old_stdout
            self.assertEqual(rc, 0)
            text = buf.getvalue()
            self.assertIn("brier: 0.080000", text)
            self.assertIn("accuracy_at_0.5: 1.000000", text)
            self.assertIn("thirty", text.lower())
        finally:
            Path(answers.name).unlink(missing_ok=True)


class JevProbePatchesTests(unittest.TestCase):
    def test_patches_reads_the_agents_diffs_with_their_labels(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "a.patch").write_text("diff --git a/x b/x\n+one\n", encoding="utf-8")
            (root / "b.patch").write_text("diff --git a/y b/y\n+two\n", encoding="utf-8")
            labels = root / "labels.json"
            labels.write_text(json.dumps([
                {"ticket": 1, "label": "changed", "patch": str(root / "a.patch"), "note": ""},
                {"ticket": 2, "label": "as_written", "patch": str(root / "b.patch"), "note": ""},
            ]), encoding="utf-8")
            out = root / "rows.jsonl"
            with unittest.mock.patch("sys.stdout", new_callable=StringIO) as printed:
                self.assertEqual(jev_probe.cmd_patches(labels, out), 0)
            rows = [json.loads(line) for line in out.read_text(encoding="utf-8").splitlines()]
            self.assertEqual([r["label"] for r in rows], ["changed", "as_written"])
            self.assertIn("+one", rows[0]["diff"])
            self.assertIn("changed: 1", printed.getvalue())


class JevProbeAucTests(unittest.TestCase):
    def test_rank_auc_counts_ties_as_half(self):
        pairs = [(0.9, 1.0), (0.2, 0.0), (0.2, 1.0), (0.1, 0.0)]
        # (0.9 beats 0.2 and 0.1) + (0.2 ties 0.2, beats 0.1) = 2 + 1.5 over 4
        self.assertAlmostEqual(jev_probe.rank_auc(pairs), 3.5 / 4)


if __name__ == "__main__":
    unittest.main()
