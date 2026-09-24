#!/usr/bin/env python3
"""Collect a grading dataset from Hydra run records (stdlib only)."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

_records_spec = importlib.util.spec_from_file_location("grader_records", HERE / "records.py")
records = importlib.util.module_from_spec(_records_spec)
sys.modules[_records_spec.name] = records
_records_spec.loader.exec_module(records)

_labels_spec = importlib.util.spec_from_file_location("grader_labels", HERE / "labels.py")
labels = importlib.util.module_from_spec(_labels_spec)
sys.modules[_labels_spec.name] = labels
_labels_spec.loader.exec_module(labels)

_redact_spec = importlib.util.spec_from_file_location("grader_redact", HERE / "redact.py")
redact = importlib.util.module_from_spec(_redact_spec)
sys.modules[_redact_spec.name] = redact
_redact_spec.loader.exec_module(redact)


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def dataset_by_run_id(path: Path) -> dict[str, dict]:
    return {row["run_id"]: row for row in records.read_jsonl(path)}


def dataset_row(raw: dict, label: dict | None, collected_at: str) -> dict:
    task_text = raw.get("task") or ""
    if label and label.get("text"):
        task_text = label["text"]
    return {
        "run_id": raw["run_id"],
        "provider": raw["provider"],
        "model": raw["canonical_model"],
        "recorded_model": raw["recorded_model"],
        "policy": raw["policy"],
        "task_text": redact.redact_string(task_text),
        "task_class": label.get("class") if label else None,
        "grade": label.get("grade") if label else None,
        "outcome": raw["outcome"],
        "human_verdict": label.get("human_verdict") if label else None,
        "tokens": raw["tokens"],
        "duration_s": raw["duration_seconds"],
        "labels_source": "puppets" if label else None,
        "collected_at": collected_at,
    }


def collect(
    runs: Path | None = None,
    home: Path | None = None,
    hydra: str | None = None,
    jobs: int = 4,
    rebuild: bool = False,
) -> int:
    """Refresh stale rows and return how many were written."""
    runs = runs or (records.hydra_root() / "runs")
    home = home or records.grader_home()
    hydra = hydra or __import__("os").environ.get("HYDRA_BIN", "hydra")
    cursor_path = home / "cursor.json"
    dataset_path = home / "dataset.jsonl"

    cursor = {} if rebuild else records.read_json(cursor_path, {})
    dataset = {} if rebuild else dataset_by_run_id(dataset_path)
    puppets_labels = labels.collect()

    scanned = []
    for path in sorted(runs.glob("*.jsonl")):
        item = records.scan_record(path)
        if item is not None:
            scanned.append(item)

    stale = [
        item
        for item in scanned
        if rebuild or cursor.get(item["run_id"]) != item["fingerprint"]
    ]
    stale_ids = {item["run_id"] for item in stale}
    if not stale:
        return 0

    aliases = records.load_aliases()
    cache_path = home / "show-cache.json"
    cache = records.read_json(cache_path, {})
    workers = max(1, jobs)

    def work(item: dict) -> None:
        records.refresh_show_answer(
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
        from concurrent.futures import ThreadPoolExecutor

        with ThreadPoolExecutor(max_workers=workers) as pool:
            for future in [pool.submit(work, item) for item in stale]:
                future.result()
    records.write_json(cache_path, cache)

    collected_at = utc_now()
    for item in scanned:
        cursor[item["run_id"]] = item["fingerprint"]
        if rebuild or item["run_id"] in stale_ids:
            answer = cache[item["run_id"]]["answer"]
            raw = records.record_row(item["first"], item["lines"], answer, aliases)
            dataset[item["run_id"]] = dataset_row(
                raw,
                puppets_labels.get(item["run_id"]),
                collected_at,
            )

    rows = [dataset[run_id] for run_id in sorted(dataset)]
    records.write_jsonl(dataset_path, rows)
    records.write_json(cursor_path, cursor)
    return len(stale)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Collect a Hydra grading dataset.")
    sub = parser.add_subparsers(dest="command", required=True)

    once = sub.add_parser("once", help="collect everything new since the cursor")
    once.add_argument("--jobs", type=int, default=4, help="parallel hydra show workers")

    watch = sub.add_parser("watch", help="loop once with a sleep between passes")
    watch.add_argument("--interval", type=int, required=True, help="seconds between passes")
    watch.add_argument("--jobs", type=int, default=4, help="parallel hydra show workers")

    rebuild = sub.add_parser("rebuild", help="drop the cursor and rewrite the dataset")
    rebuild.add_argument("--jobs", type=int, default=4, help="parallel hydra show workers")

    args = parser.parse_args(argv)
    jobs = getattr(args, "jobs", 4)

    if args.command == "once":
        print(collect(jobs=jobs))
        return 0

    if args.command == "rebuild":
        print(collect(jobs=jobs, rebuild=True))
        return 0

    # A pass that fails (a `hydra show` that errors, a record mid-write) must not end the
    # watch: report it on stderr and try again after the interval.
    while True:
        try:
            print(collect(jobs=jobs), flush=True)
        except Exception as err:  # noqa: BLE001
            print(f"grader: pass failed: {err}", file=sys.stderr, flush=True)
        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
