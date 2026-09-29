#!/usr/bin/env python3
"""A local model labels the dataset's runs, and `agree` measures it against the human (stdlib only).

The only network call goes to `HYDRA_LOCAL_LLM_URL`, an Ollama on this machine. Nothing reads
the labels yet (build 146)."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load(name: str, file: str):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


records = _load("grader_records", "records.py")
redact = _load("grader_redact", "redact.py")

DEFAULT_MODEL = "gemma3:1b-it-qat"
DEFAULT_URL = "http://127.0.0.1:11434/api/generate"
TASK_BUDGET = 3000
RESULT_BUDGET = 4000
UNPARSED_BUDGET = 500
NUM_PREDICT = 200
TIMEOUT_SECONDS = 300
CLASSES = ("quota", "gate", "agent", "task", "none")
RULE_CLASS = {"rate_limited": "quota", "gated": "gate"}
LOOSE_BELOW = 50

INSTRUCTIONS = """You are checking the work of a coding agent. Below are the task it was given, \
the text it ended with, and how much it changed.

First write one sentence saying what the agent did and whether it did what the task asked. Then \
give the verdict.

Answer with one JSON object and nothing else, with the keys in this order:
{"reason": "<one sentence>", "result_ok": true or false}

result_ok is true when the result does what the task asked and a human could use it as it is. \
result_ok is false when a human would have to fix it first."""


class Unavailable(Exception):
    """Ollama did not answer, or does not have the model."""


def ollama_url() -> str:
    return os.environ.get("HYDRA_LOCAL_LLM_URL", DEFAULT_URL)


def ollama_model() -> str:
    return os.environ.get("HYDRA_LOCAL_MODEL", DEFAULT_MODEL)


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def cut(text: str, budget: int) -> str:
    if len(text) <= budget:
        return text
    return text[:budget] + f"\n[cut: {len(text) - budget} more characters]"


def show_projection(run_id: str, home_runs: Path, hydra: str) -> dict:
    """The run's `hydra show` projection, or an empty dict when the run cannot be shown.
    Read fresh and never cached: the grader's cache leaves `result_text` out on purpose."""
    env = dict(os.environ, HYDRA_ROOT=str(home_runs.resolve().parent))
    try:
        with tempfile.TemporaryDirectory() as tmp:
            if home_runs.name != "runs":
                Path(tmp, "runs").symlink_to(home_runs.resolve(), target_is_directory=True)
                env["HYDRA_ROOT"] = tmp
            result = subprocess.run(
                [hydra, "show", run_id], env=env, capture_output=True, text=True, check=True
            )
        answer = json.loads(result.stdout)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError):
        return {}
    return answer.get("projection") or answer


def build_prompt(row: dict, projection: dict) -> str:
    task = cut(redact.redact_string(row.get("task_text") or ""), TASK_BUDGET)
    result = cut(redact.redact_string(projection.get("result_text") or "(none recorded)"), RESULT_BUDGET)
    stat = projection.get("diff_stat")
    stat_text = redact.redact_string(json.dumps(stat, sort_keys=True)) if stat else "(none recorded)"
    return (
        f"{INSTRUCTIONS}\n\nTASK:\n{task}\n\nRESULT TEXT:\n{result}\n\nDIFF STAT:\n{stat_text}\n"
    )


def generate(prompt: str) -> str:
    """The `response` text of one Ollama call. Raises Unavailable when Ollama is down or the
    model is not pulled."""
    model = ollama_model()
    body = json.dumps({
        "model": model,
        "prompt": prompt,
        "stream": False,
        "format": "json",
        "options": {"temperature": 0, "num_predict": NUM_PREDICT},
    }).encode()
    request = urllib.request.Request(
        ollama_url(), data=body, headers={"Content-Type": "application/json"}, method="POST"
    )
    try:
        with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as reply:
            raw = reply.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as err:
        err.close()
        if err.code == 404:
            raise Unavailable(f"Ollama does not have the model {model}") from err
        raise Unavailable(f"Ollama at {ollama_url()} answered HTTP {err.code}") from err
    except (urllib.error.URLError, OSError) as err:
        raise Unavailable(f"Ollama is not answering at {ollama_url()}") from err
    try:
        return json.loads(raw)["response"]
    except (json.JSONDecodeError, KeyError, TypeError):
        return raw


def parse_answer(text: str) -> dict | None:
    try:
        answer = json.loads(text)
    except (json.JSONDecodeError, TypeError):
        return None
    if not isinstance(answer, dict):
        return None
    # The prompt asks for the reason first and for `result_ok`, the positive of what the file
    # stores. Asked for `needs_correction` and then a reason, the 1B model answered true and
    # wrote "no correction needed" in 9 of 18 runs; asked this way, 0 of 19 (2026-09-29).
    if isinstance(answer.get("result_ok"), bool) and isinstance(answer.get("reason"), str):
        return {"needs_correction": not answer["result_ok"], "failure_class": None,
                "reason": answer["reason"]}
    need, klass, reason = (answer.get(k) for k in ("needs_correction", "failure_class", "reason"))
    if not isinstance(need, bool) or klass not in CLASSES or not isinstance(reason, str):
        return None
    return {"needs_correction": need, "failure_class": klass, "reason": reason}


def label_line(run_id: str, model: str, prompt_sha: str | None, answer: dict | None, raw: str = "") -> dict:
    line = {
        "run_id": run_id,
        "needs_correction": answer["needs_correction"] if answer else None,
        "failure_class": answer["failure_class"] if answer else None,
        "reason": answer["reason"] if answer else None,
        "judge_model": model,
        "prompt_sha256": prompt_sha,
        "judged_at": utc_now(),
    }
    if answer is None and model != "rule":
        line["unparsed"] = raw[:UNPARSED_BUDGET]
    return line


def label(limit: int | None, only_unlabelled: bool) -> int:
    home = records.grader_home()
    labels_path = home / "judge-labels.jsonl"
    rows = records.read_jsonl(home / "dataset.jsonl")
    done = {
        (l["run_id"], l["judge_model"], l["prompt_sha256"]) for l in records.read_jsonl(labels_path)
    }
    runs = records.hydra_root() / "runs"
    hydra = os.environ.get("HYDRA_BIN", "hydra")
    model = ollama_model()
    written = unparsed = skipped = 0
    labels_path.parent.mkdir(parents=True, exist_ok=True)

    def summary() -> str:
        return f"labelled {written}, unparsed {unparsed}, already labelled {skipped}"

    for row in rows:
        if only_unlabelled and row.get("human_verdict") is not None:
            continue
        if limit is not None and written >= limit:
            break
        run_id = row["run_id"]
        if row.get("outcome") in RULE_CLASS:
            if (run_id, "rule", None) in done:
                skipped += 1
                continue
            line = {
                "run_id": run_id,
                "needs_correction": None,
                "failure_class": RULE_CLASS[row["outcome"]],
                "reason": f"outcome {row['outcome']} is not a failure of the agent",
                "judge_model": "rule",
                "prompt_sha256": None,
                "judged_at": utc_now(),
            }
        else:
            prompt = build_prompt(row, show_projection(run_id, runs, hydra))
            sha = hashlib.sha256(prompt.encode()).hexdigest()
            if (run_id, model, sha) in done:
                skipped += 1
                continue
            try:
                raw = generate(prompt)
            except Unavailable as err:
                print(f"judge: {err}, so the pass stopped and kept the {written} labels it had written.", file=sys.stderr)
                return 1
            answer = parse_answer(raw)
            unparsed += answer is None
            line = label_line(run_id, model, sha, answer, raw)
        with labels_path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(line, allow_nan=False) + "\n")
        written += 1
    print(summary())
    return 0


def agree() -> int:
    home = records.grader_home()
    dataset = {r["run_id"]: r for r in records.read_jsonl(home / "dataset.jsonl")}
    judged = {}
    for line in records.read_jsonl(home / "judge-labels.jsonl"):
        if line.get("judge_model") != "rule":
            judged[line["run_id"]] = line
    unparsed = sum(1 for l in judged.values() if l.get("needs_correction") is None)
    table = {"fixed": [0, 0], "applied": [0, 0]}
    for run_id, line in judged.items():
        human = (dataset.get(run_id) or {}).get("human_verdict")
        need = line.get("needs_correction")
        if human in table and isinstance(need, bool):
            table[human][0 if need else 1] += 1
    both = sum(sum(cells) for cells in table.values())
    agreed = table["fixed"][0] + table["applied"][1]
    print(f"runs with both: {both}")
    print(f"agree: {agreed}")
    if both:
        print(f"agreement: {agreed}/{both} = {agreed / both:.2f}")
    else:
        print("agreement: none, no run has both labels")
    if both < LOOSE_BELOW:
        print(f"This number is loose: fewer than {LOOSE_BELOW} runs have both labels.")
    for name in ("fixed", "applied"):
        yes, no = table[name]
        print(f"human {name:<7}  judge true {yes}    judge false {no}")
    print(f"unparsed answers: {unparsed}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Label Hydra runs with a local model.")
    sub = parser.add_subparsers(dest="command", required=True)
    lab = sub.add_parser("label", help="ask the local model about each run in the dataset")
    lab.add_argument("--limit", type=int, help="write at most this many new labels")
    lab.add_argument("--only-unlabelled", action="store_true", help="skip runs with a human verdict")
    sub.add_parser("agree", help="compare the judge's labels with the human's verdicts")
    args = parser.parse_args(argv)
    if args.command == "label":
        return label(args.limit, args.only_unlabelled)
    return agree()


if __name__ == "__main__":
    sys.exit(main())
