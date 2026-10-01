#!/usr/bin/env python3
"""Master of Puppets: goal log, fold, and launcher."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import time
import uuid
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, BinaryIO, Callable, Iterator, Literal, NamedTuple

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
APPROVAL_KINDS = frozenset({"plan", "table", "launch", "diff", "degraded"})
VERDICT_RESULTS = frozenset({"applied", "fixed", "rejected"})
PLAN_COPY_CAP = 10 * 1024 * 1024

class PanelSeat(NamedTuple):
    provider: str
    model: str | None
    policy: str
    role: str
    evidence: str


# Rung two's panel in src/rungs/review.rs keeps its own seat table and never substitutes;
# only this mode panel may sit degraded with human approval (map 81-a).
PUPPETS_PANEL_SEATS: list[PanelSeat] = [
    PanelSeat("claude", "opus", "judge", "reviewer", "diff+task"),
    PanelSeat("codex", None, "standard", "adversary", "diff"),
    PanelSeat("cursor-agent", "composer-2.5", "judge", "security", "diff+tests"),
]

PANEL_ROLE_BRIEF = {
    "reviewer": (
        "Your role is reviewer: decide whether the diff does what the task asked, "
        "naming what is missing if not."
    ),
    "adversary": (
        "Your role is adversary: argue the diff should be rejected; name the strongest reason."
    ),
    "security": (
        "Your role is security: apply the checklist items about secrets, paths, gates and the record."
    ),
}

SECURITY_CHECKLIST_ITEMS = frozenset(
    {
        "security_issues",
        "baseline_test_provenance",
        "tests_present",
        "scope_creep",
    }
)

TASK_DIFF_CHECKLIST: list[tuple[str, bool, str]] = [
    (
        "scope_match",
        False,
        "The diff implements what the task asked for, naming what is missing if not",
    ),
    (
        "scope_creep",
        True,
        "Changes outside the task scope or its named paths without justification",
    ),
    (
        "tests_present",
        False,
        "A behaviour change with no test touched, naming the function and file that should have moved",
    ),
    (
        "baseline_test_provenance",
        False,
        "A weakened assertion or a test edited without comparing against the baseline",
    ),
    (
        "security_issues",
        False,
        "A credential, unsafe construct, or shell interpolation of untrusted text",
    ),
    (
        "error_handling",
        True,
        "A recoverable error path silently swallowed or misreported",
    ),
    (
        "dead_code",
        True,
        "Obvious dead code, unused imports, or unreachable branches introduced",
    ),
    (
        "naming_clarity",
        True,
        "New identifiers that are unclear or fight repo conventions",
    ),
    (
        "self_consistency",
        False,
        "The diff contradicts the task text or its own structure",
    ),
    (
        "dependencies",
        True,
        "A new dependency the task did not require",
    ),
    (
        "documentation",
        True,
        "A user-visible change with no doc update where the task touched docs",
    ),
    (
        "code_quality",
        True,
        "Unnecessary duplication or complexity beyond what the task needs",
    ),
]

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


def grader_home() -> Path:
    named = os.environ.get("HYDRA_GRADER_HOME")
    if named:
        return Path(named)
    return hydra_root() / "grader"


def hydra_bin() -> str:
    return os.environ.get("HYDRA_BIN", "hydra")


def vendors_json_path() -> Path:
    override = os.environ.get("HYDRA_PUPPETS_VENDORS")
    if override:
        return Path(override)
    return Path(__file__).resolve().parent / "vendors.json"


def provider_available(provider: str) -> bool:
    """Whether a routed provider is marked available in vendors.json (not a live probe)."""
    path = vendors_json_path()
    try:
        vendors = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return True
    rows = [row for row in vendors if row.get("provider") == provider]
    if not rows:
        return True
    provider_row = next(
        (row for row in rows if row.get("model") in (None, "")),
        rows[0],
    )
    return bool(provider_row.get("available", True))


# The text `approve` prints when a seat is down. It is not itself a substitute:
# parse_degraded_substitution rejects a value that still starts with `<`, so running
# the pending argv unchanged writes nothing.
DEGRADED_SUBSTITUTE_PLACEHOLDER = "<substitute-provider[:model]>"


def degraded_substitution_spec(provider: str) -> str:
    return f"{provider}={DEGRADED_SUBSTITUTE_PLACEHOLDER}"


def parse_degraded_substitution(spec: str) -> tuple[str, str, str | None]:
    """Parse missing=substitute or missing=substitute:model."""
    if "=" not in spec:
        raise ValueError("expected missing=substitute-provider[:model]")
    missing, rhs = spec.split("=", 1)
    missing = missing.strip()
    if not missing:
        raise ValueError("missing provider name required")
    if ":" in rhs:
        substitute, model = rhs.split(":", 1)
        substitute = substitute.strip()
        model = model.strip() or None
    else:
        substitute = rhs.strip()
        model = None
    if not substitute or substitute.startswith("<"):
        raise ValueError(
            "substitute provider required; replace "
            f"{DEGRADED_SUBSTITUTE_PLACEHOLDER} with a provider"
        )
    return missing, substitute, model


def resolve_panel_seats(state: GoalState) -> tuple[list[ResolvedSeat], list[dict[str, Any]], str | None]:
    resolved: list[ResolvedSeat] = []
    substitutions: list[dict[str, Any]] = []
    for seat in PUPPETS_PANEL_SEATS:
        if provider_available(seat.provider):
            resolved.append(
                ResolvedSeat(
                    seat=seat,
                    provider=seat.provider,
                    model=seat.model,
                    policy=seat.policy,
                )
            )
            continue
        approval = approval_available(state, "degraded", ref=seat.provider)
        if approval is None:
            return (
                [],
                [],
                f"{seat.provider} is unavailable; approve with: approve {state.goal_id} "
                f"--degraded {degraded_substitution_spec(seat.provider)}",
            )
        substitute = approval.get("substitute") or ""
        model = approval.get("model") or None
        if not substitute:
            return [], [], f"degraded approval for {seat.provider} has no substitute"
        # The substitute keeps its own seat's policy, or `judge`, never the missing seat's:
        # codex sits under `standard` only because `judge` blocked its first read, and a
        # stand-in has no reason to inherit write and network access. Map 81-a.
        own = next((s.policy for s in PUPPETS_PANEL_SEATS if s.provider == substitute), "judge")
        resolved.append(
            ResolvedSeat(
                seat=seat,
                provider=substitute,
                model=model,
                policy=own,
                substituted=True,
                missing_provider=seat.provider,
            )
        )
        substitutions.append(
            {
                "missing": seat.provider,
                "substitute": substitute,
                "model": model or "",
            }
        )
    return resolved, substitutions, None


def goal_log_path(goal_id: str) -> Path:
    return puppets_home() / f"{goal_id}.jsonl"


def goal_dir(goal_id: str) -> Path:
    return puppets_home() / goal_id


def _log_lines(raw: bytes) -> list[bytes]:
    """The goal log's lines, each with its newline; the last has none when the file does
    not end on one. Only `\\n` ends a line. `bytes.splitlines` also breaks on a lone `\\r`,
    which made `BAD\\r` in the middle of the file read as an unterminated tail and hid
    every line after it (build 199). A CRLF line still parses, since `\\r` is JSON
    whitespace."""
    lines = raw.split(b"\n")
    tail = lines.pop()
    out = [line + b"\n" for line in lines]
    if tail:
        out.append(tail)
    return out


def damaged_line_numbers(raw: bytes) -> list[int]:
    """1-based line numbers of newline-terminated lines that are blank or not JSON."""
    bad: list[int] = []
    for index, line in enumerate(_log_lines(raw)):
        if not line.endswith(b"\n"):
            break
        if _bad_line(line):
            bad.append(index + 1)
    return bad


def _bad_line(line: bytes) -> bool:
    """Blank, not decodable, or not JSON. `json.loads` on bytes drops a BOM, as `read_log`
    always has. `ValueError` covers `JSONDecodeError` and `UnicodeDecodeError`, its
    subclasses, and the plain `ValueError` an integer past 4,300 digits raises
    (`sys.set_int_max_str_digits`), which used to escape as a traceback (build 199)."""
    if not line.strip():
        return True
    try:
        json.loads(line)
    except ValueError:
        return True
    return False


def _newline_terminated_events(raw: bytes) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line in _log_lines(raw):
        if not line.endswith(b"\n"):
            break
        events.append(json.loads(line))
    return events


def read_log(goal_id: str) -> list[dict[str, Any]]:
    path = goal_log_path(goal_id)
    if not path.is_file():
        return []
    raw = path.read_bytes()
    bad = damaged_line_numbers(raw)
    if bad:
        raise DamagedLog(goal_id, bad)
    return _newline_terminated_events(raw)


# The most bad lines one `repair` event is believed about.
MAX_REPAIR_REMOVED = 1_000_000_000


class DamagedLog(Exception):
    """A newline-terminated line is blank or is not JSON."""

    def __init__(self, goal_id: str, line_numbers: list[int]) -> None:
        self.goal_id = goal_id
        self.line_numbers = line_numbers
        nums = ", ".join(str(n) for n in line_numbers)
        super().__init__(
            f"goal log {goal_id} is damaged at line {nums}; wrote nothing; "
            f"puppets.py repair {goal_id}"
        )


class UnterminatedLog(Exception):
    """The goal log's last line has no newline, so a writer appends nothing."""

    def __init__(self, goal_id: str) -> None:
        self.goal_id = goal_id
        super().__init__(
            f"goal log {goal_id} ends without a newline; wrote nothing; "
            f"puppets.py repair {goal_id}"
        )


def _torn_tail(raw: bytes) -> bytes | None:
    """Bytes after the last newline when the file does not end on one."""
    if not raw or raw.endswith(b"\n"):
        return None
    return raw[raw.rfind(b"\n") + 1 :]


def _partition_log(raw: bytes) -> tuple[list[bytes], list[tuple[int, bytes]], bytes | None]:
    """Good lines, bad (line number, bytes), and an unterminated tail if any."""
    good: list[bytes] = []
    bad: list[tuple[int, bytes]] = []
    tail: bytes | None = None
    for index, line in enumerate(_log_lines(raw)):
        if not line.endswith(b"\n"):
            tail = line
            break
        if _bad_line(line):
            bad.append((index + 1, line))
        else:
            good.append(line)
    return good, bad, tail


@contextmanager
def _locked_log(path: Path, mode: str) -> Iterator[BinaryIO]:
    """Open the goal log and hold its lock, on the file the path names.

    `repair --yes` replaces the log by rename. A process that opened the log before
    that and waited for the lock would otherwise read, and append to, a file no path
    names any more. So after taking the lock this compares the inode of its descriptor
    with the path's and opens again when they differ."""
    while True:
        handle = path.open(mode)
        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            held = os.fstat(handle.fileno())
            try:
                named = os.stat(path)
            except FileNotFoundError:
                named = None
            current = named is not None and (named.st_dev, named.st_ino) == (
                held.st_dev,
                held.st_ino,
            )
        except BaseException:
            handle.close()
            raise
        if current:
            break
        fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
        handle.close()
    try:
        yield handle
    finally:
        fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
        handle.close()


class UnsafeRepairTemporary(Exception):
    def __str__(self):
        return "UnsafeRepairTemporary: repair temporary path is not a regular file; wrote nothing"


def _check_temporary(tmp: Path) -> None:
    try:
        mode = tmp.lstat().st_mode
    except FileNotFoundError:
        return
    if not stat.S_ISREG(mode):
        raise UnsafeRepairTemporary()


def _replace_log(path: Path, content: bytes, mode: int) -> None:
    """Put `content` where the goal log is, in one step: write it to a temporary file
    beside the log, fsync, and rename it over the log. The caller holds the log's lock.

    The rename leaves the old file to whoever still has it open, so every locker checks,
    once it holds the lock, that the path still names the file it locked."""
    tmp = path.with_name(path.name + ".repair-tmp")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW
    try:
        fd = os.open(tmp, flags, 0o600)
    except FileExistsError:
        _check_temporary(tmp)
        tmp.unlink()
        fd = os.open(tmp, flags, 0o600)
    try:
        with os.fdopen(fd, "wb") as out:
            os.fchmod(out.fileno(), stat.S_IMODE(mode))
            out.write(content)
            out.flush()
            os.fsync(out.fileno())
        os.replace(tmp, path)
    except BaseException:
        tmp.unlink(missing_ok=True)
        raise
    parent = os.open(path.parent, os.O_RDONLY)
    try:
        os.fsync(parent)
    finally:
        os.close(parent)


def _locked_update(
    goal_id: str, decide: Callable[[list[dict[str, Any]]], dict[str, Any] | None]
) -> dict[str, Any] | None:
    """Hold the goal log's lock, read it, and append what `decide` returns.

    `decide` sees newline-terminated lines read under that lock. An
    unterminated tail is left in place and is not one of those lines; when
    one is present this raises `UnterminatedLog` and writes nothing. None
    from `decide` writes nothing and the lock is still released. Callers
    that must not approve a stale revision re-fold those lines and refuse
    before returning a body.
    """
    path = goal_log_path(goal_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    # Bytes, so a lone `\r` stays as it is and a tail ending in CR without LF reads as
    # unterminated rather than being turned into a newline (codex review of 190).
    with _locked_log(path, "a+b") as handle:
        handle.seek(0)
        raw = handle.read()
        bad = damaged_line_numbers(raw)
        if bad:
            raise DamagedLog(goal_id, bad)
        events = _newline_terminated_events(raw)
        unterminated = bool(_torn_tail(raw))
        body = decide(events)
        if unterminated:
            raise UnterminatedLog(goal_id)
        if body is None:
            return None
        record = {"seq": len(events) + 1, "ts": utc_now(), **body}
        line = json.dumps(record, separators=(",", ":")) + "\n"
        handle.write(line.encode("utf-8"))
        handle.flush()
        return record


def append_event(goal_id: str, body: dict[str, Any]) -> dict[str, Any]:
    record = _locked_update(goal_id, lambda _events: body)
    if record is None:
        raise RuntimeError(f"append_event wrote nothing for goal {goal_id}")
    return record


@dataclass
class TaskState:
    id: str
    text: str
    task_class: str
    grade: str
    done: str | None = None
    deps: list[str] = field(default_factory=list)
    paths: list[str] = field(default_factory=list)
    assignment: dict[str, Any] | None = None
    status: str = "pending"
    run_id: str | None = None
    attempt: int | None = None
    outcome: str | None = None
    human_result: str | None = None
    review_verdict: str | None = None
    review_degraded: bool = False
    review_substitutions: list[dict[str, Any]] = field(default_factory=list)


@dataclass
class ResolvedSeat:
    seat: PanelSeat
    provider: str
    model: str | None
    policy: str
    substituted: bool = False
    missing_provider: str | None = None


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
                "degraded": event.get("degraded", False),
                "substitutions": event.get("substitutions", []),
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
                done=event.get("done"),
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
            entry: dict[str, Any] = {
                "kind": event["kind"],
                "ref": event.get("ref"),
                "run_id": event.get("run_id"),
                "revision": event["revision"],
                "yes": event["yes"],
                "seq": seq,
                "consumed": False,
            }
            if event["kind"] == "degraded":
                entry["missing"] = event["missing"]
                entry["substitute"] = event["substitute"]
                if event.get("model"):
                    entry["model"] = event["model"]
            state.approvals.append(entry)
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
                    and approval.get("run_id") == event.get("run_id")
                    and approval["revision"] == revision
                    and approval["yes"]
                    and approval["seq"] not in consumed_approvals
                ):
                    consumed_approvals.add(approval["seq"])
        elif name == "intent_abandoned":
            intents.pop((event["task"], event["attempt"]), None)
        elif name == "review":
            task = state.tasks.get(event["task"])
            if task is not None:
                task.review_verdict = event.get("verdict")
                task.review_degraded = event.get("degraded", False)
                task.review_substitutions = list(event.get("substitutions") or [])
        elif name == "repair":
            # Past every revision an approval before the repair could name. A `removed`
            # that is not an int from 1 to MAX_REPAIR_REMOVED counts as zero, so the event
            # still bumps by one. The cap is the Rust reader's too: with it no log that
            # fits in memory reaches a revision either side cannot hold (build 199).
            removed = event.get("removed", 0)
            counted = type(removed) is int and 1 <= removed <= MAX_REPAIR_REMOVED
            revision += (removed if counted else 0) + 1

    state.revision = revision
    for key, intent in intents.items():
        if key not in runs_by_attempt:
            state.dangling_intents.append(intent)

    for approval in state.approvals:
        if approval["seq"] in consumed_approvals:
            approval["consumed"] = True
        elif approval["kind"] != "plan" and approval["revision"] != revision:
            approval["voided"] = True

    return state


def approval_available(
    state: GoalState,
    kind: str,
    ref: str | None = None,
    run_id: str | None = None,
) -> dict[str, Any] | None:
    for approval in state.approvals:
        if (
            approval.get("consumed")
            or approval.get("voided")
            or not approval["yes"]
            or approval["kind"] != kind
        ):
            continue
        # A plan approval is about the chosen plan, not the task table: importing the plan's
        # tasks moves the revision and must not void the approval that allowed the import.
        if kind != "plan" and approval["revision"] != state.revision:
            continue
        if kind == "degraded" and approval.get("missing") != ref:
            continue
        if kind in ("launch", "diff") and approval.get("ref") != ref:
            continue
        if kind == "diff" and run_id is not None and approval.get("run_id") != run_id:
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


def approve_argv(goal_id: str, revision: int, *args: str) -> list[str]:
    """The command that records one approval, bound to the revision it was drawn from."""
    return [
        sys.executable,
        str(Path(__file__).resolve()),
        "approve",
        goal_id,
        *args,
        "--expect-revision",
        str(revision),
    ]


def _pending_item(
    state: GoalState, kind: str, ref: str | None, *args: str
) -> dict[str, Any]:
    return {
        "kind": kind,
        "ref": ref,
        "revision": state.revision,
        "argv": approve_argv(state.goal_id, state.revision, *args),
    }


def panel_gate_reachable(state: GoalState) -> bool:
    """True when a command would consult the panel seats from this fold.

    `choose --judge` reaches them once two proposals have paths. `review` reaches
    them once a task has a run. Earlier refusals (not enough proposals, no run)
    are not approval waits. `review` can still refuse first when the run record
    has no branch; that check reads the record, which this fold does not.
    """
    ready = sum(1 for plan in state.plans if plan.get("path"))
    if ready >= 2:
        return True
    return any(task.run_id for task in state.tasks.values())


def degraded_provider_waiting(state: GoalState) -> str | None:
    """The missing provider `resolve_panel_seats` would ask the human to approve.

    The helper names one seat, the first it cannot fill, and stops. A later seat
    is not waiting until that one is approved.
    """
    _resolved, _substitutions, refusal = resolve_panel_seats(state)
    if not refusal or "approve with:" not in refusal:
        return None
    provider, separator, _rest = refusal.partition(" is unavailable")
    if not separator or not provider or " " in provider:
        return None
    return provider


def pending_approvals(state: GoalState) -> list[dict[str, Any]]:
    """Approvals the helper is waiting on, from the same fold its refusals read.

    A launch approval is listed only while the task is still `assigned`. Once a
    run has spent it, `launch` would refuse a second time, and that is not an
    open wait: listing it would keep every finished codex task in the lane.
    A diff is listed once the task has a run and no human verdict yet, which is
    when `verdict applied` refuses, including while the run is still marked
    running. A verdict already recorded is not listed again.
    """
    items: list[dict[str, Any]] = []
    if (
        state.chosen_plan
        and state.chosen_plan.get("path")
        and approval_available(state, "plan") is None
    ):
        items.append(
            _pending_item(
                state,
                "plan",
                None,
                "--plan",
                "--expect-plan",
                state.chosen_plan["path"],
            )
        )

    seen_table = False
    for task in state.tasks.values():
        refusal = launch_refusal(state, task.id)
        if not refusal:
            continue
        if refusal.startswith("missing table approval") and not seen_table:
            items.append(_pending_item(state, "table", None, "--table"))
            seen_table = True
        elif (
            refusal.startswith("missing launch approval") and task.status == "assigned"
        ):
            items.append(
                _pending_item(state, "launch", task.id, "--launch", task.id)
            )
    for task in state.tasks.values():
        run_id = task.run_id
        if not run_id or task.human_result is not None:
            continue
        if approval_available(state, "diff", task.id, run_id) is None:
            items.append(
                _pending_item(
                    state,
                    "diff",
                    task.id,
                    "--diff",
                    task.id,
                    "--expect-run",
                    run_id,
                )
            )
    if panel_gate_reachable(state):
        provider = degraded_provider_waiting(state)
        if provider is not None:
            items.append(
                _pending_item(
                    state,
                    "degraded",
                    provider,
                    "--degraded",
                    degraded_substitution_spec(provider),
                )
            )
    return items


def next_attempt(goal_id: str, task_id: str) -> int:
    attempts = [
        event["attempt"]
        for event in read_log(goal_id)
        if event.get("event") == "launch_intent" and event.get("task") == task_id
    ]
    return max(attempts, default=0) + 1


def worker_task_text(task: TaskState) -> str:
    """What a worker run is actually launched with: the task text, the plan's own definition
    of done (or an admission that the plan did not state one), and a standing ask to report
    what the run could not confirm before what it changed."""
    done = task.done or "not stated by the plan; say in your report what you took it to be."
    return (
        f"{task.text}\n\n"
        f"Done means: {done}\n\n"
        "When you report back, start with a section headed \"Not confirmed\": every claim "
        "you could not check yourself, and where you looked. Then say what changed."
    )


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


def prompts_dir() -> Path:
    return Path(__file__).resolve().parent / "prompts"


def strip_one_fence(text: str) -> str:
    trimmed = text.strip()
    if not trimmed.startswith("```"):
        return trimmed
    rest = trimmed[3:]
    if "\n" not in rest:
        return trimmed
    tag, body = rest.split("\n", 1)
    if tag.strip() and tag.strip().lower() != "json":
        return trimmed
    body = body.rstrip()
    if not body.endswith("```"):
        return trimmed
    body = body[:-3]
    if "```" in body:
        return trimmed
    return body.strip()


@dataclass
class JudgeFinding:
    item: str
    severity: Literal["critical", "major", "minor"]
    note: str


@dataclass
class JudgeOutcome:
    provider: str
    verdict: Literal["pass", "block"] | None
    findings: list[JudgeFinding]
    parse_error: str | None = None


@dataclass
class PlanAnswer:
    provider: str
    chosen: str | None
    findings: list[JudgeFinding]
    composed_plan: str | None = None
    parse_error: str | None = None


def checklist_ceiling(item: str, checklist: list[tuple[str, bool, str]]) -> str | None:
    for entry_id, advisory, _definition in checklist:
        if entry_id == item:
            return "minor" if advisory else "critical"
    return None


def clamp_severity(
    reported: Literal["critical", "major", "minor"], ceiling: str
) -> Literal["critical", "major", "minor"]:
    order = {"minor": 1, "major": 2, "critical": 3}
    if order[reported] > order[ceiling]:
        return ceiling  # type: ignore[return-value]
    return reported


def parse_judge_answer(
    provider: str, text: str, checklist: list[tuple[str, bool, str]] = TASK_DIFF_CHECKLIST
) -> JudgeOutcome:
    trimmed = strip_one_fence(text)
    try:
        parsed = json.loads(trimmed)
    except json.JSONDecodeError as err:
        return JudgeOutcome(provider, None, [], parse_error=str(err))
    if not isinstance(parsed, dict):
        return JudgeOutcome(provider, None, [], parse_error="answer is not a JSON object")
    verdict = parsed.get("verdict")
    if verdict not in ("pass", "block"):
        return JudgeOutcome(
            provider,
            None,
            [],
            parse_error=f"verdict {verdict!r} is not pass or block",
        )
    raw_findings = parsed.get("findings")
    if not isinstance(raw_findings, list):
        return JudgeOutcome(provider, None, [], parse_error="findings is not an array")
    findings: list[JudgeFinding] = []
    for raw in raw_findings:
        if not isinstance(raw, dict):
            return JudgeOutcome(provider, None, [], parse_error="finding is not an object")
        item = raw.get("item")
        severity = raw.get("severity")
        note = raw.get("note")
        if not isinstance(item, str) or not isinstance(note, str):
            return JudgeOutcome(provider, None, [], parse_error="finding fields must be strings")
        ceiling = checklist_ceiling(item, checklist)
        if ceiling is None:
            return JudgeOutcome(
                provider,
                None,
                [],
                parse_error=f"unknown checklist item {item!r}",
            )
        if severity not in ("critical", "major", "minor"):
            return JudgeOutcome(
                provider,
                None,
                [],
                parse_error=f"severity {severity!r} is not critical, major or minor",
            )
        findings.append(
            JudgeFinding(
                item=item,
                severity=clamp_severity(severity, ceiling),
                note=note,
            )
        )
    return JudgeOutcome(provider, verdict, findings)


def outcome_blocks(outcome: JudgeOutcome) -> bool:
    if outcome.parse_error is not None:
        return True
    if outcome.verdict == "block":
        return True
    return any(finding.severity == "critical" for finding in outcome.findings)


def diff_panel_verdict(answers: list[JudgeOutcome]) -> tuple[str, list[str]]:
    reasons: list[str] = []
    for outcome in answers:
        if outcome.parse_error is not None:
            reasons.append(f"{outcome.provider}: parse error: {outcome.parse_error}")
            continue
        if outcome.verdict == "block":
            reasons.append(f"{outcome.provider}: verdict block")
        for finding in outcome.findings:
            if finding.severity == "critical":
                reasons.append(
                    f"{outcome.provider}: critical {finding.item}: {finding.note}"
                )
    if reasons:
        return "block", reasons
    return "pass", []


def parse_plan_answer(provider: str, text: str) -> PlanAnswer:
    trimmed = strip_one_fence(text.strip())
    try:
        parsed, _end = json.JSONDecoder().raw_decode(trimmed)
    except json.JSONDecodeError as err:
        return PlanAnswer(provider, None, [], parse_error=str(err))
    if not isinstance(parsed, dict):
        return PlanAnswer(provider, None, [], parse_error="answer is not a JSON object")
    chosen = parsed.get("chosen")
    if not isinstance(chosen, str) or not chosen.strip():
        return PlanAnswer(provider, None, [], parse_error="chosen must be a non-empty string")
    raw_findings = parsed.get("findings", [])
    if not isinstance(raw_findings, list):
        return PlanAnswer(provider, None, [], parse_error="findings is not an array")
    findings: list[JudgeFinding] = []
    for raw in raw_findings:
        if not isinstance(raw, dict):
            return PlanAnswer(provider, None, [], parse_error="finding is not an object")
        item = raw.get("item")
        severity = raw.get("severity")
        note = raw.get("note")
        if not isinstance(item, str) or not isinstance(note, str):
            return PlanAnswer(provider, None, [], parse_error="finding fields must be strings")
        if severity not in ("critical", "major", "minor"):
            return PlanAnswer(
                provider,
                None,
                [],
                parse_error=f"severity {severity!r} is not critical, major or minor",
            )
        findings.append(JudgeFinding(item=item, severity=severity, note=note))
    composed_plan = None
    if chosen == "composed":
        try:
            tasks = extract_tasks_json(text)
            composed_plan = json.dumps(tasks, indent=2)
        except ValueError:
            composed_plan = None
    return PlanAnswer(provider, chosen, findings, composed_plan=composed_plan)


def plan_panel_verdict(
    seats: list[PanelSeat], answers: list[PlanAnswer | None]
) -> tuple[str | None, str | None, list[str]]:
    reasons: list[str] = []
    for seat, answer in zip(seats, answers):
        provider = seat.provider
        if answer is None:
            reasons.append(f"missing {provider}")
            continue
        if answer.parse_error is not None:
            reasons.append(f"{provider}: parse error: {answer.parse_error}")
    if reasons:
        return None, None, reasons
    chosen_values = [answer.chosen for answer in answers if answer is not None]
    if len(set(chosen_values)) != 1:
        pairs = ", ".join(
            f"{seat.provider}={answer.chosen}"
            for seat, answer in zip(seats, answers)
            if answer is not None
        )
        return None, None, [f"disagreement: {pairs}"]
    chosen = chosen_values[0]
    composed_text = None
    if chosen == "composed":
        for answer in answers:
            if answer is not None and answer.composed_plan:
                composed_text = answer.composed_plan
                break
        if composed_text is None:
            return None, None, ["composed chosen but no task block found"]
    return chosen, composed_text, []


def load_choose_prompt(goal_text: str, proposals: list[dict[str, Any]]) -> str:
    template = (prompts_dir() / "choose.md").read_text(encoding="utf-8")
    blocks: list[str] = []
    providers: list[str] = []
    for proposal in proposals:
        provider = proposal["provider"]
        providers.append(provider)
        path = proposal.get("path")
        body = Path(path).read_text(encoding="utf-8") if path else ""
        blocks.append(f"### {provider}\n\n{body.strip()}\n")
    return (
        template.replace("{{GOAL}}", goal_text)
        .replace("{{PROPOSALS}}", "\n".join(blocks))
        .replace("{{PROVIDER_NAMES}}", ", ".join(providers))
    )


def load_diff_prompt(
    task_text: str,
    base_sha: str,
    projection: str,
    diff_text: str,
    checklist: list[tuple[str, bool, str]] | None = None,
) -> str:
    template = (prompts_dir() / "diff.md").read_text(encoding="utf-8")
    checklist = checklist if checklist is not None else TASK_DIFF_CHECKLIST
    checklist_lines = []
    for item_id, advisory, definition in checklist:
        ceiling = "advisory" if advisory else "critical"
        checklist_lines.append(f"- {item_id} ({ceiling}): {definition}")
    return (
        template.replace("{{TASK}}", task_text)
        .replace("{{BASE_SHA}}", base_sha)
        .replace("{{PROJECTION}}", projection.strip() or "(no projection)")
        .replace("{{DIFF}}", diff_text.strip() or "(empty diff)")
        .replace("{{CHECKLIST}}", "\n".join(checklist_lines))
    )


def test_paths_from_diff(diff_text: str) -> str:
    """Unified-diff hunks whose paths look like tests."""
    if not diff_text.strip():
        return "(no test paths in diff)"
    keep: list[str] = []
    current: list[str] = []
    for line in diff_text.splitlines():
        if line.startswith("diff --git ") or line.startswith("--- ") or line.startswith("+++ "):
            path_bits = line.split()
            path = path_bits[-1] if path_bits else ""
            if path.startswith("b/"):
                path = path[2:]
            is_test = (
                "/tests/" in path
                or path.startswith("tests/")
                or "test_" in path
                or path.endswith("_test.rs")
                or "/test/" in path
            )
            if is_test:
                if current:
                    keep.extend(current)
                    keep.append("")
                current = [line]
            else:
                current = []
        elif current and (
            line.startswith("@@")
            or line.startswith("+")
            or line.startswith("-")
            or line.startswith(" ")
        ):
            current.append(line)
    if current:
        keep.extend(current)
    return "\n".join(keep).strip() or "(no test paths in diff)"


def build_diff_seat_prompt(
    seat: PanelSeat,
    task_text: str,
    base_sha: str,
    projection: str,
    diff_text: str,
) -> str:
    role_line = PANEL_ROLE_BRIEF[seat.role]
    parts = [f"You are one judge on a three-seat panel reviewing a task diff.\n\n## Role\n\n{role_line}\n"]
    if seat.evidence == "diff+task":
        parts.append(f"## Task\n\n{task_text}\n")
    if seat.evidence in ("diff", "diff+task", "diff+tests"):
        parts.append(
            "## Diff\n\n```diff\n"
            + (diff_text.strip() or "(empty diff)")
            + "\n```\n"
        )
    if seat.evidence == "diff+tests":
        parts.append(f"## Projection\n\n{projection.strip() or '(no projection)'}\n")
        parts.append(
            "## Tests in diff\n\n```diff\n"
            + test_paths_from_diff(diff_text)
            + "\n```\n"
        )
        parts.append(f"## Base\n\nRepository base at `{base_sha}`.\n")
    checklist = TASK_DIFF_CHECKLIST
    if seat.role == "security":
        checklist = [row for row in TASK_DIFF_CHECKLIST if row[0] in SECURITY_CHECKLIST_ITEMS]
    parts.append(
        "## Instructions\n\nReview against the checklist for your role. Answer in exactly one JSON "
        "object and nothing else:\n\n"
        '```json\n{"verdict": "pass" | "block", "findings": [{"item": <checklist id>, '
        '"severity": "critical" | "major" | "minor", "note": <string>}]}\n```\n\n'
        "Checklist items:\n\n"
    )
    for item_id, advisory, definition in checklist:
        ceiling = "advisory" if advisory else "critical"
        parts.append(f"- {item_id} ({ceiling}): {definition}\n")
    return "".join(parts)


def build_diff_panel_prompts(
    resolved: list[ResolvedSeat],
    task_text: str,
    base_sha: str,
    projection: str,
    diff_text: str,
) -> list[tuple[ResolvedSeat, str]]:
    return [
        (
            seat,
            build_diff_seat_prompt(seat.seat, task_text, base_sha, projection, diff_text),
        )
        for seat in resolved
    ]


def build_choose_seat_prompt(
    seat: PanelSeat,
    goal_text: str,
    proposal_blocks: str,
    provider_names: str,
) -> str:
    role_line = PANEL_ROLE_BRIEF[seat.role]
    parts = [
        "You are one judge on a three-seat panel choosing a work plan.\n\n",
        f"## Role\n\n{role_line}\n\n",
    ]
    if seat.evidence == "diff+task":
        parts.append(f"## Goal\n\n{goal_text}\n\n")
    if seat.evidence in ("diff", "diff+task", "diff+tests"):
        parts.append(f"## Proposals\n\n{proposal_blocks}\n")
    if seat.evidence == "diff+tests":
        parts.append(
            "## Note\n\nWeigh test and verification coverage in each proposal before you choose.\n\n"
        )
    parts.append(
        "## Instructions\n\n"
        f"Pick exactly one proposer name ({provider_names}) or `composed` when no single proposal "
        "is best.\n\n"
        "Answer in exactly one JSON object and nothing else:\n\n"
        '```json\n{"chosen": "<provider>|composed", "findings": []}\n```\n'
    )
    return "".join(parts)


def build_choose_panel_prompts(
    resolved: list[ResolvedSeat],
    goal_text: str,
    proposals: list[dict[str, Any]],
) -> list[tuple[ResolvedSeat, str]]:
    blocks: list[str] = []
    providers: list[str] = []
    for proposal in proposals:
        provider = proposal["provider"]
        providers.append(provider)
        path = proposal.get("path")
        body = Path(path).read_text(encoding="utf-8") if path else ""
        blocks.append(f"### {provider}\n\n{body.strip()}\n")
    proposal_blocks = "\n".join(blocks)
    provider_names = ", ".join(providers)
    return [
        (
            seat,
            build_choose_seat_prompt(seat.seat, goal_text, proposal_blocks, provider_names),
        )
        for seat in resolved
    ]


def spawn_judge_run(provider: str, model: str | None, policy: str, task_text: str) -> str:
    return spawn_run(build_run_argv(provider, model, policy, task_text))


def wait_for_run(run_id: str) -> None:
    while not run_file_has_end(run_id):
        time.sleep(0.05)


def read_run_text(run_id: str) -> str:
    return hydra_show_text(run_id)


def run_judge_panel(
    seat_prompts: list[tuple[ResolvedSeat, str]],
) -> tuple[list[str], list[str], list[str]]:
    run_ids: list[str] = []
    policies: list[str] = []
    texts: list[str] = []
    for resolved, task_text in seat_prompts:
        run_id = spawn_judge_run(resolved.provider, resolved.model, resolved.policy, task_text)
        run_ids.append(run_id)
        policies.append(resolved.policy)
        wait_for_run(run_id)
        texts.append(read_run_text(run_id))
    return run_ids, policies, texts


def copy_regular_file(src: Path, dest: Path, cap: int = PLAN_COPY_CAP) -> None:
    """Copy one regular file without following links, size-capped."""
    flags = os.O_RDONLY
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    fd = os.open(src, flags)
    try:
        st = os.fstat(fd)
        if not stat.S_ISREG(st.st_mode):
            raise ValueError(f"{src} is not a regular file")
        if st.st_nlink != 1:
            raise ValueError(f"{src} has {st.st_nlink} links")
        if st.st_size > cap:
            raise ValueError(f"{src} exceeds the {cap} byte cap")
        with os.fdopen(fd, "rb") as handle:
            fd = -1
            data = handle.read(cap + 1)
    finally:
        if fd >= 0:
            os.close(fd)
    if len(data) > cap:
        raise ValueError(f"{src} exceeds the {cap} byte cap")
    dest.write_bytes(data)


def git_diff_branch(repo: Path, base_sha: str, branch: str) -> str:
    if not base_sha or not branch:
        return ""
    try:
        return subprocess.check_output(
            ["git", "diff", f"{base_sha}...{branch}"],
            cwd=repo,
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return ""


def first_changed_file(repo: Path, base_sha: str, branch: str) -> str | None:
    if not base_sha or not branch:
        return None
    try:
        output = subprocess.check_output(
            ["git", "diff", "--name-only", f"{base_sha}...{branch}"],
            cwd=repo,
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    if not output:
        return None
    return output.splitlines()[0]


def findings_to_hunk_comments(
    outcomes: list[JudgeOutcome], repo: Path, base_sha: str, branch: str
) -> list[dict[str, Any]]:
    default_file = first_changed_file(repo, base_sha, branch) or "."
    comments: list[dict[str, Any]] = []
    for outcome in outcomes:
        for finding in outcome.findings:
            if finding.severity == "minor":
                continue
            comments.append(
                {
                    "filePath": default_file,
                    "hunkNumber": 1,
                    "summary": (
                        f"{outcome.provider}: {finding.item} ({finding.severity}) — "
                        f"{finding.note}"
                    ),
                    "author": "puppets",
                }
            )
    return comments


def push_hunk_comments(cwd: Path, comments: list[dict[str, Any]]) -> bool:
    if not comments or not shutil.which("hunk"):
        return False
    payload = json.dumps({"comments": comments})
    proc = subprocess.run(
        ["hunk", "session", "comment", "apply", "--repo", str(cwd), "--stdin"],
        input=payload,
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode == 0


def fork_hunk_comment_pusher(cwd: Path, comments: list[dict[str, Any]]) -> None:
    if not comments:
        return
    pid = os.fork()
    if pid != 0:
        return
    for _ in range(30):
        if push_hunk_comments(cwd, comments):
            os._exit(0)
        time.sleep(1)
    os._exit(1)


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


class VerdictScores(NamedTuple):
    """Verdict-model probabilities from `scores.jsonl`. `by_run` holds `p` only for the
    newest verdict `model_version` in the file, so one model orders the whole queue."""

    model_version: str | None
    by_run: dict[str, float]


def load_verdict_scores(path: Path) -> VerdictScores:
    """Read verdict scores. The newest model version is the verdict line with the latest
    `scored_at`; a later line breaks a tie. Older versions of the same run do not win,
    and outcome scores are ignored. A missing file is an empty score set. Nothing is written."""
    if not path.is_file():
        return VerdictScores(None, {})
    rows: list[tuple[str, int, str, str, float]] = []
    best_key: tuple[str, int] | None = None
    best_version: str | None = None
    for index, line in enumerate(path.read_text(encoding="utf-8").splitlines()):
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if not isinstance(row, dict) or row.get("label") != "verdict":
            continue
        version = row.get("model_version")
        run_id = row.get("run_id")
        p = row.get("p")
        if not isinstance(version, str) or not isinstance(run_id, str):
            continue
        if isinstance(p, bool) or not isinstance(p, (int, float)):
            continue
        scored_at = row.get("scored_at")
        if not isinstance(scored_at, str):
            scored_at = ""
        key = (scored_at, index)
        rows.append((scored_at, index, version, run_id, float(p)))
        if best_key is None or key > best_key:
            best_key = key
            best_version = version
    by_run: dict[str, float] = {}
    if best_version is None:
        return VerdictScores(None, {})
    for _scored_at, _index, version, run_id, p in rows:
        if version == best_version:
            by_run[run_id] = p
    return VerdictScores(best_version, by_run)


def human_verdict_runs(events: list[dict[str, Any]]) -> dict[str, set[str]]:
    """Run ids that already have a human verdict, per task. A later attempt keeps its own run."""
    found: dict[str, set[str]] = {}
    for event in events:
        if event.get("event") != "human_verdict":
            continue
        task_id = event.get("task")
        run_id = event.get("run_id")
        if isinstance(task_id, str) and isinstance(run_id, str):
            found.setdefault(task_id, set()).add(run_id)
    return found


def awaits_review(task: TaskState, verdicts: dict[str, set[str]]) -> bool:
    """The current run has ended, and no human verdict names that run."""
    if task.status != "ended" or not task.run_id:
        return False
    return task.run_id not in verdicts.get(task.id, ())


def format_p(p: float | None) -> str:
    if p is None:
        return "no score"
    return f"{p:.2f}"


def ordered_review_tasks(
    state: GoalState, events: list[dict[str, Any]], scores: VerdictScores
) -> list[TaskState]:
    verdicts = human_verdict_runs(events)
    waiting = [task for task in state.tasks.values() if awaits_review(task, verdicts)]

    def key(task: TaskState) -> tuple:
        p = scores.by_run.get(task.run_id or "")
        if p is None:
            return (1, 0.0, task.id)
        return (0, -p, task.id)

    waiting.sort(key=key)
    return waiting


def cmd_board(args: argparse.Namespace) -> int:
    """The goal at a glance: plans, tasks with their assignment and status, and what the
    human still has to approve. A task waiting for a review shows the verdict model's `p`.
    `show` is the same state as JSON."""
    events = read_log(args.goal)
    state = fold(args.goal, events)
    if state.goal is None:
        print(f"unknown goal {args.goal}", file=sys.stderr)
        return 1
    scores = load_verdict_scores(grader_home() / "scores.jsonl")
    verdicts = human_verdict_runs(events)
    print(f"goal {state.goal_id}  revision {state.revision}")
    print(f"  {state.goal['text']}")
    if state.plans:
        print("plans")
        for plan in state.plans:
            ready = "ready" if plan.get("path") else "running"
            print(f"  {plan['provider']:<13} {plan['run_id'][:8]}  {ready}")
    if state.chosen_plan:
        if state.chosen_plan.get("degraded"):
            subs = state.chosen_plan.get("substitutions") or []
            sub_text = ", ".join(
                f"{s['missing']}->{s['substitute']}" for s in subs
            )
            print(
                f"chosen (degraded panel)  {state.chosen_plan['chosen']}  "
                f"judges {len(state.chosen_plan['judge_run_ids'])}  {sub_text}"
            )
        else:
            print(
                f"chosen  {state.chosen_plan['chosen']}  "
                f"judges {len(state.chosen_plan['judge_run_ids'])}"
            )
    if state.tasks:
        print("tasks")
        print(f"  {'id':<12} {'class':<21} {'grade':<6} {'agent':<28} {'status':<9} {'p':<8} run")
        for task in state.tasks.values():
            agent = "(unassigned)"
            if task.assignment:
                agent = f"{task.assignment['provider']} {task.assignment.get('model') or ''}".strip()
            run = (task.run_id or "")[:8]
            verdict = f"  {task.human_result}" if task.human_result else ""
            if task.review_verdict and not task.human_result:
                label = task.review_verdict
                if task.review_degraded:
                    label = f"{label} (degraded panel)"
                verdict = f"  review {label}"
            p_cell = ""
            if awaits_review(task, verdicts):
                p_cell = format_p(scores.by_run.get(task.run_id or ""))
            print(
                f"  {task.id:<12} {task.task_class:<21} {task.grade:<6} {agent:<28} "
                f"{task.status:<9} {p_cell:<8} {run}{verdict}"
            )
    pending = []
    if state.plans and state.chosen_plan and approval_available(state, "plan") is None:
        pending.append("plan")
    if state.tasks and approval_available(state, "table") is None:
        pending.append("table")
    for task in state.tasks.values():
        if task.assignment and needs_launch_approval(task) and task.status in ("assigned",) \
                and approval_available(state, "launch", task.id) is None:
            pending.append(f"launch {task.id}")
        if task.status == "ended" and approval_available(state, "diff", task.id, task.run_id) is None:
            pending.append(f"diff {task.id}")
    if state.dangling_intents:
        pending.append("reconcile " + ", ".join(f"{i['task']}#{i['attempt']}" for i in state.dangling_intents))
    print("waiting on you: " + (", ".join(pending) if pending else "nothing"))
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
        pane = attach_pane(run_id, provider, "standard")
        print(f"proposed {provider} run {run_id}" + (f" in {pane}" if pane else ""))
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
        sources = (
            sorted(
                p
                for p in src_dir.iterdir()
                if p.is_file() and not p.is_symlink()
            )
            if src_dir.is_dir()
            else []
        )
        target = dest / f"{plan['provider']}.md"
        copied_from_worktree = False
        for source in sources:
            try:
                copy_regular_file(source, target)
                copied_from_worktree = True
                break
            except (OSError, ValueError) as err:
                print(f"skipping {source}: {err}", file=sys.stderr)
        if not copied_from_worktree:
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
    if args.judge:
        state = fold(args.goal)
        if state.goal is None:
            print(f"unknown goal {args.goal}", file=sys.stderr)
            return 1
        ready = [plan for plan in state.plans if plan.get("path")]
        if len(ready) < 2:
            print("need at least two finished proposals before judging", file=sys.stderr)
            return 1
        resolved, substitutions, refusal = resolve_panel_seats(state)
        if refusal:
            print(refusal, file=sys.stderr)
            return 1
        seat_prompts = build_choose_panel_prompts(resolved, state.goal["text"], ready)
        run_ids, policies, texts = run_judge_panel(seat_prompts)
        answers = [
            parse_plan_answer(resolved.provider, text)
            for resolved, text in zip(resolved, texts)
        ]
        for resolved, policy, run_id in zip(resolved, policies, run_ids):
            note = f" for {resolved.missing_provider}" if resolved.substituted else ""
            print(f"judge {resolved.provider}{note} ({policy}) run {run_id}")
        chosen, composed_text, reasons = plan_panel_verdict(PUPPETS_PANEL_SEATS, answers)
        if chosen is None:
            for reason in reasons:
                print(reason, file=sys.stderr)
            return 1
        path = None
        if chosen == "composed":
            dest = goal_dir(args.goal) / "plans" / "composed.md"
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(
                "Composed plan.\n\n```json\n" + composed_text + "\n```\n",
                encoding="utf-8",
            )
            path = str(dest)
        else:
            for plan in ready:
                if plan["provider"] == chosen:
                    path = plan.get("path")
                    break
            if path is None:
                print(f"no plan file for chosen provider {chosen}", file=sys.stderr)
                return 1
        append_event(
            args.goal,
            {
                "event": "panel_verdict",
                "judge_run_ids": run_ids,
                "judge_policies": policies,
                "chosen": chosen,
                "path": path,
                "degraded": bool(substitutions),
                "substitutions": substitutions,
            },
        )
        print(f"chosen {chosen} -> {path}")
        return 0

    if not args.choice:
        print("specify a provider, a plan path, or --judge", file=sys.stderr)
        return 1
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
    if approval_available(state, "plan") is None:
        print(
            f"missing plan approval at revision {state.revision}; "
            f"approve with: approve {args.goal} --plan",
            file=sys.stderr,
        )
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
                "done": task.get("done"),
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


def _refuse(code: int, message: str | None) -> int:
    if message:
        print(message, file=sys.stderr)
    return code


def _approval_matches(
    approval: dict[str, Any], kind: str, ref: Any, run_id: str | None, revision: int
) -> bool:
    if not approval.get("yes") or approval.get("kind") != kind:
        return False
    if approval.get("revision") != revision:
        return False
    if kind == "degraded":
        existing_ref = approval.get("missing")
    else:
        existing_ref = approval.get("ref")
    if existing_ref != ref:
        return False
    return approval.get("run_id") == run_id


def _duplicate_refusal(
    state: GoalState, kind: str, ref: Any, run_id: str | None
) -> tuple[int, str, None] | None:
    for approval in state.approvals:
        if _approval_matches(approval, kind, ref, run_id, state.revision):
            return 3, f"already approved {kind} at revision {state.revision}", None
    return None


def prepare_approval(
    args: argparse.Namespace, state: GoalState
) -> tuple[int, str | None, dict[str, Any] | None]:
    """The approval this command would record from `state`, or a refusal.

    Exit 2 is an `--expect-*` mismatch. Exit 3 is the same kind, ref and run
    already in the log at this revision. Exit 1 is any other refusal. A degraded
    approval's ref is the missing provider. A diff's run is that task's current
    run id; other kinds have none.
    """
    expected = args.expect_revision
    if expected is not None and expected != state.revision:
        return (
            2,
            f"expected revision {expected}, current revision {state.revision}",
            None,
        )
    if args.expect_run is not None and not args.diff:
        return 1, "use --expect-run with --diff", None
    if args.expect_plan is not None and not args.plan:
        return 1, "use --expect-plan with --plan", None
    if args.degraded:
        if args.plan or args.table or args.launch or args.diff:
            return 1, "use --degraded without --plan, --table, --launch or --diff", None
        try:
            missing, substitute, model = parse_degraded_substitution(args.degraded)
        except ValueError as err:
            return 1, str(err), None
        duplicate = _duplicate_refusal(state, "degraded", missing, None)
        if duplicate is not None:
            return duplicate
        body: dict[str, Any] = {
            "event": "approval",
            "kind": "degraded",
            "missing": missing,
            "substitute": substitute,
            "revision": state.revision,
            "yes": True,
        }
        if model:
            body["model"] = model
        return 0, None, body
    if args.plan:
        kind, ref = "plan", None
        if args.expect_plan is not None:
            current = (state.chosen_plan or {}).get("path")
            if current != args.expect_plan:
                shown = current if current else "none"
                return 2, f"expected plan {args.expect_plan}, current plan {shown}", None
    elif args.table:
        kind, ref = "table", None
    elif args.launch:
        kind, ref = "launch", args.launch
    elif args.diff:
        kind, ref = "diff", args.diff
        if args.expect_run is not None:
            task = state.tasks.get(ref)
            current = task.run_id if task is not None else None
            if current != args.expect_run:
                shown = current if current else "none"
                return 2, f"expected run {args.expect_run}, current run {shown}", None
    else:
        return 1, "specify --plan, --table, --launch, --diff or --degraded", None
    if ref is not None and ref not in state.tasks:
        return 1, f"unknown task {ref}", None
    run_id: str | None = None
    if kind == "diff":
        task = state.tasks[ref]
        if not task.run_id:
            return 1, f"task {ref} has no run to approve a diff for", None
        run_id = task.run_id
    duplicate = _duplicate_refusal(state, kind, ref, run_id)
    if duplicate is not None:
        return duplicate
    body = {
        "event": "approval",
        "kind": kind,
        "ref": ref,
        "revision": state.revision,
        "yes": True,
    }
    if run_id is not None:
        body["run_id"] = run_id
    return 0, None, body


def cmd_approve(args: argparse.Namespace) -> int:
    code, message, body = prepare_approval(args, fold(args.goal))
    if body is None:
        return _refuse(code, message)
    held: dict[str, Any] = {"code": 0, "message": None}

    def decide(events: list[dict[str, Any]]) -> dict[str, Any] | None:
        # The early fold can be stale. Check every --expect-* again on the
        # lines just read under the log lock, and record that fold's revision
        # and run rather than the ones the first read saw.
        locked_code, locked_message, locked_body = prepare_approval(
            args, fold(args.goal, events)
        )
        if locked_body is None:
            held["code"] = locked_code
            held["message"] = locked_message
            return None
        return locked_body

    written = _locked_update(args.goal, decide)
    if written is None:
        return _refuse(int(held["code"]), held["message"])
    return 0


def cmd_pending(args: argparse.Namespace) -> int:
    """Print the approvals this goal is waiting on. Reads the log and does not write."""
    state = fold(args.goal)
    if state.goal is None:
        print(f"unknown goal {args.goal}", file=sys.stderr)
        return 1
    print(json.dumps(pending_approvals(state), indent=2))
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
    argv = build_run_argv(provider, model, policy, worker_task_text(task))
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


def hydra_show_run(run_id: str) -> dict[str, Any]:
    proc = subprocess.run([hydra_bin(), "show", run_id], capture_output=True, text=True, check=False)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or proc.stdout.strip())
    return json.loads(proc.stdout)


def hunk_argv(run: dict[str, Any]) -> tuple[list[str], Path]:
    """How Hunk opens a run's diff from the repository, never from a run-writable checkout."""
    run_id = run.get("run_id") or ""
    branch = run.get("branch")
    if not branch:
        raise RuntimeError(
            f"run {run_id} has no branch; review opens Hunk on the reaped branch from the "
            "repository, not inside a run-writable checkout. If the diff exists only in an "
            "unreaped worktree, reap first."
        )
    repo = Path(run.get("repo") or os.getcwd())
    return ["hunk", "show", branch], repo


def cmd_review(args: argparse.Namespace) -> int:
    """Launches the review panel, records its verdict, pushes findings into Hunk as comments,
    then opens the human's diff review in Hunk. `--no-open` prints that command and returns
    without opening Hunk. Inside Herdr, without the flag, it goes to a new pane."""
    state = fold(args.goal)
    task = state.tasks.get(args.task)
    if task is None or not task.run_id:
        print(f"task {args.task} has no run", file=sys.stderr)
        return 1
    run = hydra_show_run(task.run_id)
    run.setdefault("run_id", task.run_id)
    if state.goal and not run.get("repo"):
        run["repo"] = state.goal.get("repo")
    try:
        argv, cwd = hunk_argv(run)
    except RuntimeError as err:
        print(str(err), file=sys.stderr)
        return 1
    base_sha = (state.goal or {}).get("base_sha", "")
    branch = run["branch"]
    projection = run.get("result_text") or json.dumps(run, indent=2)
    diff_text = git_diff_branch(cwd, base_sha, branch)
    resolved, substitutions, refusal = resolve_panel_seats(state)
    if refusal:
        print(refusal, file=sys.stderr)
        return 1
    seat_prompts = build_diff_panel_prompts(
        resolved, task.text, base_sha, projection, diff_text
    )
    run_ids, policies, texts = run_judge_panel(seat_prompts)
    outcomes = [
        parse_judge_answer(resolved.provider, text)
        for resolved, text in zip(resolved, texts)
    ]
    for resolved, policy, run_id in zip(resolved, policies, run_ids):
        note = f" for {resolved.missing_provider}" if resolved.substituted else ""
        print(f"judge {resolved.provider}{note} ({policy}) run {run_id}")
    verdict, reasons = diff_panel_verdict(outcomes)
    if verdict == "block":
        for reason in reasons:
            print(reason, file=sys.stderr)
    append_event(
        args.goal,
        {
            "event": "review",
            "task": args.task,
            "judge_run_ids": run_ids,
            "judge_policies": policies,
            "verdict": verdict,
            "reasons": reasons,
            "degraded": bool(substitutions),
            "substitutions": substitutions,
        },
    )
    comments = findings_to_hunk_comments(outcomes, cwd, base_sha, branch)
    if args.no_open:
        fork_hunk_comment_pusher(cwd, comments)
        print(" ".join(argv))
        return 0
    pane_id = os.environ.get("HERDR_PANE_ID")
    if os.environ.get("HERDR_ENV") and pane_id:
        split = subprocess.run(
            ["herdr", "pane", "split", pane_id, "--direction", "right", "--cwd", str(cwd)],
            capture_output=True,
            text=True,
            check=True,
        )
        new_pane = json.loads(split.stdout)["result"]["pane"]["pane_id"]
        subprocess.run(
            ["herdr", "pane", "run", new_pane, " ".join(argv)],
            capture_output=True,
            check=True,
        )
        for _ in range(30):
            if push_hunk_comments(cwd, comments):
                break
            time.sleep(1)
        print(f"{task.run_id} in {new_pane} ({verdict})")
        return 0
    fork_hunk_comment_pusher(cwd, comments)
    os.chdir(cwd)
    os.execvp(argv[0], argv)


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


def cmd_queue(args: argparse.Namespace) -> int:
    """Tasks whose current run has ended and still has no human verdict, highest verdict `p` first.
    The order is a suggestion. The file is only read."""
    events = read_log(args.goal)
    state = fold(args.goal, events)
    if state.goal is None:
        print(f"unknown goal {args.goal}", file=sys.stderr)
        return 1
    scores = load_verdict_scores(grader_home() / "scores.jsonl")
    rows = ordered_review_tasks(state, events, scores)
    missing = sum(1 for task in rows if scores.by_run.get(task.run_id or "") is None)
    version = scores.model_version or "none"
    noun = "task" if missing == 1 else "tasks"
    print(f"verdict {version}  {missing} {noun} with no score")
    print(f"{'task':<12} {'provider':<14} {'model':<16} {'run':<36} {'p':<8} attempt")
    for task in rows:
        assignment = task.assignment or {}
        provider = str(assignment.get("provider") or "")
        model = str(assignment.get("model") or "")
        p_cell = format_p(scores.by_run.get(task.run_id or ""))
        attempt = "" if task.attempt is None else str(task.attempt)
        print(
            f"{task.id:<12} {provider:<14} {model:<16} {task.run_id or '':<36} "
            f"{p_cell:<8} {attempt}"
        )
    return 0


def cmd_repair(args: argparse.Namespace) -> int:
    """Print damage and a torn tail, or with `--yes` move them aside and rewrite the log.

    Without `--yes` nothing is written. With `--yes` bad newline-terminated lines and
    any unterminated tail are written, under the log's lock, to
    `<goal>.torn-<UTC timestamp>.txt` beside the log. The new log, its good lines and
    a `repair` event when one or more bad lines were removed, is then written whole to
    a temporary file beside the log and renamed over it. A repair killed at any point
    leaves the old log or the new one, never the good lines without the event. A log
    with neither damage nor a torn tail is left as it is and exits 0.
    """
    goal_id = args.goal
    path = goal_log_path(goal_id)
    if not path.is_file():
        print(f"goal log {goal_id} has no torn tail")
        return 0
    with _locked_log(path, "rb") as handle:
        raw = handle.read()
        good, bad, tail = _partition_log(raw)
        if not bad and tail is None:
            print(f"goal log {goal_id} has no torn tail")
            return 0
        if not args.yes:
            for line_no, line in bad:
                preview = line.decode("utf-8", errors="replace").strip("\r\n")[:300]
                print(f"bad line {line_no}: {preview}")
            if tail is not None:
                preview = tail.decode("utf-8", errors="replace")[:300]
                print(f"unterminated tail: {preview}")
            print(
                "would move the bad lines and any unterminated tail into "
                f"{goal_id}.torn-<UTC timestamp>.txt beside the log, "
                "then rewrite the log with only its good lines"
            )
            return 0
        _check_temporary(path.with_name(path.name + ".repair-tmp"))
        dest = path.parent / f"{goal_id}.torn-{utc_now()}.txt"
        with dest.open("xb") as torn:
            for _line_no, line in bad:
                torn.write(line)
            if tail is not None:
                torn.write(tail)
            torn.flush()
            os.fsync(torn.fileno())
        new_raw = b"".join(good)
        removed = len(bad)
        if removed:
            record = {
                "seq": len(good) + 1,
                "ts": utc_now(),
                "event": "repair",
                "removed": removed,
            }
            line = json.dumps(record, separators=(",", ":")) + "\n"
            new_raw += line.encode("utf-8")
        _replace_log(path, new_raw, os.fstat(handle.fileno()).st_mode)
        print(f"moved the tail into {dest.name}")
        return 0


def cmd_verdict(args: argparse.Namespace) -> int:
    state = fold(args.goal)
    task = state.tasks.get(args.task)
    if task is None or not task.run_id:
        print(f"task {args.task} has no run", file=sys.stderr)
        return 1
    if (
        args.result in ("applied", "fixed")
        and approval_available(state, "diff", args.task, task.run_id) is None
    ):
        print(
            f"missing diff approval for task {args.task} run {task.run_id} at revision "
            f"{state.revision}; approve with: approve {state.goal_id} --diff {args.task}",
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

    board_p = sub.add_parser("board", help="the goal at a glance")
    board_p.add_argument("goal", help="goal id")
    board_p.set_defaults(func=cmd_board)

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
    choose_p.add_argument(
        "choice",
        nargs="?",
        help="provider name or plan file path (omit with --judge)",
    )
    choose_p.add_argument(
        "--judge",
        action="store_true",
        help="launch the panel to pick among finished proposals",
    )
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
    approve_p.add_argument(
        "--degraded",
        metavar="MISSING=SUBSTITUTE",
        help="approve a degraded panel when a seat's provider is unavailable",
    )
    approve_p.add_argument(
        "--expect-revision",
        type=int,
        default=None,
        help="refuse unless the goal is still at this revision",
    )
    approve_p.add_argument(
        "--expect-run",
        default=None,
        help="with --diff, refuse unless the task's current run is this id",
    )
    approve_p.add_argument(
        "--expect-plan",
        default=None,
        help="with --plan, refuse unless the chosen plan's path is this path",
    )
    approve_p.set_defaults(func=cmd_approve)

    pending_p = sub.add_parser("pending", help="approvals the goal is waiting on, as JSON")
    pending_p.add_argument("goal", help="goal id")
    pending_p.add_argument(
        "--json",
        action="store_true",
        required=True,
        help="print a JSON array of the approvals still waiting",
    )
    pending_p.set_defaults(func=cmd_pending)

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

    review_p = sub.add_parser("review", help="open a task's diff in Hunk")
    review_p.add_argument("goal", help="goal id")
    review_p.add_argument("task", help="task id")
    review_p.add_argument(
        "--no-open",
        action="store_true",
        help="record the review and print the hunk command without opening it",
    )
    review_p.set_defaults(func=cmd_review)

    queue_p = sub.add_parser("queue", help="tasks waiting on a review, highest verdict p first")
    queue_p.add_argument("goal", help="goal id")
    queue_p.set_defaults(func=cmd_queue)

    wait_p = sub.add_parser("wait", help="poll pending runs for completion")
    wait_p.add_argument("goal", help="goal id")
    wait_p.set_defaults(func=cmd_wait)

    verdict_p = sub.add_parser("verdict", help="record human verdict on a diff")
    verdict_p.add_argument("goal", help="goal id")
    verdict_p.add_argument("task", help="task id")
    verdict_p.add_argument("result", choices=sorted(VERDICT_RESULTS))
    verdict_p.add_argument("--note", default="")
    verdict_p.set_defaults(func=cmd_verdict)

    repair_p = sub.add_parser(
        "repair",
        help="save an unterminated tail and truncate the log to its last newline",
    )
    repair_p.add_argument("goal", help="goal id")
    repair_p.add_argument(
        "--yes",
        action="store_true",
        help=(
            "move the tail aside and truncate; without this flag, "
            "print what it would do and write nothing"
        ),
    )
    repair_p.set_defaults(func=cmd_repair)

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except (UnterminatedLog, DamagedLog, UnsafeRepairTemporary) as err:
        print(err, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
