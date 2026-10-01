"""Map Hydra run ids to Puppets task labels (stdlib only)."""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

CHECKOUT = Path(__file__).resolve().parents[2]


class LabelsError(RuntimeError):
    """`hydra-puppets show` could not be run, failed, or printed something this module does
    not know how to read. The grader pass that meets one writes nothing."""


def puppets_home() -> Path:
    named = os.environ.get("HYDRA_PUPPETS_HOME")
    if named:
        return Path(named)
    root = os.environ.get("HYDRA_ROOT")
    if root:
        return Path(root) / "puppets"
    return Path.home() / "hydra" / "puppets"


def grader_home() -> Path:
    named = os.environ.get("HYDRA_GRADER_HOME")
    if named:
        return Path(named)
    root = os.environ.get("HYDRA_ROOT")
    if root:
        return Path(root) / "grader"
    return Path.home() / "hydra" / "grader"


def helper_bin() -> Path:
    """The `hydra-puppets` that folds a goal log (map 93). Always an absolute path and never
    a search of `PATH`: `HYDRA_PUPPETS_BIN`, else the one beside `HYDRA_BIN` when that is
    absolute, else the release build of the checkout this file is in. The first that is
    configured is the answer, so a missing binary is an error and not a reason to try the
    next."""
    named = os.environ.get("HYDRA_PUPPETS_BIN")
    if named:
        path = Path(named)
        if not path.is_absolute():
            raise LabelsError(f"HYDRA_PUPPETS_BIN is not an absolute path: {named}")
        return path
    hydra = os.environ.get("HYDRA_BIN")
    if hydra and Path(hydra).is_absolute():
        return Path(hydra).parent / "hydra-puppets"
    return CHECKOUT / "target" / "release" / "hydra-puppets"


class _Object(dict):
    """A JSON object that also keeps every pair it was written with. `show` prints its tasks
    under `str(id)`, so the ids `3` and `"3"` are one key twice and a plain dict keeps one."""

    pairs: list[tuple[str, Any]]


def _object(pairs: list[tuple[str, Any]]) -> _Object:
    out = _Object(pairs)
    out.pairs = pairs
    return out


TASK_FIELDS = ("id", "class", "grade", "text", "run_id", "human_result")


def show_goal(goal_id: str, home: Path) -> dict[str, Any]:
    """What `hydra-puppets show <goal>` prints for the log under `home`, with the shape
    `labels_for_goal` reads checked. Raises `LabelsError` when the binary is missing, exits
    non-zero (a damaged log does), or prints anything else."""
    binary = helper_bin()
    env = dict(os.environ, HYDRA_PUPPETS_HOME=str(home))
    try:
        done = subprocess.run(
            [str(binary), "show", "--", goal_id],
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
        )
    except OSError as err:
        raise LabelsError(f"cannot run {binary}: {err}") from err
    if done.returncode != 0:
        said = done.stderr.decode("utf-8", "replace").strip()
        raise LabelsError(f"{binary} show {goal_id} exited {done.returncode}: {said}")
    try:
        state = json.loads(done.stdout, object_pairs_hook=_object)
    except ValueError as err:
        raise LabelsError(f"{binary} show {goal_id} did not print JSON: {err}") from err

    def refuse(what: str) -> LabelsError:
        return LabelsError(f"{binary} show {goal_id}: {what}")

    if not isinstance(state, dict):
        raise refuse("the state is not an object")
    for key in ("goal", "tasks", "plans"):
        if key not in state:
            raise refuse(f"no `{key}`")
    # `show` prints the empty state, exit 0, for a goal with no log. An answer about any
    # goal but the one asked for is not read as this one's.
    if state.get("goal_id") != goal_id:
        raise refuse(f"`goal_id` is {state.get('goal_id')!r}, not the goal asked for")
    goal = state["goal"]
    if goal is not None and not (isinstance(goal, dict) and "text" in goal):
        raise refuse("`goal` is neither null nor an object with `text`")
    if not isinstance(state["tasks"], _Object):
        raise refuse("`tasks` is not an object")
    for _, task in state["tasks"].pairs:
        if not isinstance(task, dict) or any(field not in task for field in TASK_FIELDS):
            raise refuse("a task lacks one of " + ", ".join(TASK_FIELDS))
    if not isinstance(state["plans"], list):
        raise refuse("`plans` is not a list")
    for plan in state["plans"]:
        if not isinstance(plan, dict) or "run_id" not in plan:
            raise refuse("a plan has no `run_id`")
    return state


def committed_events(path: Path) -> list[dict[str, Any]]:
    """Every newline-terminated line of a goal log, by the helper's rule: only `\n` ends a
    line and an unterminated tail is not an event. Call it only after `show_goal` read the
    same log, which is what refuses a blank line or one that is not JSON."""
    if not path.is_file():
        return []
    lines = path.read_bytes().split(b"\n")
    lines.pop()
    # Each line goes to the parser with its newline, as the helper hands it over: a line in
    # UTF-16 ends in the two bytes of one.
    return [json.loads(line + b"\n") for line in lines]


def labels_for_goal(
    state: dict[str, Any], events: list[dict[str, Any]]
) -> dict[str, dict[str, Any]]:
    """Run id to label fields, from the state `show_goal` returned and the log's events."""
    goal_text = (state["goal"] or {}).get("text", "")
    labels: dict[str, dict[str, Any]] = {}
    # As the fold keeps them: by id, where `3` and `3.0` are one task and the later wins.
    tasks: dict[Any, dict[str, Any]] = {}
    for _, task in state["tasks"].pairs:
        tasks[task["id"]] = task

    for task in tasks.values():
        if not task["run_id"]:
            continue
        labels[task["run_id"]] = {
            "task_id": task["id"],
            "class": task["class"],
            "grade": task["grade"],
            "human_verdict": task["human_result"],
            "text": task["text"],
        }

    for plan in state["plans"]:
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
            task = tasks.get(event.get("task"))
            for run_id in event.get("judge_run_ids", []):
                labels[run_id] = {
                    "task_id": event.get("task"),
                    "class": task["class"] if task else None,
                    "grade": task["grade"] if task else None,
                    "human_verdict": None,
                    "text": task["text"] if task else "",
                }

    return labels


def collect(home: Path | None = None) -> dict[str, dict[str, Any]]:
    """Ask `hydra-puppets show` for every goal log under ``home`` and return run_id -> label
    fields. One goal that cannot be shown raises `LabelsError`, and no label is returned. A
    file named `.jsonl` alone has no goal id and is skipped with a line on stderr. A name
    that starts with a dot is a goal like any other: the helper reads `.real.jsonl` for the
    goal `.real`."""
    home = home or puppets_home()
    labels: dict[str, dict[str, Any]] = {}
    for path in sorted(home.glob("*.jsonl")):
        # The id plus `.jsonl` is the name of the file read below, which `Path.stem` does
        # not promise: the stem of `.jsonl` is `.jsonl`, and `show` would be asked for a
        # log that is another file.
        goal_id = path.name.removesuffix(".jsonl")
        if not goal_id:
            print(f"labels: skipping {path}: its name is not a goal id", file=sys.stderr)
            continue
        state = show_goal(goal_id, home)
        for run_id, row in labels_for_goal(state, committed_events(path)).items():
            labels[run_id] = row
    return labels


def collect_llm(home: Path | None = None) -> dict[str, dict[str, Any]]:
    """Read every ``task-classes-*.jsonl`` under the grader home, oldest name first, so a later
    file wins on the same run_id. A model wrote these, not a human."""
    home = home or grader_home()
    found: dict[str, dict[str, Any]] = {}
    for path in sorted(home.glob("task-classes-*.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                item = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(item, dict) and item.get("run_id"):
                found[item["run_id"]] = item
    return found
