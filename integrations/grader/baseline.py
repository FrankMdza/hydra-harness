#!/usr/bin/env python3
"""Job 1 of map ticket 77: a logistic baseline over the grader dataset (stdlib only).

Nothing here makes a network call, and `score` reads one record file and nothing else. The
features are the ones a router has before a launch: provider, model, policy and the task's size.
Tokens and duration are left out on purpose, because a run that dies early has none, so they
would hand the model its own answer.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import sys
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

_records_spec = importlib.util.spec_from_file_location("grader_records", HERE / "records.py")
records = importlib.util.module_from_spec(_records_spec)
sys.modules[_records_spec.name] = records
_records_spec.loader.exec_module(records)

PREAMBLE = "Hydra launched this run"
# Below this many rows of the rarer class in any split, a fitted number says more about the
# split than about Hydra, so `train` refuses rather than print it.
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


def fit_temperature(logits: list[float], ys: list[int]) -> float:
    """The T that minimises log loss of sigmoid(z / T), by a log-spaced grid then a finer one."""
    def loss(t: float) -> float:
        return log_loss([sigmoid(z / t) for z in logits], ys)

    grid = [math.exp(k / 20) for k in range(-40, 41)]
    best = min(grid, key=loss)
    fine = [best * math.exp(k / 200) for k in range(-10, 11)]
    return min(fine, key=loss)


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


def split(rows: list[dict], cal_from: datetime, test_from: datetime) -> tuple[list, list, list]:
    """By launch time, so nothing launched later can train what launched earlier."""
    fit_rows = [r for r in rows if launched_at(r["run_id"]) < cal_from]
    cal_rows = [r for r in rows if cal_from <= launched_at(r["run_id"]) < test_from]
    test_rows = [r for r in rows if launched_at(r["run_id"]) >= test_from]
    return fit_rows, cal_rows, test_rows


def train(rows: list[dict], label: str, cal_from: datetime, test_from: datetime) -> dict:
    labelled = [r for r in rows if target(r, label) is not None]
    parts = split(labelled, cal_from, test_from)
    for name, part in zip(("fit", "calibration", "test"), parts):
        ys = [target(r, label) for r in part]
        minority = min(sum(ys), len(ys) - sum(ys))
        if minority < MIN_MINORITY:
            raise SystemExit(
                f"baseline: the {name} split has {len(ys)} rows and {minority} of its rarer class; "
                f"{MIN_MINORITY} is the floor. Label more runs before training on '{label}'."
            )
    fit_rows, cal_rows, test_rows = parts
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
    cal_logits = [logit(weights, bias, vector(r, vocab, scale)) for r in cal_rows]
    temperature = fit_temperature(cal_logits, [target(r, label) for r in cal_rows])
    body = {
        "label": label,
        "positive": "not completed" if label == "outcome" else "fixed",
        "vocab": vocab,
        "scale": scale,
        "weights": weights,
        "bias": bias,
        "temperature": temperature,
        "cal_from": cal_from.isoformat(),
        "test_from": test_from.isoformat(),
        "rows": {"fit": len(fit_rows), "calibration": len(cal_rows), "test": len(test_rows)},
    }
    digest = hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()[:12]
    return dict(body, version=f"baseline-{label}-{digest}")


def predict(model: dict, row: dict, calibrated: bool = True) -> float:
    scale = {k: tuple(v) for k, v in model["scale"].items()}
    z = logit(model["weights"], model["bias"], vector(row, model["vocab"], scale))
    return sigmoid(z / model["temperature"] if calibrated else z)


def report(model: dict, rows: list[dict]) -> dict:
    label = model["label"]
    labelled = [r for r in rows if target(r, label) is not None]
    fit_rows, _, test_rows = split(
        labelled,
        datetime.fromisoformat(model["cal_from"]),
        datetime.fromisoformat(model["test_from"]),
    )
    ys = [target(r, label) for r in test_rows]
    base = sum(target(r, label) for r in fit_rows) / len(fit_rows)
    raw = [predict(model, r, calibrated=False) for r in test_rows]
    cal = [predict(model, r) for r in test_rows]
    constant = [base] * len(ys)

    def measures(ps):
        return {"log_loss": log_loss(ps, ys), "brier": brier(ps, ys), "rank_auc": rank_auc(ps, ys)}

    return {
        "version": model["version"],
        "test_rows": len(ys),
        "test_positive": sum(ys),
        "fit_base_rate": base,
        "temperature": model["temperature"],
        "constant_base_rate": measures(constant),
        "uncalibrated": measures(raw),
        "calibrated": measures(cal),
        "reliability_calibrated": reliability(cal, ys),
    }


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
    tr.add_argument("model", type=Path, help="where the fitted model JSON goes")
    tr.add_argument("--label", choices=("outcome", "verdict"), default="outcome")
    tr.add_argument("--cal-from", type=day, default=day("2026-09-21"))
    tr.add_argument("--test-from", type=day, default=day("2026-09-23"))
    sc = sub.add_parser("score", help="score one run from its record")
    sc.add_argument("model", type=Path)
    sc.add_argument("run", help="a record path, or a full run id under $HYDRA_ROOT/runs/")
    args = parser.parse_args(argv)

    if args.command == "train":
        rows = records.read_jsonl(args.dataset)
        model = train(rows, args.label, args.cal_from, args.test_from)
        records.write_json(args.model, model)
        print(json.dumps(report(model, rows), indent=2))
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
