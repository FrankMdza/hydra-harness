import io
import json
import os
import shutil
import sys
import tempfile
import textwrap
import unittest
from contextlib import redirect_stderr
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import puppets


class PanelDegradedTests(unittest.TestCase):
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
        # cmd_review chdirs into the checkout before exec; with exec stubbed the chdir stays,
        # and the next test would start in a directory tearDown deleted.
        self._cwd = os.getcwd()
        self._prior = {
            key: os.environ.get(key)
            for key in ("HYDRA_PUPPETS_HOME", "HYDRA_ROOT", "HYDRA_BIN", "HYDRA_PUPPETS_VENDORS")
        }
        os.environ["HYDRA_PUPPETS_HOME"] = str(self.puppets_home)
        os.environ["HYDRA_ROOT"] = str(self.hydra_root)
        os.environ["HYDRA_BIN"] = str(self.stub)
        os.environ["HYDRA_PUPPETS_VENDORS"] = str(self.vendors_path)

    def tearDown(self):
        os.chdir(self._cwd)
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
                record = {"seq": index, "ts": "2026-09-24T12:00:00.000Z", **body}
                handle.write(json.dumps(record) + "\n")

    def mark_codex_out(self) -> None:
        rows = json.loads(self.vendors_path.read_text(encoding="utf-8"))
        for row in rows:
            if row.get("provider") == "codex":
                row["available"] = False
        self.vendors_path.write_text(json.dumps(rows), encoding="utf-8")

    def test_full_panel_builds_three_roles_and_slices(self):
        resolved = [
            puppets.ResolvedSeat(
                seat=seat, provider=seat.provider, model=seat.model, policy=seat.policy
            )
            for seat in puppets.PUPPETS_PANEL_SEATS
        ]
        prompts = puppets.build_diff_panel_prompts(
            resolved, "task body", "base123", "projection text", "diff --git a/foo b/foo\n"
        )
        self.assertEqual(len(prompts), 3)
        bodies = [text for _seat, text in prompts]
        self.assertIn("reviewer", bodies[0])
        self.assertIn("task body", bodies[0])
        self.assertNotIn("projection text", bodies[0])
        self.assertIn("adversary", bodies[1])
        self.assertNotIn("task body", bodies[1])
        self.assertIn("security", bodies[2])
        self.assertIn("projection text", bodies[2])

    def test_codex_out_refuses_without_degraded_approval(self):
        self.mark_codex_out()
        plan_dir = self.puppets_home / "g1" / "plans"
        plan_dir.mkdir(parents=True)
        codex_plan = plan_dir / "codex.md"
        cursor_plan = plan_dir / "cursor-agent.md"
        codex_plan.write_text("codex plan\n", encoding="utf-8")
        cursor_plan.write_text("cursor plan\n", encoding="utf-8")
        self.write_events(
            "g1",
            [
                {"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"},
                {"event": "plan_proposed", "provider": "codex", "run_id": "r1"},
                {
                    "event": "plan_ready",
                    "provider": "codex",
                    "run_id": "r1",
                    "path": str(codex_plan),
                },
                {"event": "plan_proposed", "provider": "cursor-agent", "run_id": "r2"},
                {
                    "event": "plan_ready",
                    "provider": "cursor-agent",
                    "run_id": "r2",
                    "path": str(cursor_plan),
                },
            ],
        )
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            code = puppets.main(["choose", "g1", "--judge"])
        self.assertEqual(code, 1)
        self.assertIn("--degraded", stderr.getvalue())
        self.assertIn("codex", stderr.getvalue())

    def test_degraded_substitute_runs_in_codex_role_and_logs_degraded(self):
        self.mark_codex_out()
        events = [
            {"event": "goal", "text": "test", "repo": str(self.tmp), "base_sha": "abc"},
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
            {
                "event": "approval",
                "kind": "degraded",
                "missing": "codex",
                "substitute": "cursor-agent",
                "model": "composer-2.5",
                "revision": 2,
                "yes": True,
            },
            {
                "event": "launch_intent",
                "task": "t1",
                "attempt": 1,
                "provider": "cursor-agent",
                "model": "composer-2.5",
                "policy": "standard",
                "argv_sha256": "x",
            },
            {"event": "run", "task": "t1", "attempt": 1, "run_id": "r1"},
            {
                "event": "run_end",
                "task": "t1",
                "run_id": "r1",
                "outcome": "completed",
                "total_tokens": 1,
            },
        ]
        self.write_events("g1", events)
        captured: list[tuple[puppets.ResolvedSeat, str]] = []

        def fake_panel(seat_prompts):
            captured.extend(seat_prompts)
            return (
                ["a", "b", "c"],
                [resolved.policy for resolved, _text in seat_prompts],
                [
                    '{"verdict":"pass","findings":[]}',
                    '{"verdict":"pass","findings":[]}',
                    '{"verdict":"pass","findings":[]}',
                ],
            )

        original_panel = puppets.run_judge_panel
        original_show = puppets.hydra_show_run
        original_diff = puppets.git_diff_branch
        original_exec = puppets.os.execvp
        original_fork = puppets.fork_hunk_comment_pusher
        puppets.run_judge_panel = fake_panel
        puppets.hydra_show_run = lambda run_id: {
            "run_id": run_id,
            "branch": f"hydra/{run_id}",
            "repo": str(self.tmp),
            "result_text": "",
        }
        puppets.git_diff_branch = lambda *_args, **_kwargs: "diff"
        puppets.fork_hunk_comment_pusher = lambda *_args, **_kwargs: None
        puppets.os.execvp = lambda *_args, **_kwargs: (_ for _ in ()).throw(SystemExit(0))
        try:
            with self.assertRaises(SystemExit):
                puppets.main(["review", "g1", "t1"])
        finally:
            puppets.run_judge_panel = original_panel
            puppets.hydra_show_run = original_show
            puppets.git_diff_branch = original_diff
            puppets.os.execvp = original_exec
            puppets.fork_hunk_comment_pusher = original_fork

        adversary = next(pair for pair in captured if pair[0].seat.role == "adversary")
        self.assertEqual(adversary[0].provider, "cursor-agent")
        self.assertEqual(adversary[0].missing_provider, "codex")
        # The stand-in keeps its own seat's `judge`, not codex's `standard`.
        self.assertEqual(adversary[0].policy, "judge")
        self.assertIn("adversary", adversary[1])
        self.assertNotIn("do work", adversary[1])

        review = next(row for row in puppets.read_log("g1") if row["event"] == "review")
        self.assertTrue(review["degraded"])
        self.assertEqual(review["substitutions"][0]["missing"], "codex")

    def test_stale_degraded_approval_does_not_count(self):
        self.mark_codex_out()
        plan_dir = self.puppets_home / "g1" / "plans"
        plan_dir.mkdir(parents=True)
        codex_plan = plan_dir / "codex.md"
        cursor_plan = plan_dir / "cursor-agent.md"
        codex_plan.write_text("a\n", encoding="utf-8")
        cursor_plan.write_text("b\n", encoding="utf-8")
        self.write_events(
            "g1",
            [
                {"event": "goal", "text": "test", "repo": "/repo", "base_sha": "abc"},
                {
                    "event": "task",
                    "id": "t1",
                    "text": "x",
                    "class": "docs",
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
                    "source": "human",
                },
                {"event": "plan_proposed", "provider": "codex", "run_id": "r1"},
                {
                    "event": "plan_ready",
                    "provider": "codex",
                    "run_id": "r1",
                    "path": str(codex_plan),
                },
                {"event": "plan_proposed", "provider": "cursor-agent", "run_id": "r2"},
                {
                    "event": "plan_ready",
                    "provider": "cursor-agent",
                    "run_id": "r2",
                    "path": str(cursor_plan),
                },
                {
                    "event": "approval",
                    "kind": "degraded",
                    "missing": "codex",
                    "substitute": "cursor-agent",
                    "revision": 0,
                    "yes": True,
                },
            ],
        )
        self.assertEqual(puppets.fold("g1").revision, 2)
        stderr = io.StringIO()
        with redirect_stderr(stderr):
            code = puppets.main(["choose", "g1", "--judge"])
        self.assertEqual(code, 1)
        self.assertIn("codex is unavailable", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
