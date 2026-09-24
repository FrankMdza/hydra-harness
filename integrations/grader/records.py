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


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temp.replace(path)


def read_jsonl(path: Path) -> list[dict]:
    if not path.is_file():
        return []
    return committed_lines(path.read_bytes())


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + ".tmp")
    with temp.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, allow_nan=False) + "\n")
    temp.replace(path)


def committed_lines(data: bytes) -> list[dict]:
    lines = []
    for line in data.splitlines(keepends=True):
        if not line.endswith(b"\n"):
            break
        lines.append(json.loads(line))
    return lines


def duration_seconds(first: dict, lines: list[dict]) -> float | None:
    end = next(
        (row["hydra"] for row in reversed(lines) if row.get("hydra", {}).get("type") == "run_end"),
        {},
    )
    if not end.get("ended_at"):
        return None
    started = datetime.fromisoformat(first["ts"].replace("Z", "+00:00"))
    ended = datetime.fromisoformat(end["ended_at"].replace("Z", "+00:00"))
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
    answer = json.loads(result.stdout)
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
    data = path.read_bytes()
    lines = committed_lines(data)
    if not lines:
        return None
    first = lines[0]
    start = first.get("hydra", {})
    if not first.get("provider") or start.get("command") not in ("run", "resume"):
        return None
    return {
        "path": path,
        "data": data,
        "lines": lines,
        "first": first,
        "run_id": first["run_id"],
        "fingerprint": hashlib.sha256(data).hexdigest(),
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
