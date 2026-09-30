"""Approve binds to the revision the human saw, and pending lists that wait."""

import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import puppets


class ApproveRevisionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.puppets_home = Path(self.tmp) / "puppets"
        self.puppets_home.mkdir()
        self.vendors_path = Path(self.tmp) / "vendors.json"
        self.vendors_path.write_text(
            json.dumps(
                [
                    {"provider": "claude", "model": None, "available": True},
                    {"provider": "codex", "model": None, "available": True},
                    {"provider": "cursor-agent", "model": None, "available": True},
                ]
            ),
            encoding="utf-8",
        )
        self._prior = {
            key: os.environ.get(key)
            for key in ("HYDRA_PUPPETS_HOME", "HYDRA_PUPPETS_VENDORS")
        }
        os.environ["HYDRA_PUPPETS_HOME"] = str(self.puppets_home)
        os.environ["HYDRA_PUPPETS_VENDORS"] = str(self.vendors_path)

    def tearDown(self):
        for key, value in self._prior.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value
        shutil.rmtree(self.tmp)

    def write_events(self, goal_id: str, events: list[dict]) -> None:
        path = self.puppets_home / f"{goal_id}.jsonl"
        with path.open("w", encoding="utf-8") as handle:
            for index, body in enumerate(events, start=1):
                record = {"seq": index, "ts": "2026-09-30T12:00:00.000Z", **body}
                handle.write(json.dumps(record) + "\n")

    def log_text(self, goal_id: str = "g1") -> str:
        path = self.puppets_home / f"{goal_id}.jsonl"
        if not path.exists():
            return ""
        return path.read_text(encoding="utf-8")

    def invoke(self, argv: list[str]) -> tuple[int, str, str]:
        stdout, stderr = io.StringIO(), io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            try:
                code = puppets.main(argv)
            except SystemExit as exc:
                code = exc.code if isinstance(exc.code, int) else 1
        return code, stdout.getvalue(), stderr.getvalue()

    def pending(self, goal_id: str = "g1") -> list[dict]:
        code, out, err = self.invoke(["pending", goal_id, "--json"])
        self.assertEqual(code, 0, err)
        data = json.loads(out)
        self.assertIsInstance(data, list)
        return data

    def goal_event(self) -> dict:
        return {"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"}

    def task_event(self, task_id: str = "t1", grade: str = "low") -> dict:
        return {
            "event": "task",
            "id": task_id,
            "text": "do work",
            "class": "batch_scripting",
            "grade": grade,
            "deps": [],
            "paths": [],
        }

    def assignment_event(
        self, task_id: str = "t1", provider: str = "cursor-agent", grade_policy: str = "standard"
    ) -> dict:
        model = "composer-2.5" if provider == "cursor-agent" else ""
        if provider == "claude":
            model = "opus"
        return {
            "event": "assignment",
            "task": task_id,
            "provider": provider,
            "model": model,
            "policy": grade_policy,
            "source": "human",
        }

    def assigned_cursor(self) -> list[dict]:
        return [self.goal_event(), self.task_event(), self.assignment_event()]

    def mark_codex_out(self) -> None:
        rows = json.loads(self.vendors_path.read_text(encoding="utf-8"))
        for row in rows:
            if row.get("provider") == "codex":
                row["available"] = False
        self.vendors_path.write_text(json.dumps(rows), encoding="utf-8")

    def approval_bodies(self, goal_id: str) -> list[dict]:
        bodies = []
        for row in puppets.read_log(goal_id):
            if row.get("event") != "approval":
                continue
            bodies.append({key: value for key, value in row.items() if key not in ("seq", "ts")})
        return bodies

    def test_expect_revision_mismatch_writes_nothing_and_exits_2(self):
        self.write_events("g1", self.assigned_cursor())
        before = self.log_text()
        code, _out, err = self.invoke(
            ["approve", "g1", "--table", "--expect-revision", "1"]
        )
        self.assertEqual(code, 2)
        self.assertEqual(err, "expected revision 1, current revision 2\n")
        self.assertEqual(self.log_text(), before)

    def test_expect_revision_is_checked_before_any_other_refusal(self):
        self.write_events("g1", self.assigned_cursor())
        before = self.log_text()
        code, _out, err = self.invoke(
            ["approve", "g1", "--launch", "missing", "--expect-revision", "0"]
        )
        self.assertEqual(code, 2)
        self.assertEqual(err, "expected revision 0, current revision 2\n")
        self.assertNotIn("unknown task", err)
        self.assertEqual(self.log_text(), before)

    def test_matching_expect_revision_writes_the_same_approval_as_without_the_flag(self):
        events = self.assigned_cursor()
        self.write_events("g1", events)
        self.write_events("g2", events)
        code, _out, err = self.invoke(["approve", "g1", "--table"])
        self.assertEqual(code, 0, err)
        code, _out, err = self.invoke(
            ["approve", "g2", "--table", "--expect-revision", "2"]
        )
        self.assertEqual(code, 0, err)
        self.assertEqual(self.approval_bodies("g1"), self.approval_bodies("g2"))
        self.assertEqual(self.approval_bodies("g2")[0]["revision"], 2)

    def test_a_later_revision_rejects_the_revision_the_human_saw(self):
        events = self.assigned_cursor() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            self.assignment_event(provider="claude"),
        ]
        self.write_events("g1", events)
        before = self.log_text()
        state = puppets.fold("g1")
        self.assertEqual(state.revision, 3)
        code, _out, err = self.invoke(
            ["approve", "g1", "--launch", "t1", "--expect-revision", "2"]
        )
        self.assertEqual(code, 2)
        self.assertEqual(err, "expected revision 2, current revision 3\n")
        self.assertEqual(self.log_text(), before)
        code, _out, err = self.invoke(["approve", "g1", "--table"])
        self.assertEqual(code, 0, err)
        self.assertEqual(len(self.approval_bodies("g1")), 2)

    def test_pending_lists_the_table_wait_and_does_not_write(self):
        self.write_events("g1", self.assigned_cursor())
        before = self.log_text()
        items = self.pending()
        self.assertEqual(self.log_text(), before)
        self.assertEqual(len(items), 1)
        item = items[0]
        self.assertEqual(item["kind"], "table")
        self.assertIsNone(item["ref"])
        self.assertEqual(item["revision"], 2)
        argv = item["argv"]
        self.assertEqual(argv[0], sys.executable)
        self.assertTrue(argv[1].endswith("puppets.py"))
        self.assertEqual(
            argv[2:],
            ["approve", "g1", "--table", "--expect-revision", "2"],
        )

    def test_pending_argv_approves_and_the_wait_drops(self):
        self.write_events("g1", self.assigned_cursor())
        items = self.pending()
        proc = subprocess.run(items[0]["argv"], capture_output=True, text=True, check=False)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertEqual(self.approval_bodies("g1")[0]["kind"], "table")
        self.assertEqual(self.pending(), [])

    def test_pending_agrees_with_the_commands_that_refuse(self):
        # Each case folds the log the commands read. pending must name the approval
        # that command asks for, and must not name one the command does not.
        cases = {
            "table blocks launch": (
                self.assigned_cursor(),
                [("table", None)],
            ),
            "cursor launch needs no second approval": (
                self.assigned_cursor()
                + [
                    {
                        "event": "approval",
                        "kind": "table",
                        "ref": None,
                        "revision": 2,
                        "yes": True,
                    }
                ],
                [],
            ),
            "codex launch waits only after the table is approved": (
                [
                    self.goal_event(),
                    self.task_event(),
                    self.assignment_event(provider="codex"),
                    {
                        "event": "approval",
                        "kind": "table",
                        "ref": None,
                        "revision": 2,
                        "yes": True,
                    },
                ],
                [("launch", "t1")],
            ),
            "codex without a table is a table wait, not a launch wait": (
                [
                    self.goal_event(),
                    self.task_event(),
                    self.assignment_event(provider="codex"),
                ],
                [("table", None)],
            ),
        }
        for name, (events, expected) in cases.items():
            with self.subTest(name=name):
                self.write_events("g1", events)
                before = self.log_text()
                state = puppets.fold("g1")
                if expected:
                    code, _out, err = self.invoke(["launch", "g1", "t1"])
                    self.assertEqual(self.log_text(), before)
                    if any(kind == "table" for kind, _ref in expected):
                        self.assertEqual(code, 1)
                        self.assertIn("table approval", err)
                        self.assertNotIn("launch approval", err)
                    else:
                        self.assertEqual(code, 1)
                        self.assertIn("launch approval", err)
                        self.assertNotIn("table approval", err)
                else:
                    # A cursor task with a table approval would launch. Check the
                    # refusal helper instead of spawning a run.
                    self.assertIsNone(puppets.launch_refusal(state, "t1"))
                items = self.pending()
                self.assertEqual(self.log_text(), before)
                got = [(item["kind"], item["ref"]) for item in items]
                self.assertEqual(got, expected)
                for item in items:
                    self.assertEqual(item["revision"], state.revision)
                    flag = item["argv"].index("--expect-revision")
                    self.assertEqual(item["argv"][flag + 1], str(state.revision))

    def test_pending_skips_gates_the_command_refuses_for_another_reason(self):
        dangling = self.assigned_cursor() + [
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "abc",
            }
        ]
        self.write_events("g1", dangling)
        code, _out, err = self.invoke(["launch", "g1", "t1"])
        self.assertEqual(code, 1)
        self.assertIn("ambiguous launch intent", err)
        self.assertNotIn("table approval", err)
        self.assertEqual(self.pending(), [])

        unassigned = [self.goal_event(), self.task_event()]
        self.write_events("g1", unassigned)
        code, _out, err = self.invoke(["launch", "g1", "t1"])
        self.assertEqual(code, 1)
        self.assertIn("no assignment", err)
        self.assertEqual(self.pending(), [])

    def test_pending_lists_a_plan_only_when_tasks_would_wait_for_it(self):
        plan_path = self.puppets_home / "g1" / "plans" / "chosen.md"
        plan_path.parent.mkdir(parents=True)
        plan_path.write_text("plan\n", encoding="utf-8")
        self.write_events(
            "g1",
            [
                self.goal_event(),
                {
                    "event": "panel_verdict",
                    "judge_run_ids": [],
                    "chosen": "codex",
                    "path": str(plan_path),
                },
            ],
        )
        before = self.log_text()
        code, _out, err = self.invoke(["tasks", "g1"])
        self.assertEqual(code, 1)
        self.assertIn("plan approval", err)
        self.assertEqual(self.log_text(), before)
        items = self.pending()
        self.assertEqual([(item["kind"], item["ref"], item["revision"]) for item in items], [("plan", None, 0)])
        self.assertEqual(
            items[0]["argv"][2:],
            ["approve", "g1", "--plan", "--expect-revision", "0"],
        )

        self.write_events(
            "g1",
            [
                self.goal_event(),
                {
                    "event": "panel_verdict",
                    "judge_run_ids": [],
                    "chosen": "codex",
                    "path": None,
                },
            ],
        )
        code, _out, err = self.invoke(["tasks", "g1"])
        self.assertEqual(code, 1)
        self.assertIn("no chosen plan with a path", err)
        self.assertNotIn("plan approval", err)
        self.assertEqual(self.pending(), [])

    def test_pending_lists_a_diff_when_an_ended_run_waits_for_verdict(self):
        events = self.assigned_cursor() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "abc",
            },
            {"event": "run", "task": "t1", "attempt": 1, "run_id": "run-1"},
        ]
        self.write_events("g1", events)
        # verdict applied refuses as soon as the task has a run, including while
        # that run is still marked running. pending lists the same wait.
        before = self.log_text()
        code, _out, err = self.invoke(["verdict", "g1", "t1", "applied"])
        self.assertEqual(code, 1)
        self.assertIn("diff approval", err)
        self.assertEqual(self.log_text(), before)
        running = self.pending()
        self.assertEqual([(item["kind"], item["ref"]) for item in running], [("diff", "t1")])

        events.append(
            {
                "event": "run_end",
                "task": "t1",
                "run_id": "run-1",
                "outcome": "completed",
                "total_tokens": 1,
            }
        )
        self.write_events("g1", events)
        before = self.log_text()
        code, _out, err = self.invoke(["verdict", "g1", "t1", "applied"])
        self.assertEqual(code, 1)
        self.assertIn("diff approval", err)
        self.assertEqual(self.log_text(), before)
        items = self.pending()
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["kind"], "diff")
        self.assertEqual(items[0]["ref"], "t1")
        self.assertEqual(items[0]["revision"], 2)
        self.assertEqual(
            items[0]["argv"][2:],
            ["approve", "g1", "--diff", "t1", "--expect-revision", "2"],
        )

        events.append(
            {
                "event": "approval",
                "kind": "diff",
                "ref": "t1",
                "run_id": "run-1",
                "revision": 2,
                "yes": True,
            }
        )
        self.write_events("g1", events)
        self.assertEqual(self.pending(), [])

        events = [row for row in events if row.get("kind") != "diff"]
        events.append(
            {
                "event": "human_verdict",
                "task": "t1",
                "run_id": "run-1",
                "result": "rejected",
                "note": "",
            }
        )
        self.write_events("g1", events)
        code, _out, err = self.invoke(["verdict", "g1", "t1", "applied"])
        self.assertEqual(code, 1)
        self.assertIn("diff approval", err)
        self.assertEqual(self.pending(), [])

    def test_pending_lists_degraded_when_the_panel_would_ask(self):
        self.mark_codex_out()
        self.write_events("g1", [self.goal_event()])
        _code, _out, err = self.invoke(["choose", "g1", "--judge"])
        self.assertNotIn("--degraded", err)
        self.assertEqual(self.pending(), [])

        plan_dir = self.puppets_home / "g1" / "plans"
        plan_dir.mkdir(parents=True)
        codex_plan = plan_dir / "codex.md"
        cursor_plan = plan_dir / "cursor-agent.md"
        codex_plan.write_text("a\n", encoding="utf-8")
        cursor_plan.write_text("b\n", encoding="utf-8")
        self.write_events(
            "g1",
            [
                self.goal_event(),
                {"event": "plan_proposed", "provider": "codex", "run_id": "r1"},
                {"event": "plan_ready", "provider": "codex", "run_id": "r1", "path": str(codex_plan)},
                {"event": "plan_proposed", "provider": "cursor-agent", "run_id": "r2"},
                {
                    "event": "plan_ready",
                    "provider": "cursor-agent",
                    "run_id": "r2",
                    "path": str(cursor_plan),
                },
            ],
        )
        before = self.log_text()
        code, _out, err = self.invoke(["choose", "g1", "--judge"])
        self.assertEqual(code, 1)
        self.assertIn("--degraded", err)
        self.assertEqual(self.log_text(), before)
        state = puppets.fold("g1")
        _resolved, _subs, refusal = puppets.resolve_panel_seats(state)
        self.assertIsNotNone(refusal)
        items = self.pending()
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["kind"], "degraded")
        self.assertEqual(items[0]["ref"], "codex")
        self.assertEqual(items[0]["revision"], state.revision)
        spec = "codex=<substitute-provider[:model]>"
        self.assertIn(spec, items[0]["argv"])
        self.assertIn(spec, refusal)
        self.assertEqual(items[0]["argv"][-2:], ["--expect-revision", str(state.revision)])

        proc = subprocess.run(items[0]["argv"], capture_output=True, text=True, check=False)
        self.assertNotEqual(proc.returncode, 0)
        self.assertEqual(self.log_text(), before)

        code, _out, err = self.invoke(
            [
                "approve",
                "g1",
                "--degraded",
                "codex=cursor-agent:composer-2.5",
                "--expect-revision",
                str(state.revision),
            ]
        )
        self.assertEqual(code, 0, err)
        self.assertEqual(self.pending(), [])

    def test_pending_names_only_the_seat_the_panel_refusal_names(self):
        rows = json.loads(self.vendors_path.read_text(encoding="utf-8"))
        for row in rows:
            if row.get("provider") in ("claude", "codex"):
                row["available"] = False
        self.vendors_path.write_text(json.dumps(rows), encoding="utf-8")
        plan_dir = self.puppets_home / "g1" / "plans"
        plan_dir.mkdir(parents=True)
        for name in ("codex", "cursor-agent"):
            (plan_dir / f"{name}.md").write_text(name + "\n", encoding="utf-8")
        self.write_events(
            "g1",
            [
                self.goal_event(),
                {"event": "plan_proposed", "provider": "codex", "run_id": "r1"},
                {
                    "event": "plan_ready",
                    "provider": "codex",
                    "run_id": "r1",
                    "path": str(plan_dir / "codex.md"),
                },
                {"event": "plan_proposed", "provider": "cursor-agent", "run_id": "r2"},
                {
                    "event": "plan_ready",
                    "provider": "cursor-agent",
                    "run_id": "r2",
                    "path": str(plan_dir / "cursor-agent.md"),
                },
            ],
        )
        code, _out, err = self.invoke(["choose", "g1", "--judge"])
        self.assertEqual(code, 1)
        self.assertIn("claude is unavailable", err)
        self.assertNotIn("codex is unavailable", err)
        items = self.pending()
        self.assertEqual([item["ref"] for item in items], ["claude"])

    def test_a_task_run_makes_a_down_seat_pending_before_choose_would_ask(self):
        self.mark_codex_out()
        events = self.assigned_cursor() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "abc",
            },
            {"event": "run", "task": "t1", "attempt": 1, "run_id": "run-1"},
        ]
        self.write_events("g1", events)
        _code, _out, err = self.invoke(["choose", "g1", "--judge"])
        self.assertNotIn("--degraded", err)
        state = puppets.fold("g1")
        _resolved, _subs, refusal = puppets.resolve_panel_seats(state)
        self.assertIn("codex is unavailable", refusal)
        items = self.pending()
        self.assertEqual(
            [(item["kind"], item["ref"]) for item in items],
            [("diff", "t1"), ("degraded", "codex")],
        )

    def test_a_spent_launch_approval_is_not_listed_for_a_task_already_running(self):
        # `launch` would refuse a second time, and that refusal is not an open wait:
        # the approval was spent by the run already in flight. The diff still is.
        events = [
            self.goal_event(),
            self.task_event(),
            self.assignment_event(provider="codex"),
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "approval",
                "kind": "launch",
                "ref": "t1",
                "revision": 2,
                "yes": True,
            },
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "codex",
                "model": "",
                "policy": "standard",
                "argv_sha256": "abc",
            },
            {"event": "run", "task": "t1", "attempt": 1, "run_id": "run-1"},
        ]
        self.write_events("g1", events)
        state = puppets.fold("g1")
        self.assertEqual(state.tasks["t1"].status, "running")
        refusal = puppets.launch_refusal(state, "t1")
        self.assertIsNotNone(refusal)
        self.assertIn("launch approval", refusal)
        items = self.pending()
        self.assertEqual([(item["kind"], item["ref"]) for item in items], [("diff", "t1")])

    def test_pending_unknown_goal_writes_nothing(self):
        code, _out, err = self.invoke(["pending", "missing", "--json"])
        self.assertEqual(code, 1)
        self.assertIn("missing", err)
        self.assertEqual(list(self.puppets_home.iterdir()), [])

    def test_a_voided_table_approval_is_waiting_again_at_the_new_revision(self):
        events = self.assigned_cursor() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            self.assignment_event(provider="claude"),
        ]
        self.write_events("g1", events)
        state = puppets.fold("g1")
        self.assertEqual(state.revision, 3)
        self.assertIsNone(puppets.approval_available(state, "table"))
        items = self.pending()
        self.assertEqual([(item["kind"], item["revision"]) for item in items], [("table", 3)])
        self.assertEqual(items[0]["argv"][-1], "3")


if __name__ == "__main__":
    unittest.main()
