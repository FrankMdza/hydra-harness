"""Map Hydra run ids to Puppets task labels (stdlib only)."""
from __future__ import annotations

import importlib.util
import json
import os
import sys
from pathlib import Path
from typing import Any

PUPPETS_MODULE = Path(__file__).resolve().parents[1] / "puppets" / "puppets.py"
_spec = importlib.util.spec_from_file_location("puppets", PUPPETS_MODULE)
puppets = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = puppets
_spec.loader.exec_module(puppets)


def puppets_home() -> Path:
    named = os.environ.get("HYDRA_PUPPETS_HOME")
    if named:
        return Path(named)
    root = os.environ.get("HYDRA_ROOT")
    if root:
        return Path(root) / "puppets"
    return Path.home() / "hydra" / "puppets"


def committed_events(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        return []
    events: list[dict[str, Any]] = []
    raw = path.read_bytes()
    lines = raw.splitlines(keepends=True)
    for index, line in enumerate(lines):
        if not line.endswith(b"\n"):
            break
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            if index == len(lines) - 1:
                break
            raise
    return events


def labels_for_goal(goal_id: str, events: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    state = puppets.fold(goal_id, events)
    goal_text = (state.goal or {}).get("text", "")
    labels: dict[str, dict[str, Any]] = {}

    for task in state.tasks.values():
        if not task.run_id:
            continue
        labels[task.run_id] = {
            "task_id": task.id,
            "class": task.task_class,
            "grade": task.grade,
            "human_verdict": task.human_result,
            "text": task.text,
        }

    for plan in state.plans:
        labels[plan["run_id"]] = {
            "task_id": None,
            "class": "plan_proposer",
            "grade": None,
            "human_verdict": None,
            "text": goal_text,
        }

    for event in events:
        name = event.get("event")
        if name == "panel_verdict":
            for run_id in event.get("judge_run_ids", []):
                labels[run_id] = {
                    "task_id": None,
                    "class": "plan_judge",
                    "grade": None,
                    "human_verdict": None,
                    "text": goal_text,
                }
        elif name == "review":
            task = state.tasks.get(event.get("task"))
            for run_id in event.get("judge_run_ids", []):
                labels[run_id] = {
                    "task_id": event.get("task"),
                    "class": task.task_class if task else None,
                    "grade": task.grade if task else None,
                    "human_verdict": None,
                    "text": task.text if task else "",
                }

    return labels


def collect(home: Path | None = None) -> dict[str, dict[str, Any]]:
    """Fold every goal log under ``home`` and return run_id -> label fields."""
    home = home or puppets_home()
    labels: dict[str, dict[str, Any]] = {}
    for path in sorted(home.glob("*.jsonl")):
        goal_id = path.stem
        for run_id, row in labels_for_goal(goal_id, committed_events(path)).items():
            labels[run_id] = row
    return labels
