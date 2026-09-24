import json
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import puppets


class PuppetsTestCase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.puppets_home = Path(self.tmp) / "puppets"
        self.puppets_home.mkdir()
        self.hydra_root = Path(self.tmp) / "hydra"
        (self.hydra_root / "runs").mkdir(parents=True)
        self.stub = Path(self.tmp) / "hydra-stub"
        self.stub.write_text(
            textwrap.dedent(
                """\
                #!/bin/bash
                set -e
                cmd="$1"
                shift
                if [ "$cmd" = "run" ]; then
                  id=$(python3 -c 'import uuid; print(uuid.uuid4())')
                  echo "hydra: run $id" >&2
                  root="${HYDRA_ROOT:-$HOME/hydra}"
                  printf '%s\\n' '{"hydra":{"type":"run_start"}}' '{"hydra":{"type":"run_end"}}' > "$root/runs/$id.jsonl"
                  exit 0
                fi
                if [ "$cmd" = "show" ]; then
                  echo '{"outcome":"completed","usage":{"counts":{"total_tokens":42}}}'
                  exit 0
                fi
                echo "unknown command $cmd" >&2
                exit 1
                """
            ),
            encoding="utf-8",
        )
        self.stub.chmod(0o755)
        self.env = os.environ.copy()
        self.env["HYDRA_PUPPETS_HOME"] = str(self.puppets_home)
        self.env["HYDRA_ROOT"] = str(self.hydra_root)
        self.env["HYDRA_BIN"] = str(self.stub)
        self._prior_puppets_home = os.environ.get("HYDRA_PUPPETS_HOME")
        self._prior_hydra_root = os.environ.get("HYDRA_ROOT")
        self._prior_hydra_bin = os.environ.get("HYDRA_BIN")
        os.environ["HYDRA_PUPPETS_HOME"] = str(self.puppets_home)
        os.environ["HYDRA_ROOT"] = str(self.hydra_root)
        os.environ["HYDRA_BIN"] = str(self.stub)

    def tearDown(self):
        if self._prior_puppets_home is None:
            os.environ.pop("HYDRA_PUPPETS_HOME", None)
        else:
            os.environ["HYDRA_PUPPETS_HOME"] = self._prior_puppets_home
        if self._prior_hydra_root is None:
            os.environ.pop("HYDRA_ROOT", None)
        else:
            os.environ["HYDRA_ROOT"] = self._prior_hydra_root
        if self._prior_hydra_bin is None:
            os.environ.pop("HYDRA_BIN", None)
        else:
            os.environ["HYDRA_BIN"] = self._prior_hydra_bin
        shutil.rmtree(self.tmp)

    def goal(self, goal_id: str = "g1") -> str:
        return goal_id

    def write_events(self, goal_id: str, events: list[dict]) -> None:
        path = self.puppets_home / f"{goal_id}.jsonl"
        with path.open("w", encoding="utf-8") as handle:
            for index, body in enumerate(events, start=1):
                record = {"seq": index, "ts": "2026-09-24T12:00:00.000Z", **body}
                handle.write(json.dumps(record) + "\n")

    def base_task_events(self) -> list[dict]:
        return [
            {"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"},
            {
                "event": "task",
                "id": "t1",
                "text": "do work",
                "class": "batch_scripting",
                "grade": "low",
                "deps": [],
                "paths": [],
            },
            {
                "event": "assignment",
                "task": "t1",
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "source": "ranking",
            },
        ]

    def test_fold_revision_and_status(self):
        events = self.base_task_events()
        self.write_events("g1", events)
        state = puppets.fold("g1", events)
        self.assertEqual(state.revision, 2)
        self.assertEqual(state.tasks["t1"].status, "assigned")
        self.assertEqual(state.tasks["t1"].assignment["provider"], "cursor-agent")

    def test_launch_refused_without_table_approval(self):
        events = self.base_task_events()
        self.write_events("g1", events)
        state = puppets.fold("g1", events)
        refusal = puppets.launch_refusal(state, "t1")
        self.assertIsNotNone(refusal)
        self.assertIn("table approval", refusal)

    def test_approval_voided_by_later_assignment(self):
        events = self.base_task_events() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "assignment",
                "task": "t1",
                "provider": "claude",
                "model": "sonnet",
                "policy": "standard",
                "source": "human",
            },
        ]
        self.write_events("g1", events)
        state = puppets.fold("g1", events)
        self.assertEqual(state.revision, 3)
        self.assertIsNone(puppets.approval_available(state, "table"))

    def test_codex_task_refused_without_launch_approval(self):
        events = self.base_task_events()
        events[2] = {
            "event": "assignment",
            "task": "t1",
            "provider": "codex",
            "model": "",
            "policy": "standard",
            "source": "ranking",
        }
        events.append(
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True}
        )
        self.write_events("g1", events)
        state = puppets.fold("g1", events)
        refusal = puppets.launch_refusal(state, "t1")
        self.assertIsNotNone(refusal)
        self.assertIn("launch approval", refusal)
        self.assertIn("codex", refusal)

    def test_launch_intent_before_run(self):
        events = self.base_task_events() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "deadbeef",
            },
            {"event": "run", "task": "t1", "attempt": 1, "run_id": "run-1"},
        ]
        self.write_events("g1", events)
        log = puppets.read_log("g1")
        intent_index = next(
            i for i, row in enumerate(log) if row["event"] == "launch_intent"
        )
        run_index = next(i for i, row in enumerate(log) if row["event"] == "run")
        self.assertLess(intent_index, run_index)

    def test_dangling_intent_reported_not_relaunched(self):
        events = self.base_task_events() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 7,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "deadbeef",
            },
        ]
        self.write_events("g1", events)
        state = puppets.fold("g1", events)
        self.assertEqual(len(state.dangling_intents), 1)
        self.assertEqual(state.dangling_intents[0]["attempt"], 7)
        refusal = puppets.launch_refusal(state, "t1")
        self.assertIn("ambiguous", refusal)
        self.assertIn("7", refusal)

    def test_plans_copies_from_worktree(self):
        run_id = "plan-run-1"
        proposal_dir = (
            self.hydra_root / "worktrees" / run_id / "docs" / "proposals"
        )
        proposal_dir.mkdir(parents=True)
        proposal = proposal_dir / "plan.md"
        proposal.write_text("proposal body\n", encoding="utf-8")
        events = [
            {"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"},
            {
                "event": "plan_proposed",
                "provider": "codex",
                "run_id": run_id,
            },
        ]
        self.write_events("g1", events)
        result = subprocess.run(
            [sys.executable, str(ROOT / "puppets.py"), "plans", "g1"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        dest = self.puppets_home / "g1" / "plans" / "codex.md"
        self.assertTrue(dest.is_file())
        self.assertEqual(dest.read_text(encoding="utf-8"), "proposal body\n")
        log = puppets.read_log("g1")
        self.assertTrue(any(row["event"] == "plan_ready" for row in log))

    def test_tasks_imports_json_block(self):
        plan_path = self.puppets_home / "g1" / "plans" / "chosen.md"
        plan_path.parent.mkdir(parents=True)
        plan_path.write_text(
            textwrap.dedent(
                """\
                Here is the plan.

                ```json
                [
                  {
                    "id": "t1",
                    "text": "write tests",
                    "class": "docs",
                    "grade": "low",
                    "deps": [],
                    "paths": ["tests/"]
                  }
                ]
                ```
                """
            ),
            encoding="utf-8",
        )
        events = [
            {"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"},
            {
                "event": "panel_verdict",
                "judge_run_ids": [],
                "chosen": "codex",
                "path": str(plan_path),
            },
        ]
        self.write_events("g1", events)
        result = subprocess.run(
            [sys.executable, str(ROOT / "puppets.py"), "tasks", "g1"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        state = puppets.fold("g1")
        self.assertIn("t1", state.tasks)
        self.assertEqual(state.tasks["t1"].text, "write tests")
        self.assertEqual(state.tasks["t1"].task_class, "docs")

    def test_launch_command_writes_intent_then_run(self):
        events = self.base_task_events() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
        ]
        self.write_events("g1", events)
        result = subprocess.run(
            [sys.executable, str(ROOT / "puppets.py"), "launch", "g1", "t1"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        log = puppets.read_log("g1")
        intent_index = next(
            i for i, row in enumerate(log) if row["event"] == "launch_intent"
        )
        run_index = next(i for i, row in enumerate(log) if row["event"] == "run")
        self.assertLess(intent_index, run_index)
        self.assertTrue(result.stdout.strip())

    def test_wait_appends_run_end(self):
        run_id = str(uuid.uuid4())
        run_file = self.hydra_root / "runs" / f"{run_id}.jsonl"
        run_file.write_text(
            '{"hydra":{"type":"run_start"}}\n{"hydra":{"type":"run_end"}}\n',
            encoding="utf-8",
        )
        events = self.base_task_events() + [
            {"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True},
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "x",
            },
            {"event": "run", "task": "t1", "attempt": 1, "run_id": run_id},
        ]
        self.write_events("g1", events)
        result = subprocess.run(
            [sys.executable, str(ROOT / "puppets.py"), "wait", "g1"],
            env=self.env,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        log = puppets.read_log("g1")
        ended = [row for row in log if row["event"] == "run_end"]
        self.assertEqual(len(ended), 1)
        self.assertEqual(ended[0]["outcome"], "completed")
        self.assertEqual(ended[0]["total_tokens"], 42)


@unittest.skipUnless(shutil.which("hydra"), "hydra binary not on PATH")
class SessionReviewTests(PuppetsTestCase):
    """Added by the session's review of build 108: decision C's batch table, the
    reconcile command, and the diff approval a verdict needs."""

    def two_task_events(self):
        events = self.base_task_events()
        events += [
            {"event": "task", "id": "t2", "text": "more", "class": "docs", "grade": "low",
             "deps": [], "paths": []},
            {"event": "assignment", "task": "t2", "provider": "cursor-agent",
             "model": "composer-2.5", "policy": "standard", "source": "ranking"},
        ]
        return events

    def test_one_table_approval_covers_every_launch_at_its_revision(self):
        events = self.two_task_events()
        events.append({"event": "approval", "kind": "table", "ref": None, "revision": 4, "yes": True})
        self.write_events("g1", events)
        self.assertEqual(puppets.main(["launch", "g1", "t1"]), 0)
        self.assertEqual(puppets.main(["launch", "g1", "t2"]), 0)
        state = puppets.fold("g1")
        self.assertEqual(state.tasks["t1"].status, "running")
        self.assertEqual(state.tasks["t2"].status, "running")

    def test_reconcile_clears_a_dangling_intent(self):
        events = self.base_task_events()
        events.append({"event": "approval", "kind": "table", "ref": None, "revision": 2, "yes": True})
        events.append({"event": "launch_intent", "task": "t1", "attempt": 1, "provider": "cursor-agent",
                       "model": "composer-2.5", "policy": "standard", "argv_sha256": "x"})
        self.write_events("g1", events)
        self.assertEqual(puppets.main(["launch", "g1", "t1"]), 1)
        self.assertEqual(puppets.main(["reconcile", "g1", "t1", "1", "--abandon"]), 0)
        self.assertEqual(puppets.fold("g1").dangling_intents, [])
        self.assertEqual(puppets.main(["launch", "g1", "t1"]), 0)

    def test_verdict_applied_needs_a_diff_approval(self):
        events = self.base_task_events()
        events += [
            {"event": "launch_intent", "task": "t1", "attempt": 1, "provider": "cursor-agent",
             "model": "composer-2.5", "policy": "standard", "argv_sha256": "x"},
            {"event": "run", "task": "t1", "attempt": 1, "run_id": "r1"},
            {"event": "run_end", "task": "t1", "run_id": "r1", "outcome": "completed", "total_tokens": 1},
        ]
        self.write_events("g1", events)
        self.assertEqual(puppets.main(["verdict", "g1", "t1", "applied"]), 1)
        self.assertEqual(puppets.main(["verdict", "g1", "t1", "rejected"]), 0)
        self.assertEqual(puppets.main(["approve", "g1", "--diff", "t1"]), 0)
        self.assertEqual(puppets.main(["verdict", "g1", "t1", "applied"]), 0)
        self.assertEqual(puppets.fold("g1").tasks["t1"].status, "applied")


class RankedAssignmentTests(PuppetsTestCase):
    def test_a_prior_only_ranking_yields_to_the_default_table(self):
        agents = self.puppets_home / "agents"
        agents.mkdir()
        (agents / "x--y.json").write_text("{}", encoding="utf-8")

        class Sheet:
            @staticmethod
            def rank(sheets, task_class, grade, verdicts, policy=None):
                return [{"provider": "x", "model": "y", "sample_size": 0}]

        original = puppets.load_sheet_module
        puppets.load_sheet_module = lambda: Sheet
        try:
            events = self.base_task_events()[:2]
            self.write_events("g1", events)
            self.assertEqual(puppets.main(["assign", "g1"]), 0)
            state = puppets.fold("g1")
            self.assertEqual(state.tasks["t1"].assignment["source"], "default")
            self.assertEqual(state.tasks["t1"].assignment["provider"], "cursor-agent")
        finally:
            puppets.load_sheet_module = original

    def test_a_ranking_with_verdicts_is_taken(self):
        agents = self.puppets_home / "agents"
        agents.mkdir()
        (agents / "x--y.json").write_text("{}", encoding="utf-8")

        class Sheet:
            @staticmethod
            def rank(sheets, task_class, grade, verdicts, policy=None):
                return [{"provider": "codex", "model": None, "sample_size": 3}]

        original = puppets.load_sheet_module
        puppets.load_sheet_module = lambda: Sheet
        try:
            self.write_events("g1", self.base_task_events()[:2])
            self.assertEqual(puppets.main(["assign", "g1"]), 0)
            assignment = puppets.fold("g1").tasks["t1"].assignment
            self.assertEqual((assignment["provider"], assignment["source"]), ("codex", "ranking"))
        finally:
            puppets.load_sheet_module = original


class HunkArgvTests(unittest.TestCase):
    def test_an_unreaped_run_opens_its_worktree(self):
        tmp = Path(tempfile.mkdtemp())
        (tmp / "r1").mkdir()
        argv, cwd = puppets.hunk_argv({"run_id": "r1", "branch": "hydra/r1"}, tmp)
        self.assertEqual((argv, cwd), (["hunk", "diff"], tmp / "r1"))
        shutil.rmtree(tmp)

    def test_a_reaped_run_opens_its_branch_from_the_repo(self):
        tmp = Path(tempfile.mkdtemp())
        argv, cwd = puppets.hunk_argv({"run_id": "r1", "branch": "hydra/r1", "repo": "/repo"}, tmp)
        self.assertEqual((argv, cwd), (["hunk", "show", "hydra/r1"], Path("/repo")))
        with self.assertRaises(RuntimeError):
            puppets.hunk_argv({"run_id": "r2"}, tmp)
        shutil.rmtree(tmp)


class PuppetsLiveTest(unittest.TestCase):
    def test_new_and_show_without_launch(self):
        env = os.environ.copy()
        tmp = tempfile.mkdtemp()
        env["HYDRA_PUPPETS_HOME"] = tmp
        try:
            new = subprocess.run(
                [sys.executable, str(ROOT / "puppets.py"), "new", "smoke goal"],
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(new.returncode, 0, new.stderr)
            goal_id = new.stdout.strip()
            self.assertTrue(goal_id)
            show = subprocess.run(
                [sys.executable, str(ROOT / "puppets.py"), "show", goal_id],
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(show.returncode, 0, show.stderr)
            data = json.loads(show.stdout)
            self.assertEqual(data["goal"]["text"], "smoke goal")
            self.assertEqual(data["revision"], 0)
        finally:
            shutil.rmtree(tmp)


if __name__ == "__main__":
    unittest.main()
