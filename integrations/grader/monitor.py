#!/usr/bin/env python3
"""A logistic monitor over a prefix of a run record (stdlib only).

`baseline.py` predicts from what is known before launch. This predicts from that
plus the first fraction of the record's own lines. Nothing here stops a run.
Stopping on the score is a decision for the map, and this command does not make it.

The fractions are 0.1, 0.25 and 0.5. A prefix is the first
`max(1, floor(fraction * lines))` lines, so a record of one line is read whole
and a record of ten lines gives 1, 2 and 5 lines.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import sys
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent

_baseline_spec = importlib.util.spec_from_file_location("grader_baseline", HERE / "baseline.py")
baseline = importlib.util.module_from_spec(_baseline_spec)
sys.modules[_baseline_spec.name] = baseline
_baseline_spec.loader.exec_module(baseline)

FRACTIONS = (0.1, 0.25, 0.5)
# Counts that vary in size. One-hot names and `seat_requested` stay as 0/1, the
# same way `baseline.py` leaves provider and policy unscaled and scales only
# `log_task_chars`.
SCALE_NAMES = (
    "log_task_chars",
    "events_so_far",
    "tool_calls_so_far",
    "refusals_so_far",
    "distinct_tools",
    "output_tokens_so_far",
    "seconds_elapsed",
)
# `cursor-agent` names a call by the key on `tool_call`. The spellings are the
# ones `src/normalize/cursor.rs` maps; an unlisted `*ToolCall` key keeps its stem.
CURSOR_TOOLS = {
    "readToolCall": "Read",
    "shellToolCall": "Shell",
    "editToolCall": "Write",
    "deleteToolCall": "Delete",
    "grepToolCall": "Grep",
    "globToolCall": "Grep",
    "lsToolCall": "List",
    "webSearchToolCall": "WebSearch",
    "webFetchToolCall": "Fetch",
}
CODEX_TOOLS = {
    "command_execution": "Bash",
    "web_search": "WebSearch",
    "apply_patch": "ApplyPatch",
    "file_change": "ApplyPatch",
}


def fraction_token(fraction: float) -> str:
    return format(fraction, "g")


def parse_fraction(text: str) -> float:
    try:
        value = float(text)
    except ValueError as err:
        raise argparse.ArgumentTypeError(
            f"at must be one of {', '.join(fraction_token(f) for f in FRACTIONS)}"
        ) from err
    for allowed in FRACTIONS:
        if math.isclose(value, allowed, rel_tol=0, abs_tol=1e-9):
            return allowed
    raise argparse.ArgumentTypeError(
        f"at must be one of {', '.join(fraction_token(f) for f in FRACTIONS)}"
    )


def prefix_len(n: int, fraction: float) -> int:
    if n <= 0:
        return 0
    return min(n, max(1, math.floor(fraction * n)))


def is_denial(text: str) -> bool:
    """Hydra's own refusal voice. `hydra/allow` is the gate allowing a call."""
    if "hydra/" not in text:
        return False
    return "hydra/" in text.replace("hydra/allow", "")


def parse_ts(line: dict) -> datetime | None:
    ts = line.get("ts")
    if not isinstance(ts, str):
        return None
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00"))
    except ValueError:
        return None


def output_tokens(raw: dict) -> int | None:
    """The latest usage report on this frame, or None when it carries none.

    Providers park the object in different places: on the frame (`claude`'s
    `result`, `codex`'s `turn.completed`), under `result`, under `step_update`
    (`agy`) and under `message` (a harvested `claude` transcript). The last one
    on the frame wins. A running total and a per-step count are not added.
    """
    found = None
    containers = [raw.get("usage")]
    for key in ("result", "step_update", "message"):
        inner = raw.get(key)
        if isinstance(inner, dict):
            containers.append(inner.get("usage"))
    for container in containers:
        if not isinstance(container, dict):
            continue
        value = container.get("output_tokens", container.get("outputTokens"))
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            found = int(value)
    return found


def cursor_name(tool_call: dict) -> str | None:
    for key, name in CURSOR_TOOLS.items():
        if key in tool_call:
            return name
    for key in tool_call:
        if key.endswith("ToolCall"):
            return key[: -len("ToolCall")]
    return None


def cursor_args(tool_call: dict):
    for key, value in tool_call.items():
        if key.endswith("ToolCall") and isinstance(value, dict):
            args = value.get("args")
            # The call id rides inside the args and differs on every call, so a repeat
            # would never match on it.
            if isinstance(args, dict):
                args = {k: v for k, v in args.items() if k != "toolCallId"}
            return args
    return None


def cursor_denied(tool_call: dict) -> bool:
    for value in tool_call.values():
        if not isinstance(value, dict):
            continue
        result = value.get("result")
        if isinstance(result, str) and is_denial(result):
            return True
        if not isinstance(result, dict):
            continue
        for part in ("error", "failure"):
            if part in result and is_denial(json.dumps(result[part])):
                return True
    return False


def hook_denial(raw: dict) -> str | None:
    """The denied tool, or `""` when the hook denied and named no tool.

    None means this frame is not a denial. A following `tool_result` repeats
    the same denial in prose; the hook is the one that counts.
    """
    if raw.get("subtype") != "hook_response":
        return None
    output = raw.get("output")
    parsed = output if isinstance(output, dict) else None
    if isinstance(output, str):
        try:
            parsed = json.loads(output)
        except json.JSONDecodeError:
            return None
    if not isinstance(parsed, dict):
        return None
    specific = parsed.get("hookSpecificOutput") or parsed
    if not isinstance(specific, dict) or specific.get("permissionDecision") != "deny":
        return None
    hook_name = raw.get("hook_name") or ""
    if ":" in hook_name:
        return hook_name.split(":", 1)[1]
    return ""


def call_key(name: str, args) -> str:
    """The tool name plus a hash of its input. No input keys on the name alone."""
    if args is None:
        return name
    text = json.dumps(args, sort_keys=True, default=str)
    return f"{name}:{hashlib.sha256(text.encode()).hexdigest()[:16]}"


def claude_calls(raw: dict) -> list[tuple[str, str | None, object]]:
    message = raw.get("message")
    if not isinstance(message, dict):
        return []
    content = message.get("content")
    if not isinstance(content, list):
        return []
    found = []
    for block in content:
        if isinstance(block, dict) and block.get("type") == "tool_use" and block.get("name"):
            call_id = block.get("id")
            found.append((
                block["name"],
                call_id if isinstance(call_id, str) else None,
                block.get("input"),
            ))
    return found


def claude_results(raw: dict) -> list[tuple[str | None, bool]]:
    message = raw.get("message")
    if not isinstance(message, dict):
        return []
    content = message.get("content")
    if not isinstance(content, list):
        return []
    found = []
    for block in content:
        if not isinstance(block, dict) or block.get("type") != "tool_result":
            continue
        text = block.get("content")
        if isinstance(text, list):
            text = " ".join(
                part.get("text", "") for part in text if isinstance(part, dict)
            )
        denied = bool(block.get("is_error")) and isinstance(text, str) and is_denial(text)
        call_id = block.get("tool_use_id")
        found.append((call_id if isinstance(call_id, str) else None, denied))
    return found


class Prefix:
    def __init__(self) -> None:
        self.tool_calls = 0
        self.refusals = 0
        self.first_refused: str | None = None
        self.tools: set[str] = set()
        self.output_tokens: int | None = None
        self.seat = False
        self.pending: dict[str, str] = {}
        self.anon: list[str] = []
        self.opened: set[str] = set()
        self.seen_keys: set[str] = set()
        self.repeated = 0
        self.last_key: str | None = None
        self.run = 0
        self.longest_run = 0

    def call(self, name: str, call_id: str | None, args=None) -> None:
        self.tool_calls += 1
        self.tools.add(name)
        key = call_key(name, args)
        if key in self.seen_keys:
            self.repeated += 1
        self.seen_keys.add(key)
        self.run = self.run + 1 if key == self.last_key else 1
        self.last_key = key
        self.longest_run = max(self.longest_run, self.run)
        if call_id:
            self.pending[call_id] = name
            self.opened.add(call_id)
        else:
            self.anon.append(name)

    def refuse(self, name: str | None) -> None:
        if self.refusals == 0:
            self.first_refused = name or None
        self.refusals += 1

    def finish(self, call_id: str | None, denied: bool, name: str | None = None) -> None:
        """Close one open call. A denial counts once, when the call is still open
        or when the frame names a tool the prefix never saw opened."""
        resolved = None
        if call_id is not None and call_id in self.pending:
            resolved = self.pending.pop(call_id)
        elif name:
            # The last open call of this name. A hook names the tool, not the
            # call id, and it belongs to the call that just happened.
            match = None
            for cid, cname in self.pending.items():
                if cname == name:
                    match = cid
            if match is not None:
                resolved = self.pending.pop(match)
            elif name in self.anon:
                for index in range(len(self.anon) - 1, -1, -1):
                    if self.anon[index] == name:
                        resolved = self.anon.pop(index)
                        break
        if denied and (resolved is not None or name):
            self.refuse(resolved or name)
        elif denied and call_id is None and not name:
            self.refuse(None)

    def observe(self, line: dict) -> None:
        hydra = line.get("hydra")
        if isinstance(hydra, dict) and hydra.get("type") == "seat_request":
            self.seat = True
        raw = line.get("raw")
        if not isinstance(raw, dict):
            return
        tokens = output_tokens(raw)
        if tokens is not None:
            self.output_tokens = tokens
        self._calls(raw)

    def _calls(self, raw: dict) -> None:
        if raw.get("type") == "tool_call":
            self._cursor(raw)
            return
        if raw.get("type") == "item.completed":
            self._codex(raw)
            return
        if raw.get("event") == "step_update":
            self._agy(raw)
            return
        for name, call_id, args in claude_calls(raw):
            self.call(name, call_id, args)
        denied = hook_denial(raw)
        if denied is not None:
            self.finish(None, True, denied or None)
        for call_id, is_denied in claude_results(raw):
            # The hook already closed this call. The result echoes it.
            if call_id is not None and call_id not in self.pending:
                continue
            self.finish(call_id, is_denied)

    def _cursor(self, raw: dict) -> None:
        tool_call = raw.get("tool_call") if isinstance(raw.get("tool_call"), dict) else {}
        call_id = raw.get("call_id") or tool_call.get("toolCallId")
        call_id = call_id if isinstance(call_id, str) else None
        name = cursor_name(tool_call)
        if raw.get("subtype") == "started":
            if name and (call_id is None or call_id not in self.opened):
                self.call(name, call_id, cursor_args(tool_call))
            return
        if raw.get("subtype") != "completed":
            return
        if name and call_id is not None and call_id not in self.opened:
            self.call(name, call_id, cursor_args(tool_call))
        elif name and call_id is None and not self.anon and not self.pending:
            self.call(name, None, cursor_args(tool_call))
        self.finish(call_id, cursor_denied(tool_call), name)

    def _codex(self, raw: dict) -> None:
        item = raw.get("item")
        if not isinstance(item, dict):
            return
        kind = item.get("type")
        name = CODEX_TOOLS.get(kind)
        if kind == "mcp_tool_call":
            name = item.get("name") if isinstance(item.get("name"), str) else "mcp"
        if not name:
            return
        call_id = item.get("id")
        call_id = call_id if isinstance(call_id, str) else None
        output = item.get("aggregated_output")
        if not isinstance(output, str):
            output = item.get("output") if isinstance(item.get("output"), str) else ""
        args = item.get("command") if kind == "command_execution" else None
        self.call(name, call_id, args)
        self.finish(call_id, is_denial(output), name)

    def _agy(self, raw: dict) -> None:
        step = raw.get("step_update")
        if not isinstance(step, dict) or step.get("step_type") != "tool":
            return
        info = step.get("tool_info") if isinstance(step.get("tool_info"), dict) else {}
        name = step.get("tool_name") or info.get("name") or "tool"
        if not isinstance(name, str):
            name = "tool"
        index = step.get("step_index")
        call_id = f"agy:{index}"
        if call_id not in self.opened:
            self.call(name, call_id, info.get("parameters"))
        if step.get("state") == "ACTIVE":
            return
        error = info.get("error") if isinstance(info.get("error"), dict) else {}
        message = error.get("message") if isinstance(error.get("message"), str) else ""
        denied = step.get("state") == "ERROR" and is_denial(message)
        self.finish(call_id, denied, name)


def summarize(lines: list, fraction: float) -> dict:
    prefix = lines[: prefix_len(len(lines), fraction)]
    walk = Prefix()
    for line in prefix:
        if isinstance(line, dict):
            walk.observe(line)
    stamps = [parsed for line in prefix if isinstance(line, dict) and (parsed := parse_ts(line))]
    elapsed = 0.0
    if len(stamps) >= 2:
        elapsed = max(0.0, (stamps[-1] - stamps[0]).total_seconds())
    return {
        "events_so_far": len(prefix),
        "tool_calls_so_far": walk.tool_calls,
        "refusals_so_far": walk.refusals,
        "first_refused_tool": walk.first_refused,
        "distinct_tools": len(walk.tools),
        "repeated_calls_so_far": walk.repeated,
        "longest_repeat_run": walk.longest_run,
        "output_tokens_so_far": walk.output_tokens,
        "seconds_elapsed": elapsed,
        "seat_requested": walk.seat,
    }


def raw_features(row: dict) -> dict[str, float]:
    """`baseline.raw_features` plus the prefix counts. The first refused tool is
    one-hot, the same shape as `provider=`."""
    feats = dict(baseline.raw_features(row))
    prefix = row.get("_prefix") or {}
    tokens = prefix.get("output_tokens_so_far")
    feats.update({
        "events_so_far": float(prefix.get("events_so_far") or 0),
        "tool_calls_so_far": float(prefix.get("tool_calls_so_far") or 0),
        "refusals_so_far": float(prefix.get("refusals_so_far") or 0),
        "distinct_tools": float(prefix.get("distinct_tools") or 0),
        "output_tokens_so_far": float(tokens or 0),
        "seconds_elapsed": float(prefix.get("seconds_elapsed") or 0),
        "seat_requested": 1.0 if prefix.get("seat_requested") else 0.0,
    })
    tool = prefix.get("first_refused_tool")
    if tool:
        feats[f"first_refused={tool}"] = 1.0
    return feats


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


def load_record(path: Path) -> dict:
    item = baseline.records.scan_record(path)
    if item is None:
        raise SystemExit(f"monitor: {path} is not a run or resume record")
    return item


def feature_view(path: Path, fraction: float) -> dict:
    item = load_record(path)
    prefix = summarize(item["lines"], fraction)
    # Pre-launch fields come from `baseline.row_from_record`, which is the
    # reader `score` uses, so the printed vector and a later score agree.
    features = {
        "events_so_far": prefix["events_so_far"],
        "tool_calls_so_far": prefix["tool_calls_so_far"],
        "refusals_so_far": prefix["refusals_so_far"],
        "first_refused_tool": prefix["first_refused_tool"],
        "distinct_tools": prefix["distinct_tools"],
        "repeated_calls_so_far": prefix["repeated_calls_so_far"],
        "longest_repeat_run": prefix["longest_repeat_run"],
        "output_tokens_so_far": prefix["output_tokens_so_far"],
        "seconds_elapsed": prefix["seconds_elapsed"],
        "seat_requested": prefix["seat_requested"],
    }
    features.update(baseline.raw_features(baseline.row_from_record(path)))
    return {
        "run_id": item["run_id"],
        "at": fraction,
        "events": len(item["lines"]),
        "events_read": prefix["events_so_far"],
        "features": features,
    }


def join(rows: list[dict], records_dir: Path, fraction: float) -> list[dict]:
    """Dataset rows that have a readable record, with `_prefix` attached.

    A row with no file, or a file that is not a run, is left out. The label
    stays on the dataset row: the record supplies the prefix only.
    """
    joined = []
    for row in rows:
        path = records_dir / f"{row['run_id']}.jsonl"
        if not path.is_file():
            continue
        item = baseline.records.scan_record(path)
        if item is None:
            continue
        joined.append(dict(row, _prefix=summarize(item["lines"], fraction)))
    return joined


def scale_of(rows: list[dict]) -> dict[str, tuple[float, float]]:
    scale = {}
    for name in SCALE_NAMES:
        values = [raw_features(row).get(name, 0.0) for row in rows]
        mean = sum(values) / len(values)
        std = math.sqrt(sum((value - mean) ** 2 for value in values) / len(values)) or 1.0
        scale[name] = (mean, std)
    return scale


def train(rows: list[dict], label: str, test_from: datetime, fraction: float) -> dict:
    """`rows` are already joined to their prefixes. The split, the floor and the
    fit are `baseline.py`'s."""
    labelled = [row for row in rows if baseline.target(row, label) is not None]
    parts = baseline.split(labelled, test_from)
    for name, part in zip(("fit", "test"), parts):
        ys = [baseline.target(row, label) for row in part]
        minority = min(sum(ys), len(ys) - sum(ys))
        if minority < baseline.MIN_MINORITY:
            raise SystemExit(
                f"monitor: {len(labelled)} labelled rows for '{label}' had a record; the "
                f"{name} slice has {len(ys)} of them and {minority} of its rarer class, and "
                f"{baseline.MIN_MINORITY} is the floor. Label more runs before training on "
                f"'{label}'."
            )
    fit_rows, test_rows = parts
    vocab = vocabulary(fit_rows)
    scale = scale_of(fit_rows)
    xs = [vector(row, vocab, scale) for row in fit_rows]
    ys = [baseline.target(row, label) for row in fit_rows]
    weights, bias = baseline.fit(xs, ys)
    body = {
        "label": label,
        "positive": baseline.POSITIVE[label],
        "at": fraction,
        "vocab": vocab,
        "scale": scale,
        "weights": weights,
        "bias": bias,
        "test_from": test_from.isoformat(),
        "rows": {"fit": len(fit_rows), "test": len(test_rows)},
    }
    digest = hashlib.sha256(json.dumps(body, sort_keys=True).encode()).hexdigest()[:12]
    return dict(body, version=f"monitor-{label}-{fraction_token(fraction)}-{digest}")


def predict(model: dict, row: dict) -> float:
    scale = {key: tuple(value) for key, value in model["scale"].items()}
    return baseline.sigmoid(baseline.logit(
        model["weights"], model["bias"], vector(row, model["vocab"], scale)
    ))


def report(model: dict, rows: list[dict]) -> dict:
    label = model["label"]
    labelled = [row for row in rows if baseline.target(row, label) is not None]
    fit_rows, test_rows = baseline.split(labelled, datetime.fromisoformat(model["test_from"]))
    ys = [baseline.target(row, label) for row in test_rows]
    base = sum(baseline.target(row, label) for row in fit_rows) / len(fit_rows)
    scored = [predict(model, row) for row in test_rows]
    rate = baseline.provider_model_rates(fit_rows, label)

    def measures(ps):
        return {
            "log_loss": baseline.log_loss(ps, ys),
            "brier": baseline.brier(ps, ys),
            "rank_auc": baseline.rank_auc(ps, ys),
        }

    ours = measures(scored)
    bar = measures([rate(row) for row in test_rows])
    return {
        "version": model["version"],
        "label": label,
        "test_rows": len(ys),
        "test_positive": sum(ys),
        "fit_base_rate": base,
        "constant_base_rate": measures([base] * len(ys)),
        "provider_model_base_rate": bar,
        "model": ours,
        "beats_provider_model_base": {name: ours[name] < bar[name] for name in ("log_loss", "brier")},
        "reliability": baseline.reliability(scored, ys),
    }


def model_path(label: str, fraction: float, home: Path | None = None) -> Path:
    if label not in baseline.LABELS:
        raise ValueError(f"unknown label {label}")
    name = f"monitor-{label}-{fraction_token(fraction)}.json"
    # A folder of its own: `grader.py` scores every `models/*.json` as a pre-launch model.
    return (home or baseline.records.grader_home()) / "models" / "monitor" / name


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    fe = sub.add_parser("features", help="print the prefix feature vector of one record")
    fe.add_argument("record", help="a record path, or a full run id under $HYDRA_ROOT/runs/")
    fe.add_argument("--at", type=parse_fraction, required=True)
    tr = sub.add_parser("train", help="fit on dataset rows joined to their records")
    tr.add_argument("dataset", type=Path)
    tr.add_argument("model", type=Path, nargs="?",
                    help="where the fitted model JSON goes (default: "
                         "$HYDRA_GRADER_HOME/models/monitor/monitor-LABEL-FRACTION.json)")
    tr.add_argument("--label", choices=baseline.LABELS, default="outcome")
    tr.add_argument("--at", type=parse_fraction, required=True)
    tr.add_argument("--test-from", type=baseline.day, default=baseline.day("2026-09-23"))
    tr.add_argument("--records", type=Path,
                    help="directory of <run_id>.jsonl files (default: $HYDRA_ROOT/runs)")
    sc = sub.add_parser("score", help="score one record at the fraction stored on the model")
    sc.add_argument("model", type=Path)
    sc.add_argument("run", help="a record path, or a full run id under $HYDRA_ROOT/runs/")
    args = parser.parse_args(argv)

    if args.command == "features":
        print(json.dumps(feature_view(baseline.record_path(args.record), args.at), indent=2))
        return 0
    if args.command == "train":
        records_dir = args.records or (baseline.records.hydra_root() / "runs")
        rows = join(baseline.records.read_jsonl(args.dataset), records_dir, args.at)
        model = train(rows, args.label, args.test_from, args.at)
        baseline.records.write_json(args.model or model_path(args.label, args.at), model)
        print(json.dumps(report(model, rows), indent=2))
        return 0
    model = baseline.records.read_json(args.model)
    fraction = model.get("at")
    if not isinstance(fraction, (int, float)) or not any(
        math.isclose(float(fraction), allowed, rel_tol=0, abs_tol=1e-9) for allowed in FRACTIONS
    ):
        raise SystemExit(f"monitor: {args.model} has no prefix fraction")
    fraction = next(
        allowed for allowed in FRACTIONS
        if math.isclose(float(fraction), allowed, rel_tol=0, abs_tol=1e-9)
    )
    path = baseline.record_path(args.run)
    item = load_record(path)
    prefix = summarize(item["lines"], fraction)
    row = dict(baseline.row_from_record(path), _prefix=prefix)
    print(json.dumps({
        "run_id": row["run_id"],
        "label": model["label"],
        "positive": model["positive"],
        "p": predict(model, row),
        "model_version": model["version"],
        "at": fraction,
        "events_read": prefix["events_so_far"],
    }))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
