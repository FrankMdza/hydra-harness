"""Walk Hydra run records for grading (stdlib only)."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from collections.abc import Iterator
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from pathlib import Path
import subprocess
import sys
import tempfile

PUPPETS = Path(__file__).resolve().parents[1] / "puppets"
_models_spec = importlib.util.spec_from_file_location("grader_models", PUPPETS / "models.py")
_models = importlib.util.module_from_spec(_models_spec)
_models_spec.loader.exec_module(_models)
canonical = _models.canonical
load_aliases = _models.load_aliases


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


def read_json(path: Path, default=None):
    return json.loads(path.read_text()) if path.exists() else default


def current_preamble() -> str | None:
    path = hydra_root() / "config" / "preamble.md"
    if path.is_file():
        return path.read_text(encoding="utf-8").rstrip("\n") + "\n\n"
    return None


def strip_preamble(task_text: str, current: str | None) -> tuple[str, bool]:
    if current and task_text.startswith(current):
        return task_text[len(current):], True

    first_line = "Hydra launched this run and wrote its instructions at spawn."
    marker = "say so and stop.\n\n"
    if task_text.startswith(first_line) and marker in task_text:
        idx = task_text.find(marker) + len(marker)
        return task_text[idx:], True

    return task_text, False


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temp.replace(path)


def read_jsonl(path: Path) -> list[dict]:
    if not path.is_file():
        return []
    return committed_lines(path.read_bytes())[0]


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + ".tmp")
    with temp.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, allow_nan=False) + "\n")
    temp.replace(path)


def committed_lines(data: bytes) -> tuple[list[dict], int]:
    """The parsed complete lines and how many bytes they cover. A torn tail is left out; a
    complete line that does not parse raises, and the caller decides what to do with the
    record."""
    lines = []
    covered = 0
    for line in data.splitlines(keepends=True):
        if not line.endswith(b"\n"):
            break
        lines.append(json.loads(line))
        covered += len(line)
    return lines, covered


def duration_seconds(first: dict, lines: list[dict]) -> float | None:
    end = next(
        (row["hydra"] for row in reversed(lines) if row.get("hydra", {}).get("type") == "run_end"),
        {},
    )
    if not end.get("ended_at"):
        return None
    try:
        started = datetime.fromisoformat(str(first.get("ts")).replace("Z", "+00:00"))
        ended = datetime.fromisoformat(str(end["ended_at"]).replace("Z", "+00:00"))
    except (ValueError, TypeError):
        return None
    return (ended - started).total_seconds()


def refresh_show_answer(
    run_id: str,
    runs: Path,
    hydra: str,
    cache: dict,
    fingerprint: str,
) -> dict:
    env = dict(os.environ, HYDRA_ROOT=str(runs.resolve().parent))
    with tempfile.TemporaryDirectory() as tmp:
        if runs.name != "runs":
            Path(tmp, "runs").symlink_to(runs.resolve(), target_is_directory=True)
            env["HYDRA_ROOT"] = tmp
        result = subprocess.run(
            [hydra, "show", run_id, "--events"],
            env=env,
            capture_output=True,
            text=True,
            check=True,
        )
    full = json.loads(result.stdout)
    projection = full.get("projection") or {}
    # Only what a dataset row reads. The first version cached the whole answer, every event
    # of every record unredacted, which put a raw copy of the corpus beside the redacted
    # dataset (the security review of goal ea69da1f).
    answer = {
        "projection": {
            "outcome": projection.get("outcome"),
            "usage": {"counts": (projection.get("usage") or {}).get("counts") or {}},
        }
    }
    cache[run_id] = {"fingerprint": fingerprint, "answer": answer}
    return answer


def record_row(
    first: dict,
    lines: list[dict],
    answer: dict,
    aliases: dict,
) -> dict:
    start = first.get("hydra", {})
    projection = answer.get("projection") or {}
    usage = projection.get("usage") or {}
    recorded_model = first.get("model")
    canon_provider, canon_model = canonical(first["provider"], recorded_model, aliases)
    return {
        "run_id": first["run_id"],
        "provider": first["provider"],
        "recorded_model": recorded_model,
        "canonical_provider": canon_provider,
        "canonical_model": canon_model,
        "policy": start.get("policy"),
        "task": start.get("task"),
        "outcome": projection.get("outcome"),
        "tokens": usage.get("counts") or {},
        "duration_seconds": duration_seconds(first, lines),
    }


def scan_record(path: Path) -> dict | None:
    """One record, or None when it is not a provider run or cannot be read. A garbled line
    skips this record and names it on stderr rather than ending the pass for every other one."""
    data = path.read_bytes()
    try:
        lines, covered = committed_lines(data)
    except (json.JSONDecodeError, UnicodeDecodeError) as err:
        print(f"grader: skipping {path.name}: {err}", file=sys.stderr)
        return None
    if not lines:
        return None
    first = lines[0]
    if not isinstance(first, dict):
        print(f"grader: skipping {path.name}: line 0 is not an object", file=sys.stderr)
        return None
    start = first.get("hydra") or {}
    if not first.get("provider") or not first.get("run_id") or start.get("command") not in ("run", "resume"):
        return None
    return {
        "path": path,
        "data": data,
        "lines": lines,
        "first": first,
        "run_id": first["run_id"],
        "fingerprint": hashlib.sha256(data[:covered]).hexdigest(),
    }


def iter_records(
    runs: Path | None = None,
    home: Path | None = None,
    hydra: str | None = None,
    aliases: dict | None = None,
    jobs: int = 1,
) -> Iterator[dict]:
    """Yield one grading row per run or resume record under ``runs``."""
    runs = runs or (hydra_root() / "runs")
    home = home or grader_home()
    hydra = hydra or os.environ.get("HYDRA_BIN", "hydra")
    aliases = load_aliases() if aliases is None else aliases
    cache_path = home / "show-cache.json"
    cache = read_json(cache_path, {})
    pending = []
    for path in sorted(runs.glob("*.jsonl")):
        item = scan_record(path)
        if item is not None:
            pending.append(item)

    stale = [
        item
        for item in pending
        if cache.get(item["run_id"], {}).get("fingerprint") != item["fingerprint"]
    ]
    if stale:
        workers = max(1, jobs)

        def work(item: dict) -> None:
            refresh_show_answer(
                item["run_id"],
                runs,
                hydra,
                cache,
                item["fingerprint"],
            )

        if workers == 1:
            for item in stale:
                work(item)
        else:
            with ThreadPoolExecutor(max_workers=workers) as pool:
                for future in [pool.submit(work, item) for item in stale]:
                    future.result()
        write_json(cache_path, cache)

    for item in pending:
        answer = cache[item["run_id"]]["answer"]
        yield record_row(item["first"], item["lines"], answer, aliases)
