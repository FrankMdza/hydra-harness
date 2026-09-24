#!/usr/bin/env python3
"""Master of Puppets: goal log, fold, and launcher."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterator

TASK_CLASSES = frozenset(
    {
        "batch_scripting",
        "rust_off_launch_path",
        "rust_launch_path",
        "security_review",
        "research",
        "docs",
        "probe",
    }
)
TASK_GRADES = frozenset({"low", "medium", "high", "agi"})
APPROVAL_KINDS = frozenset({"plan", "table", "launch", "diff"})
VERDICT_RESULTS = frozenset({"applied", "fixed", "rejected"})

DEFAULT_ASSIGNMENTS: dict[str, tuple[str, str, str]] = {
    "batch_scripting": ("cursor-agent", "composer-2.5", "standard"),
    "docs": ("cursor-agent", "composer-2.5", "standard"),
    "rust_off_launch_path": ("cursor-agent", "composer-2.5", "standard"),
    "rust_launch_path": ("claude", "sonnet", "standard"),
    "security_review": ("codex", "", "standard"),
    "probe": ("agy", "gemini-3.1-pro-high", "standard"),
    "research": ("cursor-agent", "composer-2.5", "standard"),
}


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.%f")[:-3] + "Z"


def puppets_home() -> Path:
    named = os.environ.get("HYDRA_PUPPETS_HOME")
    if named:
        return Path(named)
    return Path.home() / "hydra" / "puppets"


def hydra_root() -> Path:
    named = os.environ.get("HYDRA_ROOT")
    if named:
        return Path(named)
    return Path.home() / "hydra"


def hydra_bin() -> str:
    return os.environ.get("HYDRA_BIN", "hydra")


def goal_log_path(goal_id: str) -> Path:
    return puppets_home() / f"{goal_id}.jsonl"


def goal_dir(goal_id: str) -> Path:
    return puppets_home() / goal_id


def read_log(goal_id: str) -> list[dict[str, Any]]:
    path = goal_log_path(goal_id)
    if not path.is_file():
        return []
    events: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                events.append(json.loads(line))
    return events


def append_event(goal_id: str, body: dict[str, Any]) -> dict[str, Any]:
    path = goal_log_path(goal_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    events = read_log(goal_id)
    record = {"seq": len(events) + 1, "ts": utc_now(), **body}
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, separators=(",", ":")) + "\n")
    return record


@dataclass
class TaskState:
    id: str
    text: str
    task_class: str
    grade: str
    deps: list[str] = field(default_factory=list)
    paths: list[str] = field(default_factory=list)
    assignment: dict[str, Any] | None = None
    status: str = "pending"
    run_id: str | None = None
    attempt: int | None = None
    outcome: str | None = None
    human_result: str | None = None


@dataclass
class GoalState:
    goal_id: str
    goal: dict[str, Any] | None = None
    revision: int = 0
    plans: list[dict[str, Any]] = field(default_factory=list)
    chosen_plan: dict[str, Any] | None = None
    human_plan: dict[str, Any] | None = None
    tasks: dict[str, TaskState] = field(default_factory=dict)
    approvals: list[dict[str, Any]] = field(default_factory=list)
    dangling_intents: list[dict[str, Any]] = field(default_factory=list)
    pending_runs: list[dict[str, Any]] = field(default_factory=list)


def fold(goal_id: str, events: list[dict[str, Any]] | None = None) -> GoalState:
    if events is None:
        events = read_log(goal_id)
    state = GoalState(goal_id=goal_id)
    revision = 0
    consumed_approvals: set[int] = set()
    intents: dict[tuple[str, int], dict[str, Any]] = {}
    runs_by_attempt: dict[tuple[str, int], str] = {}

    for index, event in enumerate(events, start=1):
        name = event["event"]
        seq = event.get("seq", index)
        if name == "goal":
            state.goal = {
                "text": event["text"],
                "repo": event["repo"],
                "base_sha": event["base_sha"],
            }
        elif name == "plan_proposed":
            state.plans.append(
                {
                    "provider": event["provider"],
                    "model": event.get("model"),
                    "run_id": event["run_id"],
                }
            )
        elif name == "plan_ready":
            for plan in state.plans:
                if plan["run_id"] == event["run_id"]:
                    plan["path"] = event["path"]
        elif name == "panel_verdict":
            state.chosen_plan = {
                "judge_run_ids": event.get("judge_run_ids", []),
                "chosen": event["chosen"],
                "path": event.get("path"),
            }
        elif name == "human_plan":
            state.human_plan = {"accepted": event["accepted"], "note": event.get("note")}
        elif name == "task":
            revision += 1
            task_id = event["id"]
            state.tasks[task_id] = TaskState(
                id=task_id,
                text=event["text"],
                task_class=event["class"],
                grade=event["grade"],
                deps=list(event.get("deps", [])),
                paths=list(event.get("paths", [])),
            )
        elif name == "assignment":
            revision += 1
            task_id = event["task"]
            task = state.tasks.get(task_id)
            if task is None:
                task = TaskState(
                    id=task_id,
                    text="",
                    task_class="batch_scripting",
                    grade="low",
                )
                state.tasks[task_id] = task
            task.assignment = {
                "provider": event["provider"],
                "model": event.get("model", ""),
                "policy": event["policy"],
                "source": event["source"],
            }
            if task.status == "pending":
                task.status = "assigned"
        elif name == "approval":
            state.approvals.append(
                {
                    "kind": event["kind"],
                    "ref": event.get("ref"),
                    "revision": event["revision"],
                    "yes": event["yes"],
                    "seq": seq,
                    "consumed": False,
                }
            )
        elif name == "launch_intent":
            key = (event["task"], event["attempt"])
            intents[key] = event
        elif name == "run":
            key = (event["task"], event["attempt"])
            runs_by_attempt[key] = event["run_id"]
            task = state.tasks.get(event["task"])
            if task is not None:
                task.status = "running"
                task.run_id = event["run_id"]
                task.attempt = event["attempt"]
            state.pending_runs.append(
                {
                    "task": event["task"],
                    "attempt": event["attempt"],
                    "run_id": event["run_id"],
                }
            )
            # A table approval covers every launch at its revision (decision C: once per
            # batch); only launch and diff approvals are spent by one use.
            if task is not None and needs_launch_approval(task):
                for approval in state.approvals:
                    if (
                        approval["kind"] == "launch"
                        and approval.get("ref") == event["task"]
                        and approval["revision"] == revision
                        and approval["yes"]
                        and approval["seq"] not in consumed_approvals
                    ):
                        consumed_approvals.add(approval["seq"])
                        break
        elif name == "run_end":
            task = state.tasks.get(event["task"])
            if task is not None and task.run_id == event["run_id"]:
                task.status = "ended"
                task.outcome = event.get("outcome")
            state.pending_runs = [
                pending
                for pending in state.pending_runs
                if pending["run_id"] != event["run_id"]
            ]
        elif name == "human_verdict":
            task = state.tasks.get(event["task"])
            if task is not None:
                task.human_result = event["result"]
                task.status = event["result"]
            for approval in state.approvals:
                if (
                    approval["kind"] == "diff"
                    and approval["ref"] == event["task"]
                    and approval["revision"] == revision
                    and approval["yes"]
                    and approval["seq"] not in consumed_approvals
                ):
                    consumed_approvals.add(approval["seq"])
        elif name == "intent_abandoned":
            intents.pop((event["task"], event["attempt"]), None)
        elif name == "review":
            pass

    state.revision = revision
    for key, intent in intents.items():
        if key not in runs_by_attempt:
            state.dangling_intents.append(intent)

    for approval in state.approvals:
        if approval["seq"] in consumed_approvals:
            approval["consumed"] = True
        elif approval["revision"] != revision:
            approval["voided"] = True

    return state


def approval_available(
    state: GoalState, kind: str, ref: str | None = None
) -> dict[str, Any] | None:
    for approval in state.approvals:
        if (
            approval.get("consumed")
            or approval.get("voided")
            or not approval["yes"]
            or approval["kind"] != kind
            or approval["revision"] != state.revision
        ):
            continue
        if kind in ("launch", "diff") and approval.get("ref") != ref:
            continue
        return approval
    return None


def needs_launch_approval(task: TaskState) -> bool:
    if task.assignment is None:
        return False
    provider = task.assignment["provider"]
    if provider == "codex":
        return True
    if provider == "claude" and task.grade in ("high", "agi"):
        return True
    return False


def launch_refusal(state: GoalState, task_id: str) -> str | None:
    if state.dangling_intents:
        attempt = state.dangling_intents[0]["attempt"]
        return (
            f"ambiguous launch intent attempt {attempt}; reconcile with hydra status "
            "before launching again"
        )
    task = state.tasks.get(task_id)
    if task is None:
        return f"unknown task {task_id}"
    if task.assignment is None:
        return f"task {task_id} has no assignment"
    if approval_available(state, "table") is None:
        return (
            f"missing table approval at revision {state.revision}; "
            f"approve with: approve {state.goal_id} --table"
        )
    if needs_launch_approval(task) and approval_available(state, "launch", task_id) is None:
        provider = task.assignment["provider"]
        if provider == "codex":
            reason = "codex gate can fail open on a dead hook"
        else:
            reason = "claude gate can fail open when it fails to start"
        return (
            f"missing launch approval for task {task_id} at revision {state.revision} "
            f"({reason}); approve with: approve {state.goal_id} --launch {task_id}"
        )
    return None


def next_attempt(goal_id: str, task_id: str) -> int:
    attempts = [
        event["attempt"]
        for event in read_log(goal_id)
        if event.get("event") == "launch_intent" and event.get("task") == task_id
    ]
    return max(attempts, default=0) + 1


def build_run_argv(
    provider: str, model: str | None, policy: str, task_text: str
) -> list[str]:
    argv = [hydra_bin(), "run", "--provider", provider]
    if model:
        argv.extend(["--model", model])
    argv.extend(["--policy", policy, "--timeout", "0", "--task", task_text])
    return argv


def argv_sha256(argv: list[str]) -> str:
    joined = "\0".join(argv)
    return hashlib.sha256(joined.encode("utf-8")).hexdigest()


def spawn_run(argv: list[str]) -> str:
    """Runs `hydra run --timeout 0` and returns the id it prints on stderr. The launch itself
    never goes through a pane: `hydra-in-pane` execs `hydra logs --follow` and would not return
    until the run ended, so inside Herdr the pane is opened afterwards by `attach_pane`."""
    proc = subprocess.run(argv, capture_output=True, text=True, check=False)
    stderr = proc.stderr or ""
    match = re.search(r"^hydra: run (\S+)", stderr, re.MULTILINE)
    if not match:
        raise RuntimeError(f"hydra run printed no run id; stderr was:\n{stderr.strip()}")
    return match.group(1)


def attach_pane(run_id: str, provider: str, policy: str) -> str | None:
    """Inside Herdr, splits a pane below the current one that tails the run and carries the
    tokens the bridge keys on, the way `hydra-in-pane` does. Returns the pane id, or None
    outside Herdr or when Herdr refuses."""
    pane_id = os.environ.get("HERDR_PANE_ID")
    if not os.environ.get("HERDR_ENV") or not pane_id:
        return None
    try:
        split = subprocess.run(
            ["herdr", "pane", "split", pane_id, "--direction", "down", "--cwd", os.getcwd(),
             "--env", f"HYDRA_BIN={hydra_bin()}", "--no-focus"],
            capture_output=True, text=True, check=True,
        )
        new_pane = json.loads(split.stdout)["result"]["pane"]["pane_id"]
        subprocess.run(
            ["herdr", "pane", "report-metadata", new_pane, "--source", "hydra-bridge",
             "--token", f"run_id={run_id}", "--token", f"provider={provider}",
             "--token", f"policy={policy}"],
            capture_output=True, text=True, check=True,
        )
        subprocess.run(
            ["herdr", "pane", "run", new_pane, f"{hydra_bin()} logs {run_id} --follow"],
            capture_output=True, text=True, check=True,
        )
        return new_pane
    except (OSError, subprocess.CalledProcessError, ValueError, KeyError):
        return None


def git_repo_and_sha(cwd: Path | None = None) -> tuple[str, str]:
    cwd = cwd or Path.cwd()
    try:
        repo = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            cwd=cwd,
            text=True,
        ).strip()
        sha = subprocess.check_output(
            ["git", "rev-parse", "HEAD"],
            cwd=cwd,
            text=True,
        ).strip()
        return repo, sha
    except (subprocess.CalledProcessError, FileNotFoundError):
        return str(cwd), ""


def parse_seat(seat: str) -> tuple[str, str | None]:
    if ":" in seat:
        provider, model = seat.split(":", 1)
        return provider, model
    return seat, None


def load_plan_prompt(goal_text: str) -> str:
    prompt_path = Path(__file__).resolve().parent / "prompts" / "plan.md"
    template = prompt_path.read_text(encoding="utf-8")
    return template.replace("{{GOAL}}", goal_text)


def extract_tasks_json(text: str) -> list[dict[str, Any]]:
    match = re.search(r"```(?:json)?\s*(\[[\s\S]*?\])\s*```", text)
    if not match:
        raise ValueError("no fenced JSON task array found in plan")
    tasks = json.loads(match.group(1))
    if not isinstance(tasks, list):
        raise ValueError("plan JSON block is not an array")
    return tasks


def load_sheet_module():
    sheet_path = Path(__file__).resolve().parent / "sheet.py"
    if not sheet_path.is_file():
        return None
    import importlib.util

    spec = importlib.util.spec_from_file_location("puppets_sheet", sheet_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def ranked_assignment(
    task: TaskState, events: list[dict[str, Any]]
) -> tuple[str, str, str] | None:
    """The cheapest capable agent for a task from `sheet.rank`, over the sheets built under
    the puppets home, with this goal's own verdicts. None when no sheet exists or nobody
    qualifies, in which case the caller falls back to the default table."""
    sheet = load_sheet_module()
    agents_dir = puppets_home() / "agents"
    if sheet is None or not agents_dir.is_dir():
        return None
    sheets = [json.loads(p.read_text(encoding="utf-8")) for p in sorted(agents_dir.glob("*.json"))]
    if not sheets:
        return None
    rows = sheet.rank(sheets, task.task_class, task.grade, events, policy="standard")
    if not rows:
        return None
    best = rows[0]
    # A ranking with no verdict behind it is the priors talking; the default table is the
    # human's own memory of each agent and wins until this goal has taught the sheet something.
    if not best.get("sample_size"):
        return None
    return best["provider"], best.get("model") or "", "standard"


def cmd_new(args: argparse.Namespace) -> int:
    goal_id = str(uuid.uuid4())
    repo, base_sha = git_repo_and_sha()
    append_event(
        goal_id,
        {
            "event": "goal",
            "text": args.goal_text,
            "repo": repo,
            "base_sha": base_sha,
        },
    )
    print(goal_id)
    return 0


def cmd_show(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    print(json.dumps(state_to_dict(state), indent=2))
    return 0


def state_to_dict(state: GoalState) -> dict[str, Any]:
    return {
        "goal_id": state.goal_id,
        "goal": state.goal,
        "revision": state.revision,
        "plans": state.plans,
        "chosen_plan": state.chosen_plan,
        "human_plan": state.human_plan,
        "tasks": {
            task_id: {
                "id": task.id,
                "text": task.text,
                "class": task.task_class,
                "grade": task.grade,
                "deps": task.deps,
                "paths": task.paths,
                "assignment": task.assignment,
                "status": task.status,
                "run_id": task.run_id,
                "attempt": task.attempt,
                "outcome": task.outcome,
                "human_result": task.human_result,
            }
            for task_id, task in state.tasks.items()
        },
        "approvals": state.approvals,
        "dangling_intents": state.dangling_intents,
        "pending_runs": state.pending_runs,
    }


def cmd_plan(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    if state.goal is None:
        print(f"unknown goal {args.goal}", file=sys.stderr)
        return 1
    brief = load_plan_prompt(state.goal["text"])
    for seat in args.seat:
        provider, model = parse_seat(seat)
        argv = build_run_argv(provider, model, "standard", brief)
        run_id = spawn_run(argv)
        body: dict[str, Any] = {
            "event": "plan_proposed",
            "provider": provider,
            "run_id": run_id,
        }
        if model:
            body["model"] = model
        append_event(args.goal, body)
        print(f"proposed {provider} run {run_id}")
    return 0


def hydra_show_text(run_id: str) -> str:
    proc = subprocess.run(
        [hydra_bin(), "show", run_id], capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        return ""
    try:
        return json.loads(proc.stdout).get("result_text") or ""
    except json.JSONDecodeError:
        return ""


def cmd_plans(args: argparse.Namespace) -> int:
    """A proposer answers with its plan as text, so the plan is the run's `result_text`;
    a file under the worktree's `docs/proposals/` is taken instead when the run wrote one."""
    state = fold(args.goal)
    dest = goal_dir(args.goal) / "plans"
    dest.mkdir(parents=True, exist_ok=True)
    copied = 0
    for plan in state.plans:
        if plan.get("path"):
            continue
        run_id = plan["run_id"]
        src_dir = hydra_root() / "worktrees" / run_id / "docs" / "proposals"
        sources = sorted(p for p in src_dir.iterdir() if p.is_file()) if src_dir.is_dir() else []
        target = dest / f"{plan['provider']}.md"
        if sources:
            shutil.copy2(sources[0], target)
        else:
            if not run_file_has_end(run_id):
                print(f"{plan['provider']} run {run_id} has not ended", file=sys.stderr)
                continue
            text = hydra_show_text(run_id)
            if not text.strip():
                print(f"{plan['provider']} run {run_id} left no plan text", file=sys.stderr)
                continue
            target.write_text(text, encoding="utf-8")
        append_event(
            args.goal,
            {"event": "plan_ready", "provider": plan["provider"], "run_id": run_id, "path": str(target)},
        )
        copied += 1
        print(f"plan {plan['provider']} -> {target}")
    if copied == 0:
        print("no finished proposals to copy", file=sys.stderr)
        return 1
    return 0


def cmd_choose(args: argparse.Namespace) -> int:
    chosen = args.choice
    path = None
    if os.path.isfile(chosen):
        path = os.path.abspath(chosen)
        chosen = Path(path).stem
    append_event(
        args.goal,
        {
            "event": "panel_verdict",
            "judge_run_ids": [],
            "chosen": chosen,
            "path": path,
        },
    )
    return 0


def cmd_tasks(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    if state.chosen_plan is None or not state.chosen_plan.get("path"):
        print("no chosen plan with a path", file=sys.stderr)
        return 1
    plan_text = Path(state.chosen_plan["path"]).read_text(encoding="utf-8")
    tasks = extract_tasks_json(plan_text)
    for task in tasks:
        missing = [k for k in ("id", "text", "class", "grade") if k not in task]
        if missing:
            print(f"task {task.get('id', '?')} lacks {', '.join(missing)}", file=sys.stderr)
            return 1
        if task["class"] not in TASK_CLASSES or task["grade"] not in TASK_GRADES:
            print(f"task {task['id']} has an unknown class or grade", file=sys.stderr)
            return 1
    for task in tasks:
        append_event(
            args.goal,
            {
                "event": "task",
                "id": task["id"],
                "text": task["text"],
                "class": task["class"],
                "grade": task["grade"],
                "deps": task.get("deps", []),
                "paths": task.get("paths", []),
            },
        )
    return 0


def cmd_assign(args: argparse.Namespace) -> int:
    events = read_log(args.goal)
    state = fold(args.goal, events)
    for task_id, task in state.tasks.items():
        if task.assignment is not None:
            continue
        if task.grade == "agi":
            print(f"task {task_id} needs a human assignment (agi)", file=sys.stderr)
            continue
        row = ranked_assignment(task, events)
        source = "ranking"
        if row is None:
            row = DEFAULT_ASSIGNMENTS.get(task.task_class)
            source = "default"
        if row is None:
            print(f"task {task_id} needs a human assignment", file=sys.stderr)
            continue
        provider, model, policy = row
        if not provider:
            print(f"task {task_id} needs a human assignment", file=sys.stderr)
            continue
        body: dict[str, Any] = {
            "event": "assignment",
            "task": task_id,
            "provider": provider,
            "policy": policy,
            "source": source,
        }
        if model:
            body["model"] = model
        append_event(args.goal, body)
        print(f"{task_id}: {provider} {model or ''} ({source})")
    return 0


def cmd_set(args: argparse.Namespace) -> int:
    body: dict[str, Any] = {
        "event": "assignment",
        "task": args.task,
        "provider": args.provider,
        "policy": args.policy,
        "source": "human",
    }
    if args.model:
        body["model"] = args.model
    append_event(args.goal, body)
    return 0


def cmd_approve(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    if args.plan:
        kind, ref = "plan", None
    elif args.table:
        kind, ref = "table", None
    elif args.launch:
        kind, ref = "launch", args.launch
    elif args.diff:
        kind, ref = "diff", args.diff
    else:
        print("specify --plan, --table, --launch or --diff", file=sys.stderr)
        return 1
    if ref is not None and ref not in state.tasks:
        print(f"unknown task {ref}", file=sys.stderr)
        return 1
    append_event(
        args.goal,
        {
            "event": "approval",
            "kind": kind,
            "ref": ref,
            "revision": state.revision,
            "yes": True,
        },
    )
    return 0


def cmd_launch(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    refusal = launch_refusal(state, args.task)
    if refusal:
        print(refusal, file=sys.stderr)
        return 1
    task = state.tasks[args.task]
    assignment = task.assignment
    assert assignment is not None
    provider = assignment["provider"]
    model = assignment.get("model") or None
    policy = assignment["policy"]
    attempt = next_attempt(args.goal, args.task)
    argv = build_run_argv(provider, model, policy, task.text)
    sha = argv_sha256(argv)
    append_event(
        args.goal,
        {
            "event": "launch_intent",
            "task": args.task,
            "attempt": attempt,
            "provider": provider,
            "model": model or "",
            "policy": policy,
            "argv_sha256": sha,
        },
    )
    run_id = spawn_run(argv)
    append_event(
        args.goal,
        {
            "event": "run",
            "task": args.task,
            "attempt": attempt,
            "run_id": run_id,
        },
    )
    pane = attach_pane(run_id, provider, policy)
    print(run_id if pane is None else f"{run_id} {pane}")
    return 0


def cmd_reconcile(args: argparse.Namespace) -> int:
    """Settles a launch intent with no run after it: either the run id the human found with
    `hydra status`, or abandonment. Nothing is relaunched here."""
    state = fold(args.goal)
    match = [i for i in state.dangling_intents if i["task"] == args.task and i["attempt"] == args.attempt]
    if not match:
        print(f"no dangling intent for task {args.task} attempt {args.attempt}", file=sys.stderr)
        return 1
    if args.run_id:
        append_event(
            args.goal,
            {"event": "run", "task": args.task, "attempt": args.attempt, "run_id": args.run_id},
        )
    else:
        append_event(args.goal, {"event": "intent_abandoned", "task": args.task, "attempt": args.attempt})
    return 0


def run_file_has_end(run_id: str) -> bool:
    path = hydra_root() / "runs" / f"{run_id}.jsonl"
    if not path.is_file():
        return False
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            body = record.get("hydra") or record
            if body.get("type") == "run_end":
                return True
    return False


def hydra_show_outcome(run_id: str) -> tuple[str, int]:
    proc = subprocess.run(
        [hydra_bin(), "show", run_id],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or proc.stdout.strip())
    data = json.loads(proc.stdout)
    outcome = data.get("outcome") or "unknown"
    usage = data.get("usage") or {}
    counts = usage.get("counts") or {}
    total_tokens = int(counts.get("total_tokens") or 0)
    return outcome, total_tokens


def cmd_wait(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    if not state.pending_runs:
        print("no pending runs")
        return 0
    for pending in list(state.pending_runs):
        run_id = pending["run_id"]
        if not run_file_has_end(run_id):
            continue
        outcome, total_tokens = hydra_show_outcome(run_id)
        append_event(
            args.goal,
            {
                "event": "run_end",
                "task": pending["task"],
                "run_id": run_id,
                "outcome": outcome,
                "total_tokens": total_tokens,
            },
        )
        print(f"ended {run_id} {outcome}")
    return 0


def cmd_verdict(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    task = state.tasks.get(args.task)
    if task is None or not task.run_id:
        print(f"task {args.task} has no run", file=sys.stderr)
        return 1
    if args.result in ("applied", "fixed") and approval_available(state, "diff", args.task) is None:
        print(
            f"missing diff approval for task {args.task} at revision {state.revision}; "
            f"approve with: approve {state.goal_id} --diff {args.task}",
            file=sys.stderr,
        )
        return 1
    append_event(
        args.goal,
        {
            "event": "human_verdict",
            "task": args.task,
            "run_id": task.run_id,
            "result": args.result,
            "note": args.note or "",
        },
    )
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Master of Puppets goal log")
    sub = parser.add_subparsers(dest="command", required=True)

    new_p = sub.add_parser("new", help="start a goal")
    new_p.add_argument("goal_text", help="goal description")
    new_p.set_defaults(func=cmd_new)

    show_p = sub.add_parser("show", help="fold and print goal state")
    show_p.add_argument("goal", help="goal id")
    show_p.set_defaults(func=cmd_show)

    plan_p = sub.add_parser("plan", help="launch plan proposers")
    plan_p.add_argument("goal", help="goal id")
    plan_p.add_argument(
        "--seat",
        action="append",
        required=True,
        help="provider or provider:model",
    )
    plan_p.set_defaults(func=cmd_plan)

    plans_p = sub.add_parser("plans", help="copy finished proposals from worktrees")
    plans_p.add_argument("goal", help="goal id")
    plans_p.set_defaults(func=cmd_plans)

    choose_p = sub.add_parser("choose", help="record panel verdict")
    choose_p.add_argument("goal", help="goal id")
    choose_p.add_argument("choice", help="provider name or plan file path")
    choose_p.set_defaults(func=cmd_choose)

    tasks_p = sub.add_parser("tasks", help="import tasks from chosen plan")
    tasks_p.add_argument("goal", help="goal id")
    tasks_p.set_defaults(func=cmd_tasks)

    assign_p = sub.add_parser("assign", help="assign providers from sheet")
    assign_p.add_argument("goal", help="goal id")
    assign_p.set_defaults(func=cmd_assign)

    set_p = sub.add_parser("set", help="human assignment override")
    set_p.add_argument("goal", help="goal id")
    set_p.add_argument("task", help="task id")
    set_p.add_argument("--provider", required=True)
    set_p.add_argument("--model", default="")
    set_p.add_argument("--policy", required=True)
    set_p.set_defaults(func=cmd_set)

    approve_p = sub.add_parser("approve", help="record human approval")
    approve_p.add_argument("goal", help="goal id")
    approve_p.add_argument("--plan", action="store_true")
    approve_p.add_argument("--table", action="store_true")
    approve_p.add_argument("--launch", metavar="TASK")
    approve_p.add_argument("--diff", metavar="TASK")
    approve_p.set_defaults(func=cmd_approve)

    launch_p = sub.add_parser("launch", help="launch a task run")
    launch_p.add_argument("goal", help="goal id")
    launch_p.add_argument("task", help="task id")
    launch_p.set_defaults(func=cmd_launch)

    rec_p = sub.add_parser("reconcile", help="settle a launch intent that has no run")
    rec_p.add_argument("goal", help="goal id")
    rec_p.add_argument("task", help="task id")
    rec_p.add_argument("attempt", type=int, help="attempt number from the intent")
    group = rec_p.add_mutually_exclusive_group(required=True)
    group.add_argument("--run-id", help="the run hydra status shows for this attempt")
    group.add_argument("--abandon", action="store_true", help="no run was ever started")
    rec_p.set_defaults(func=cmd_reconcile)

    wait_p = sub.add_parser("wait", help="poll pending runs for completion")
    wait_p.add_argument("goal", help="goal id")
    wait_p.set_defaults(func=cmd_wait)

    verdict_p = sub.add_parser("verdict", help="record human verdict on a diff")
    verdict_p.add_argument("goal", help="goal id")
    verdict_p.add_argument("task", help="task id")
    verdict_p.add_argument("result", choices=sorted(VERDICT_RESULTS))
    verdict_p.add_argument("--note", default="")
    verdict_p.set_defaults(func=cmd_verdict)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
