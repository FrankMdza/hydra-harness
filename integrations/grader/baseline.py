#!/usr/bin/env python3
"""Job 1 of map ticket 77: a logistic baseline over the grader dataset (stdlib only).

Nothing here makes a network call, and `score` reads one record file and nothing else. The
features are the ones a router has before a launch: provider, model, policy and the task's size.
Tokens and duration are left out on purpose, because a run that dies early has none, so they
would hand the model its own answer.

Three labels, three models (build 143, D1): `outcome` says a run will not complete, `verdict` says
its diff will need a correction, `task` says it will fail for a reason in the task, with quota
and gate refusals left out (build 165, `TASK_FAILURES`). Each is trained, stored and scored apart.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import json
import math
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

_records_spec = importlib.util.spec_from_file_location("grader_records", HERE / "records.py")
records = importlib.util.module_from_spec(_records_spec)
sys.modules[_records_spec.name] = records
_records_spec.loader.exec_module(records)

PREAMBLE = "Hydra launched this run"
LABELS = ("outcome", "verdict", "task")
# Outcomes the `task` label counts as the task's own failure. `completed` is 0. Every other
# outcome (`rate_limited`, `gated`, `skill_collision`, `auth_failed`, anything new) says nothing
# about the task and leaves the row out.
TASK_FAILURES = ("failed", "crashed", "timed_out")
POSITIVE = {"outcome": "not completed", "verdict": "fixed", "task": "failed for the task"}
# The human's verdicts from the label queue, run id to `applied` or `fixed`. Collect reads it
# too, so a dataset line rewritten after an import keeps its verdict.
TICKET_VERDICTS = "ticket-verdicts.json"
# Below this many rows of the rarer class in either slice, a fitted number says more about the
# slice than about Hydra, so `train` refuses rather than print it.
MIN_MINORITY = 10
L2 = 1.0
STEPS = 3000
RATE = 0.5


def launched_at(run_id: str) -> datetime:
    """A run id is a UUIDv7, so its first 48 bits are the launch time in milliseconds."""
    millis = int(run_id.replace("-", "")[:12], 16)
    return datetime.fromtimestamp(millis / 1000, timezone.utc)


def target(row: dict, label: str) -> int | None:
    """1 for the outcome the router wants to avoid, None when the row carries no label."""
    if label == "outcome":
        if row.get("outcome") is None:
            return None
        return int(row["outcome"] != "completed")
    if label == "task":
        outcome = row.get("outcome")
        if outcome == "completed":
            return 0
        return 1 if outcome in TASK_FAILURES else None
    if label == "verdict":
        return {"fixed": 1, "applied": 0}.get(row.get("human_verdict"))
    raise ValueError(f"unknown label {label}")


def raw_features(row: dict) -> dict[str, float]:
    text = row.get("task_text") or ""
    return {
        f"provider={row.get('provider')}": 1.0,
        f"model={row.get('model')}": 1.0,
        f"policy={row.get('policy')}": 1.0,
        "log_task_chars": math.log1p(len(text)),
        "preamble": float(PREAMBLE in text),
    }


def vocabulary(rows: list[dict]) -> list[str]:
    return sorted({name for row in rows for name in raw_features(row)})


def vector(row: dict, vocab: list[str], scale: dict[str, tuple[float, float]]) -> list[float]:
    feats = raw_features(row)
    out = []
    for name in vocab:
        value = feats.get(name, 0.0)
        if name in scale:
            mean, std = scale[name]
            value = (value - mean) / std
        out.append(value)
    return out


def sigmoid(z: float) -> float:
    if z >= 0:
        return 1.0 / (1.0 + math.exp(-z))
    e = math.exp(z)
    return e / (1.0 + e)


def logit(weights: list[float], bias: float, x: list[float]) -> float:
    return bias + sum(w * v for w, v in zip(weights, x))


def fit(xs: list[list[float]], ys: list[int]) -> tuple[list[float], float]:
    """Full-batch gradient descent on L2-penalised log loss. Deterministic: no shuffling."""
    n, d = len(xs), len(xs[0])
    weights, bias = [0.0] * d, 0.0
    for _ in range(STEPS):
        grad_w, grad_b = [0.0] * d, 0.0
        for x, y in zip(xs, ys):
            err = sigmoid(logit(weights, bias, x)) - y
            grad_b += err
            for j, v in enumerate(x):
                if v:
                    grad_w[j] += err * v
        bias -= RATE * grad_b / n
        weights = [w - RATE * (g / n + L2 * w / n) for w, g in zip(weights, grad_w)]
    return weights, bias


def log_loss(ps: list[float], ys: list[int]) -> float:
    eps = 1e-12
    return -sum(
        y * math.log(max(p, eps)) + (1 - y) * math.log(max(1 - p, eps)) for p, y in zip(ps, ys)
    ) / len(ys)


def brier(ps: list[float], ys: list[int]) -> float:
    return sum((p - y) ** 2 for p, y in zip(ps, ys)) / len(ys)


def rank_auc(ps: list[float], ys: list[int]) -> float | None:
    pos = [p for p, y in zip(ps, ys) if y]
    neg = [p for p, y in zip(ps, ys) if not y]
    if not pos or not neg:
        return None
    wins = sum(1.0 if a > b else 0.5 if a == b else 0.0 for a in pos for b in neg)
    return wins / (len(pos) * len(neg))


def reliability(ps: list[float], ys: list[int], bins: int = 5) -> list[dict]:
    table = []
    for k in range(bins):
        lo, hi = k / bins, (k + 1) / bins
        members = [(p, y) for p, y in zip(ps, ys) if lo <= p < hi or (k == bins - 1 and p == 1.0)]
        table.append({
            "bin": f"{lo:.1f}-{hi:.1f}",
            "n": len(members),
            "mean_p": sum(p for p, _ in members) / len(members) if members else None,
            "observed": sum(y for _, y in members) / len(members) if members else None,
        })
    return table


def split(rows: list[dict], test_from: datetime) -> tuple[list, list]:
    """By launch time, so nothing launched later can train what launched earlier. Two slices,
    fit and test: a calibration slice spent scarce rows and its temperature moved test Brier the
    wrong way (build 143, D2)."""
    fit_rows = [r for r in rows if launched_at(r["run_id"]) < test_from]
    test_rows = [r for r in rows if launched_at(r["run_id"]) >= test_from]
    return fit_rows, test_rows


def train(rows: list[dict], label: str, test_from: datetime) -> dict:
    labelled = [r for r in rows if target(r, label) is not None]
    parts = split(labelled, test_from)
    for name, part in zip(("fit", "test"), parts):
        ys = [target(r, label) for r in part]
        minority = min(sum(ys), len(ys) - sum(ys))
        if minority < MIN_MINORITY:
            raise SystemExit(
                f"baseline: {len(labelled)} labelled rows for '{label}'; the {name} slice has "
                f"{len(ys)} of them and {minority} of its rarer class, and {MIN_MINORITY} is the "
                f"floor. Label more runs before training on '{label}'."
            )
    fit_rows, test_rows = parts
    vocab = vocabulary(fit_rows)
    scale = {}
    for name in ("log_task_chars",):
        values = [raw_features(r)[name] for r in fit_rows]
        mean = sum(values) / len(values)
        std = math.sqrt(sum((v - mean) ** 2 for v in values) / len(values)) or 1.0
        scale[name] = (mean, std)
    xs = [vector(r, vocab, scale) for r in fit_rows]
    ys = [target(r, label) for r in fit_rows]
    weights, bias = fit(xs, ys)
    body = {
        "label": label,
        "positive": POSITIVE[label],
        "vocab": vocab,
        "scale": scale,
        "weights": weights,
        "bias": bias,
        "test_from": test_from.isoformat(),
        "rows": {"fit": len(fit_rows), "test": len(test_rows)},
    }
    digest = hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()[:12]
    return dict(body, version=f"baseline-{label}-{digest}")


def predict(model: dict, row: dict) -> float:
    scale = {k: tuple(v) for k, v in model["scale"].items()}
    return sigmoid(logit(model["weights"], model["bias"], vector(row, model["vocab"], scale)))


# How many rows of weight the parent rate gets when a pair's or a provider's own rate is
# estimated. Without it a pair that never failed in the fit slice is priced at exactly 0, pays
# about 27.6 log loss on its first failure in test, and any model beats the bar without merit.
SHRINK = 5.0


def provider_model_rates(fit_rows: list[dict], label: str):
    """The fit slice's positive rate for a row's (provider, model), shrunk toward its provider's,
    which is shrunk toward the whole slice's. This is the bar of build 143, D5: most positives are
    rate limits, so a model that learned only which provider runs out of quota should not beat
    it."""
    pairs: dict[tuple, list[int]] = {}
    providers: dict[str, list[int]] = {}
    for row in fit_rows:
        y = target(row, label)
        pairs.setdefault((row.get("provider"), row.get("model")), []).append(y)
        providers.setdefault(row.get("provider"), []).append(y)
    overall_ys = [target(r, label) for r in fit_rows]
    overall = sum(overall_ys) / len(overall_ys)

    def shrunk(ys: list[int], prior: float) -> float:
        return (sum(ys) + SHRINK * prior) / (len(ys) + SHRINK)

    def rate(row: dict) -> float:
        provider = shrunk(providers.get(row.get("provider"), []), overall)
        return shrunk(pairs.get((row.get("provider"), row.get("model")), []), provider)

    return rate


def report(model: dict, rows: list[dict]) -> dict:
    label = model["label"]
    labelled = [r for r in rows if target(r, label) is not None]
    fit_rows, test_rows = split(labelled, datetime.fromisoformat(model["test_from"]))
    ys = [target(r, label) for r in test_rows]
    base = sum(target(r, label) for r in fit_rows) / len(fit_rows)
    scored = [predict(model, r) for r in test_rows]
    rate = provider_model_rates(fit_rows, label)

    def measures(ps):
        return {"log_loss": log_loss(ps, ys), "brier": brier(ps, ys), "rank_auc": rank_auc(ps, ys)}

    ours = measures(scored)
    bar = measures([rate(r) for r in test_rows])
    return {
        "version": model["version"],
        "label": label,
        "test_rows": len(ys),
        "test_positive": sum(ys),
        "fit_base_rate": base,
        "constant_base_rate": measures([base] * len(ys)),
        "provider_model_base_rate": bar,
        "model": ours,
        "beats_provider_model_base": {m: ours[m] < bar[m] for m in ("log_loss", "brier")},
        "reliability": reliability(scored, ys),
    }


def model_path(label: str, home: Path | None = None) -> Path:
    """One model file per label under the grader home, which is where collect looks."""
    if label not in LABELS:
        raise ValueError(f"unknown label {label}")
    return (home or records.grader_home()) / "models" / f"{label}.json"


QUEUE_FIELDS = ["ticket", "run_id", "provider", "model", "outcome", "verdict"]
VERDICTS = ("applied", "fixed", "unknown")
# A run id as a ticket spells it: the first eight hex digits of the UUIDv7, sometimes more.
RUN_ID = re.compile(r"\b[0-9a-f]{8}(?:-[0-9a-f]{4}){0,3}(?:-[0-9a-f]{12})?\b")


def jev_probe():
    """The Jev probe's ticket reader, loaded on first use so collect never imports it."""
    spec = importlib.util.spec_from_file_location(
        "grader_jev_probe", HERE.parent / "puppets" / "jev_probe.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def outcome_section(body: str) -> str | None:
    """The text under `## Outcome` up to the next `## ` heading, cut where jev_probe cuts it."""
    marker = "## Outcome"
    start = body.find(marker)
    if start < 0:
        return None
    rest = body[start + len(marker):]
    end = rest.find("\n## ")
    return (rest if end < 0 else rest[:end]).strip()


def label_queue(tickets: Path, dataset: list[dict]) -> tuple[list[dict], list[str]]:
    """One row per dataset run named in the `## Outcome` of a resolved build ticket, with the
    verdict left empty for the human. A name that matches no dataset run, or several, becomes a
    skip line instead of a row, so a commit sha or an unknown run never gets labelled."""
    probe = jev_probe()
    queue, skips = [], []
    for path in sorted(tickets.glob("*.md")):
        number = probe.ticket_number(path)
        if number is None:
            continue
        body = path.read_text(encoding="utf-8")
        if probe.ticket_status(body) != "resolved":
            continue
        outcome = outcome_section(body)
        if not outcome:
            continue
        seen = set()
        for name in RUN_ID.findall(outcome):
            matches = [r for r in dataset if r["run_id"].startswith(name)]
            if len(matches) != 1:
                skips.append(f"{number}: {name} names {len(matches)} dataset runs")
                continue
            row = matches[0]
            if row["run_id"] in seen:
                continue
            seen.add(row["run_id"])
            queue.append({
                "ticket": number,
                "run_id": row["run_id"],
                "provider": row.get("provider"),
                "model": row.get("model"),
                "outcome": outcome[:400],
                "verdict": "",
            })
    return queue, skips


def label_import(csv_path: Path, dataset_path: Path, home: Path) -> tuple[int, int]:
    """Fold a filled-in queue into `ticket-verdicts.json` under the grader home and into the
    dataset's `human_verdict`. `unknown` and an empty cell leave the run unlabelled; any other
    word refuses the whole file. Returns how many runs were labelled and how many left unknown."""
    with csv_path.open(newline="", encoding="utf-8") as handle:
        filled = list(csv.DictReader(handle))
    verdicts: dict[str, str] = {}
    unknown = 0
    for row in filled:
        verdict = (row.get("verdict") or "").strip().lower()
        if verdict and verdict not in VERDICTS:
            raise SystemExit(
                f"baseline: run {row.get('run_id')} has verdict '{verdict}'; "
                f"use one of {', '.join(VERDICTS)} or leave it empty"
            )
        if verdict in ("", "unknown"):
            unknown += 1
            continue
        verdicts[row["run_id"]] = verdict
    kept_path = home / TICKET_VERDICTS
    kept = records.read_json(kept_path, {})
    kept.update(verdicts)
    records.write_json(kept_path, kept)
    rows = records.read_jsonl(dataset_path)
    for row in rows:
        if row["run_id"] in kept:
            row["human_verdict"] = kept[row["run_id"]]
    records.write_jsonl(dataset_path, rows)
    return len(verdicts), unknown


def row_from_record(path: Path) -> dict:
    """The pre-launch fields of one record, spelled the way a dataset row spells them."""
    item = records.scan_record(path)
    if item is None:
        raise SystemExit(f"baseline: {path} is not a run or resume record")
    first = item["first"]
    start = first.get("hydra") or {}
    _, model = records.canonical(first["provider"], first.get("model"), records.load_aliases())
    return {
        "run_id": first["run_id"],
        "provider": first["provider"],
        "model": model,
        "policy": start.get("policy"),
        "task_text": start.get("task") or "",
    }


def record_path(target_arg: str) -> Path:
    path = Path(target_arg)
    if path.is_file():
        return path
    return records.hydra_root() / "runs" / f"{target_arg}.jsonl"


def day(text: str) -> datetime:
    return datetime.fromisoformat(text).replace(tzinfo=timezone.utc)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    tr = sub.add_parser("train", help="fit on a dataset and print the test report")
    tr.add_argument("dataset", type=Path)
    tr.add_argument("model", type=Path, nargs="?",
                    help="where the fitted model JSON goes (default: "
                         "$HYDRA_GRADER_HOME/models/LABEL.json)")
    tr.add_argument("--label", choices=LABELS, default="outcome")
    tr.add_argument("--test-from", type=day, default=day("2026-09-23"))
    sc = sub.add_parser("score", help="score one run from its record")
    sc.add_argument("model", type=Path)
    sc.add_argument("run", help="a record path, or a full run id under $HYDRA_ROOT/runs/")
    lq = sub.add_parser("label-queue", help="write the CSV of runs for a human to label")
    lq.add_argument("out", type=Path)
    lq.add_argument("--tickets", type=Path, default=HERE.parents[1] / ".scratch/hydra/build")
    lq.add_argument("--dataset", type=Path, help="default: $HYDRA_GRADER_HOME/dataset.jsonl")
    li = sub.add_parser("label-import", help="fold a filled-in label queue into the dataset")
    li.add_argument("csv", type=Path)
    li.add_argument("--home", type=Path, help="default: $HYDRA_GRADER_HOME")
    li.add_argument("--dataset", type=Path, help="default: HOME/dataset.jsonl")
    args = parser.parse_args(argv)

    if args.command == "train":
        rows = records.read_jsonl(args.dataset)
        model = train(rows, args.label, args.test_from)
        records.write_json(args.model or model_path(args.label), model)
        print(json.dumps(report(model, rows), indent=2))
        return 0
    if args.command == "label-queue":
        dataset = records.read_jsonl(args.dataset or records.grader_home() / "dataset.jsonl")
        queue, skips = label_queue(args.tickets, dataset)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.DictWriter(handle, fieldnames=QUEUE_FIELDS)
            writer.writeheader()
            writer.writerows(queue)
        print(f"rows: {len(queue)}")
        for line in skips:
            print(f"skip: {line}")
        return 0
    if args.command == "label-import":
        home = args.home or records.grader_home()
        labelled, unknown = label_import(args.csv, args.dataset or home / "dataset.jsonl", home)
        print(f"labelled: {labelled}")
        print(f"unknown: {unknown}")
        return 0
    model = records.read_json(args.model)
    row = row_from_record(record_path(args.run))
    print(json.dumps({
        "run_id": row["run_id"],
        "label": model["label"],
        "positive": model["positive"],
        "p": predict(model, row),
        "model_version": model["version"],
    }))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
