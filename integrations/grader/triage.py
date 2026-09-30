import argparse
import json
import random
from datetime import datetime, timezone
from pathlib import Path

import sys
import importlib.util
HERE = Path(__file__).resolve().parent

_records_spec = importlib.util.spec_from_file_location("grader_records", HERE / "records.py")
records = importlib.util.module_from_spec(_records_spec)
sys.modules[_records_spec.name] = records
_records_spec.loader.exec_module(records)

_monitor_spec = importlib.util.spec_from_file_location("grader_monitor", HERE / "monitor.py")
monitor = importlib.util.module_from_spec(_monitor_spec)
sys.modules[_monitor_spec.name] = monitor
_monitor_spec.loader.exec_module(monitor)

def parse_time(ts_str):
    if not ts_str:
        return None
    try:
        return datetime.fromisoformat(str(ts_str).replace("Z", "+00:00"))
    except (ValueError, TypeError):
        return None

LIMIT_TEXT = "You've hit your"
SIGNAL_NOTE = "the child died on signal "  # src/watch.rs SIGNAL_NOTE
STALL_SECONDS = 600


def cut(line) -> str:
    return json.dumps(line)[:300]


def hydra_of(line) -> dict:
    value = line.get("hydra")
    return value if isinstance(value, dict) else {}


def quota_line(lines):
    """The frame `src/outcome.rs` reads a rate limit from: `api_error_status` 429 on a
    result, or the providers' usage-limit sentence. A `rate_limit_event` frame is not
    one: `claude` sends it on healthy runs too."""
    for line in lines:
        raw = line.get("raw")
        if isinstance(raw, dict) and raw.get("api_error_status") == 429:
            return line
        text = json.dumps(line)
        if LIMIT_TEXT in text and "limit" in text:
            return line
    return None


def last_refusal(lines):
    """The last line on which `monitor.Prefix` counted a refusal, and whether no
    tool call came after it."""
    walk = monitor.Prefix()
    found, last = None, False

    def open_calls():
        return len(walk.pending) + len(walk.anon)

    for line in lines:
        refusals, calls, opened = walk.refusals, walk.tool_calls, open_calls()
        walk.observe(line)
        if walk.refusals > refusals:
            found, last = line, True
        elif walk.tool_calls > calls or open_calls() < opened:
            # A new call, or a call that closed without a refusal, comes after it.
            last = False
    return found, last


def gap_before_end(lines):
    """Seconds between the last line before `run_end` and `run_end` itself, on the
    envelope `ts` every line carries."""
    stamps = [parse_time(line.get("ts")) for line in lines]
    stamps = [s for s in stamps if s is not None]
    if len(stamps) < 2:
        return 0.0
    return (stamps[-1] - stamps[-2]).total_seconds()


def get_cause(row, lines):
    """First rule that matches wins. The order goes from what a source said out loud
    to what is inferred from shape."""
    outcome = row.get("outcome")
    ends = [line for line in lines if hydra_of(line).get("type") == "run_end"]
    end = hydra_of(ends[-1]) if ends else {}
    refusal = end.get("refusal")

    # collision: the refusal Hydra writes, or the derived outcome.
    if refusal == "skill_collision" or outcome == "skill_collision":
        return "collision", cut(ends[-1]) if ends else '{"outcome": "skill_collision"}'

    # quota: the provider stopped serving the run.
    hit = quota_line(lines)
    if hit is not None:
        return "quota", cut(hit)
    if outcome == "rate_limited":
        return "quota", '{"outcome": "rate_limited"}'

    # gate: a pre-launch refusal, the gate refusing everything, or a refusal as
    # the last tool event.
    if refusal:
        return "gate", cut(ends[-1])
    refused, was_last = last_refusal(lines)
    if refused is not None and (outcome == "gated" or was_last):
        return "gate", cut(refused)

    # never_started: no provider frame at all, the shape of build 147.
    if not any(isinstance(line.get("raw"), dict) for line in lines):
        return "never_started", cut(lines[-1]) if lines else "empty record"

    # provider_crash: no `run_end`, a death on a signal the watcher saw, or an
    # exit code of 128 and up (killed by a signal it did not see).
    if not ends:
        return "provider_crash", "no run_end"
    error = end.get("error")
    code = end.get("exit_code")
    if (isinstance(error, str) and error.startswith(SIGNAL_NOTE)) or outcome == "crashed" or (
        isinstance(code, int) and code >= 128
    ):
        return "provider_crash", cut(ends[-1])

    # stall: more than ten minutes of silence right before the end.
    if gap_before_end(lines) > STALL_SECONDS:
        return "stall", cut(ends[-1])

    return "unknown", cut(ends[-1])


def run_triage(home: Path, runs: Path):
    dataset_path = home / "dataset.jsonl"
    out_path = home / "triage.jsonl"
    rows = records.read_jsonl(dataset_path)

    out = []
    for row in rows:
        # A live run has no outcome yet; triage waits for it.
        if row.get("outcome") in ("completed", None):
            continue
        run_id = row["run_id"]
        run_file = runs / f"{run_id}.jsonl"
        lines = []
        if run_file.exists():
            lines, _ = records.committed_lines(run_file.read_bytes())

        cause, evidence = get_cause(row, lines)
        out.append({
            "run_id": run_id,
            "outcome": row.get("outcome"),
            "cause": cause,
            "evidence": evidence,
            "triaged_at": datetime.now(timezone.utc).isoformat()
        })
    records.write_jsonl(out_path, out)

def table(home: Path):
    out_path = home / "triage.jsonl"
    if not out_path.exists():
        return
    rows = records.read_jsonl(out_path)
    dataset = records.read_jsonl(home / "dataset.jsonl")
    provider_map = {r["run_id"]: r.get("provider", "unknown") for r in dataset}

    by_outcome_cause = {}
    by_provider_cause = {}

    for r in rows:
        oc = (r.get("outcome"), r.get("cause"))
        by_outcome_cause[oc] = by_outcome_cause.get(oc, 0) + 1

        provider = provider_map.get(r["run_id"], "unknown")
        pc = (provider, r.get("cause"))
        by_provider_cause[pc] = by_provider_cause.get(pc, 0) + 1

    print("Outcome and cause:")
    for (o, c), count in sorted(by_outcome_cause.items(), key=lambda kv: tuple(map(str, kv[0]))):
        print(f"{o} {c}: {count}")

    print("\nProvider and cause:")
    for (p, c), count in sorted(by_provider_cause.items(), key=lambda kv: tuple(map(str, kv[0]))):
        print(f"{p} {c}: {count}")

def sample(home: Path, n: int, seed: str):
    out_path = home / "triage.jsonl"
    if not out_path.exists():
        return
    rows = records.read_jsonl(out_path)
    random.seed(seed)
    chosen = random.sample(rows, min(n, len(rows)))
    for r in chosen:
        print(f"Run {r['run_id']} - Cause: {r.get('cause')}")
        print(f"Evidence: {r.get('evidence')}\n")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("run")
    sub.add_parser("table")
    s = sub.add_parser("sample")
    s.add_argument("--n", type=int, default=20)
    s.add_argument("--seed", type=str, default="s")

    args = parser.parse_args()

    home = records.grader_home()
    runs = records.hydra_root() / "runs"

    if args.cmd == "run":
        run_triage(home, runs)
    elif args.cmd == "table":
        table(home)
    elif args.cmd == "sample":
        sample(home, args.n, args.seed)
