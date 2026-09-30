#!/usr/bin/env python3
"""Collect a grading dataset from Hydra run records (stdlib only)."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
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


_baseline = None


def baseline():
    """The classifier, loaded on the first pass that has something to score."""
    global _baseline
    if _baseline is None:
        spec = importlib.util.spec_from_file_location("grader_baseline", HERE / "baseline.py")
        _baseline = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_baseline)
    return _baseline


def score_rows(home: Path, rows: list[dict], scored_at: str) -> int:
    """Score each row with every model under `home/models/` and append one line per run and
    model version to `home/scores.jsonl`. Nothing reads these yet (build 143, D6), so the model
    does not choose which runs get reviewed. Returns how many lines were appended."""
    paths = sorted((home / "models").glob("*.json"))
    if not paths or not rows:
        return 0
    scores_path = home / "scores.jsonl"
    done = {(s["run_id"], s["model_version"]) for s in records.read_jsonl(scores_path)}
    fresh = []
    for path in paths:
        model = records.read_json(path)
        for row in rows:
            if (row["run_id"], model["version"]) in done:
                continue
            fresh.append({
                "run_id": row["run_id"],
                "label": model["label"],
                "p": baseline().predict(model, row),
                "model_version": model["version"],
                "scored_at": scored_at,
            })
    if fresh:
        with scores_path.open("a", encoding="utf-8") as handle:
            handle.write("".join(json.dumps(s, allow_nan=False) + "\n" for s in fresh))
    return len(fresh)


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def dataset_by_run_id(path: Path) -> dict[str, dict]:
    return {row["run_id"]: row for row in records.read_jsonl(path)}


def label_digest(label: dict) -> str:
    return hashlib.sha256(json.dumps(label, sort_keys=True).encode()).hexdigest()[:16]


def dataset_row(
    raw: dict, label: dict | None, collected_at: str, llm_label: dict | None = None
) -> dict:
    task_text = raw.get("task") or ""
    if label and label.get("text"):
        task_text = label["text"]
    # Puppets names the run: it wins. Otherwise a class a model wrote fills the row, marked "llm".
    llm = None if label else llm_label
    row = {
        "run_id": raw["run_id"],
        "provider": raw["provider"],
        "model": raw["canonical_model"],
        "recorded_model": raw["recorded_model"],
        "policy": raw["policy"],
        "task_text": redact.redact_string(task_text),
        "task_class": label.get("class") if label else (llm or {}).get("task_class"),
        "grade": label.get("grade") if label else (llm or {}).get("grade"),
        "area": (llm or {}).get("area"),
        "kind": (llm or {}).get("kind"),
        "outcome": raw["outcome"],
        "human_verdict": label.get("human_verdict") if label else None,
        "tokens": raw["tokens"],
        "duration_s": raw["duration_seconds"],
        "labels_source": "puppets" if label else ("llm" if llm else None),
        "collected_at": collected_at,
    }
    if llm and llm.get("unsure"):
        row["label_unsure"] = True
    return row


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
    hydra = hydra or os.environ.get("HYDRA_BIN", "hydra")
    cursor_path = home / "cursor.json"
    dataset_path = home / "dataset.jsonl"

    cursor = {} if rebuild else records.read_json(cursor_path, {})
    dataset = {} if rebuild else dataset_by_run_id(dataset_path)
    puppets_labels = labels.collect()
    llm_labels = labels.collect_llm(home)

    def effective(item: dict) -> str:
        """The record's fingerprint, plus a digest of the model-written label when one fills
        the row, so a new or changed class file makes the run stale."""
        run_id = item["run_id"]
        if run_id in puppets_labels or run_id not in llm_labels:
            return item["fingerprint"]
        return item["fingerprint"] + ":llm-" + label_digest(llm_labels[run_id])

    scanned = []
    for path in sorted(runs.glob("*.jsonl")):
        item = records.scan_record(path)
        if item is not None:
            scanned.append(item)

    stale = [
        item
        for item in scanned
        if rebuild or cursor.get(item["run_id"]) != effective(item)
    ]
    stale_ids = {item["run_id"] for item in stale}
    if not stale:
        return 0

    aliases = records.load_aliases()
    cache_path = home / "show-cache.json"
    cache = records.read_json(cache_path, {})
    workers = max(1, jobs)

    def work(item: dict) -> None:
        if cache.get(item["run_id"], {}).get("fingerprint") == item["fingerprint"]:
            return  # only the label changed
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

    # Verdicts the human gave through `baseline.py label-import`, kept so a rewritten line
    # does not lose one.
    ticket_verdicts = records.read_json(home / "ticket-verdicts.json", {})
    collected_at = utc_now()
    for item in scanned:
        cursor[item["run_id"]] = effective(item)
        if rebuild or item["run_id"] in stale_ids:
            answer = cache[item["run_id"]]["answer"]
            raw = records.record_row(item["first"], item["lines"], answer, aliases)
            row = dataset_row(
                raw,
                puppets_labels.get(item["run_id"]),
                collected_at,
                llm_labels.get(item["run_id"]),
            )
            if item["run_id"] in ticket_verdicts:
                row["human_verdict"] = ticket_verdicts[item["run_id"]]
            dataset[item["run_id"]] = row

    rows = [dataset[run_id] for run_id in sorted(dataset)]
    records.write_jsonl(dataset_path, rows)
    records.write_json(cursor_path, cursor)
    score_rows(home, [dataset[run_id] for run_id in sorted(stale_ids)], collected_at)
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

    sub.add_parser("backfill", help="score every row already in the dataset")

    args = parser.parse_args(argv)
    jobs = getattr(args, "jobs", 4)

    if args.command == "backfill":
        home = records.grader_home()
        print(score_rows(home, records.read_jsonl(home / "dataset.jsonl"), utc_now()))
        return 0

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
