#!/usr/bin/env python3
"""The apprentice: a bank of tasks about run records, the local model's attempts, and the pairs
(stdlib only).

The only network call goes to `HYDRA_LOCAL_LLM_URL`, an Ollama on this machine. Nothing trains on
the pairs yet (build 160)."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


# The record scan and the redactor the grader's Python had. Build ticket 220 moved the two
# files here when it deleted the rest, since this file and `integrations/otel/` still load them.
records = _load("apprentice_records", HERE / "records.py")
redact = _load("apprentice_redact", HERE / "redact.py")

DEFAULT_MODEL = "gemma3:1b-it-qat"
DEFAULT_URL = "http://127.0.0.1:11434/api/generate"
SEED = "apprentice-1"
EXCERPT_BUDGET = 3000
NUM_PREDICT = 300
TIMEOUT_SECONDS = 300
clock = time.monotonic

INSTRUCTIONS = """You answer questions about records of coding-agent runs. Below is one task.

First write your reasoning in one or two sentences. Then give the answer. Answer with one JSON \
object and nothing else, with the keys in this order:
{"reason": "<one or two sentences>", "answer": "<your answer>"}

When the task wants a number, the answer is the number alone. When the task lists the possible \
answers, the answer is one of them, spelled exactly. When the task wants a command or a piece of \
writing, the answer is that text."""


class Unavailable(Exception):
    """Ollama did not answer, or does not have the model."""


def utc_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def home() -> Path:
    return records.grader_home() / "apprentice"


def cut(text: str, budget: int) -> str:
    if len(text) <= budget:
        return text
    return text[:budget] + f"\n[cut: {len(text) - budget} more characters]"


def safe(text: str) -> str:
    return cut(redact.redact_string(text), EXCERPT_BUDGET)


# ---------------------------------------------------------------- facts about one record


def list_records(runs: Path) -> list[dict]:
    items = []
    for path in sorted(runs.glob("*.jsonl")):
        item = records.scan_record(path)
        if item is not None:
            items.append(item)
    return items


def _blocks(line: dict) -> list[dict]:
    raw = line.get("raw")
    message = raw.get("message") if isinstance(raw, dict) else None
    content = message.get("content") if isinstance(message, dict) else None
    return [b for b in content if isinstance(b, dict)] if isinstance(content, list) else []


def _text_of(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(b.get("text", "") for b in content if isinstance(b, dict))
    return ""


def facts(item: dict) -> dict:
    """What the templates ask about, read from the record's own lines. Refusals and the last
    assistant text are read from `claude`'s frame shape only; other providers get none, so no
    task is asked about them."""
    first, lines = item["first"], item["lines"]
    start = first.get("hydra") or {}
    end = next((l["hydra"] for l in reversed(lines)
                if (l.get("hydra") or {}).get("type") == "run_end"), None)
    if end is None:
        outcome = "unfinished"
    elif end.get("refusal"):
        outcome = "refused"
    elif end.get("exit_code") == 0:
        outcome = "success"
    else:
        outcome = "failed"
    model = first.get("model")
    if not model:
        model = next((l["raw"]["model"] for l in lines[1:6]
                      if isinstance(l.get("raw"), dict) and isinstance(l["raw"].get("model"), str)), None)
    tools, refused, final_text = {}, [], None
    if first.get("provider") == "claude":
        for line in lines:
            for block in _blocks(line):
                if block.get("type") == "tool_use":
                    tools[block.get("id")] = block.get("name")
                elif block.get("type") == "tool_result" and block.get("is_error"):
                    text = _text_of(block.get("content"))
                    if text.startswith("hydra/"):
                        refused.append({"tool": tools.get(block.get("tool_use_id")), "text": text})
                elif block.get("type") == "text" and (line.get("raw") or {}).get("type") == "assistant":
                    final_text = block.get("text")
    seat_requests = [l["hydra"] for l in lines if (l.get("hydra") or {}).get("type") == "seat_request"]
    launched = 0
    for req in seat_requests:
        if req.get("decision") == "launched":
            runs = req.get("runs")
            launched += len(runs) if isinstance(runs, list) else int(req.get("seats") or 0)
    task = start.get("task") or ""
    task, _ = records.strip_preamble(task, records.current_preamble())
    return {
        "run_id": item["run_id"],
        "provider": first["provider"],
        "model": model,
        "policy": start.get("policy"),
        "interactive": start.get("launch_mode") == "interactive",
        "outcome": outcome,
        "task": task.strip(),
        "start": {k: start.get(k) for k in ("command", "policy", "launch_mode", "task")},
        "end": {k: end.get(k) for k in ("exit_code", "error", "refusal")} if end else None,
        "refused": refused,
        "final_text": final_text,
        "seat_requests": seat_requests,
        "seats_launched": launched,
    }


def _dump(value) -> str:
    return json.dumps(value, sort_keys=True, ensure_ascii=False)


def _start_line(f: dict) -> str:
    shown = dict(f["start"])
    shown["task"] = None if not shown.get("task") else "(shown elsewhere)"
    return _dump({"run_start": shown})


# Each template: name, class, check, applies(f), question, excerpt(f), answer(f).
# `one_of` answers are lists of accepted spellings.
VERIFIABLE = [
    {
        "name": "outcome", "check": "exact",
        "applies": lambda f: True,
        "question": "How did this run end? Answer with one word: success (exit code 0, no refusal), "
                    "failed (a nonzero exit code), refused (Hydra refused it) or unfinished (no "
                    "run_end line).",
        "excerpt": lambda f: _start_line(f) + "\n" + _dump({"run_end": f["end"]}),
        "answer": lambda f: f["outcome"],
    },
    {
        "name": "provider_model", "check": "one_of",
        "applies": lambda f: True,
        "question": "Which provider and model ran this? Answer as provider/model, for example "
                    "codex/gpt-5. When no model is recorded, answer with the provider alone.",
        "excerpt": lambda f: _dump({"provider": f["provider"], "model": f["model"]}),
        "answer": lambda f: [f"{f['provider']}/{f['model']}"] if f["model"] else [f["provider"]],
    },
    {
        "name": "refused_count", "check": "number",
        "applies": lambda f: f["provider"] == "claude",
        "question": "How many tool calls did the gate refuse in this run? A refusal is an error "
                    "tool result whose text starts with hydra/. Answer with the number.",
        "excerpt": lambda f: "\n".join(_dump({"error_result": r["text"], "tool": r["tool"]})
                                       for r in f["refused"]) or "(no error results)",
        "answer": lambda f: len(f["refused"]),
    },
    {
        "name": "first_refused_tool", "check": "exact",
        "applies": lambda f: f["provider"] == "claude" and bool(f["refused"]) and bool(f["refused"][0]["tool"]),
        "question": "Which tool did the gate refuse first in this run? Answer with the tool name "
                    "alone.",
        "excerpt": lambda f: "\n".join(_dump({"error_result": r["text"], "tool": r["tool"]})
                                       for r in f["refused"]),
        "answer": lambda f: f["refused"][0]["tool"],
    },
    {
        "name": "interactive", "check": "exact",
        "applies": lambda f: True,
        "question": "Was this run an interactive session? Answer yes or no. launch_mode is "
                    "interactive for a session and recorded for a run Hydra launched with a task.",
        "excerpt": lambda f: _start_line(f),
        "answer": lambda f: "yes" if f["interactive"] else "no",
    },
    {
        "name": "seats_launched", "check": "number",
        "applies": lambda f: f["interactive"] and bool(f["seat_requests"]),
        "question": "How many seats did the seat_request lines of this run launch in total? Only a "
                    "request whose decision is launched counts, and each run in its runs list is "
                    "one seat. Answer with the number.",
        "excerpt": lambda f: "\n".join(_dump({"seat_request": r}) for r in f["seat_requests"]),
        "answer": lambda f: f["seats_launched"],
    },
    {
        "name": "policy", "check": "exact",
        "applies": lambda f: True,
        "question": "Which policy was this run launched under? Answer with the policy name, or "
                    "none when the run_start line has a null policy.",
        "excerpt": lambda f: _start_line(f),
        "answer": lambda f: f["policy"] or "none",
    },
]

OPEN = [
    {
        "name": "run_line",
        "applies": lambda f: bool(f["task"]),
        "question": "Write the `hydra run` command line that would launch this task text. Name "
                    "the provider and the policy in it.",
        "excerpt": lambda f: "TASK TEXT:\n" + f["task"],
    },
    {
        "name": "provider_policy",
        "applies": lambda f: bool(f["task"]),
        "question": "Pick a provider (claude, cursor-agent, agy or codex) and a policy for this "
                    "task text, and say why in two sentences.",
        "excerpt": lambda f: "TASK TEXT:\n" + f["task"],
    },
    {
        "name": "summarize",
        "applies": lambda f: bool(f["task"]) and bool(f["final_text"]),
        "question": "Say in one sentence what this run did.",
        "excerpt": lambda f: "TASK TEXT:\n" + f["task"] + "\n\nFINAL TEXT:\n" + f["final_text"],
    },
    {
        "name": "refusal_meaning",
        "applies": lambda f: bool(f["refused"]),
        "question": "Say what this refusal line means and what the agent should do next.",
        "excerpt": lambda f: "REFUSAL LINE:\n" + f["refused"][0]["text"],
    },
]


def _pick(pool: list[dict], name: str) -> list[dict]:
    return sorted(pool, key=lambda f: hashlib.sha256(f"{SEED}:{name}:{f['run_id']}".encode()).hexdigest())


def _make(template: dict, f: dict, klass: str) -> dict:
    prompt = f"{template['question']}\n\n{safe(template['excerpt'](f))}"
    task = {
        "id": f"{template['name']}:{f['run_id']}",
        "template": template["name"],
        "class": klass,
        "run_id": f["run_id"],
        "harness": f["provider"] or "hydra",
        "prompt": prompt,
    }
    if klass == "verifiable":
        task["check"] = template["check"]
        task["answer"] = template["answer"](f)
    return task


def build_bank(runs: Path, n: int) -> list[dict]:
    """`n` verifiable and `n` open tasks. Each template's runs are ordered by a hash of the run id,
    so the same records give the same bank whatever order they are listed in. Templates take turns
    so one cannot fill the class."""
    all_facts = [facts(item) for item in list_records(runs)]
    bank = []
    for klass, templates in (("verifiable", VERIFIABLE), ("open", OPEN)):
        pools = [(t, _pick([f for f in all_facts if t["applies"](f)], t["name"])) for t in templates]
        taken = [0] * len(pools)
        count = 0
        while count < n:
            progressed = False
            for i, (template, pool) in enumerate(pools):
                if count >= n:
                    break
                if taken[i] < len(pool):
                    bank.append(_make(template, pool[taken[i]], klass))
                    taken[i] += 1
                    count += 1
                    progressed = True
            if not progressed:
                break
    return sorted(bank, key=lambda t: t["id"])


def bank_build(n: int) -> int:
    runs = records.hydra_root() / "runs"
    bank = build_bank(runs, n)
    records.write_jsonl(home() / "bank.jsonl", bank)
    verifiable = sum(t["class"] == "verifiable" for t in bank)
    print(f"bank: {verifiable} verifiable and {len(bank) - verifiable} open tasks from {runs}")
    return 0


# ---------------------------------------------------------------- attempts


def ollama_url() -> str:
    return os.environ.get("HYDRA_LOCAL_LLM_URL", DEFAULT_URL)


def ollama_model() -> str:
    return os.environ.get("HYDRA_LOCAL_MODEL", DEFAULT_MODEL)


def prompt_sha() -> str:
    return hashlib.sha256(INSTRUCTIONS.encode()).hexdigest()


def generate(prompt: str) -> str:
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


def parse_reply(text: str) -> tuple[str, str] | None:
    try:
        reply = json.loads(text)
    except (json.JSONDecodeError, TypeError):
        return None
    if not isinstance(reply, dict) or "answer" not in reply:
        return None
    answer = reply["answer"]
    if isinstance(answer, bool) or not isinstance(answer, (str, int, float)):
        return None
    reason = reply.get("reason")
    return str(answer), reason if isinstance(reason, str) else ""


def _norm(text) -> str:
    return str(text).strip().strip("`\"'").rstrip(".").strip().lower()


def grade(task: dict, answer) -> bool:
    check, want = task["check"], task["answer"]
    if check == "exact":
        return _norm(answer) == _norm(want)
    if check == "one_of":
        return _norm(answer) in {_norm(w) for w in want}
    if check == "number":
        found = re.fullmatch(r"\s*(-?\d+)\s*\.?\s*", str(answer))
        return found is not None and int(found.group(1)) == int(want)
    raise ValueError(f"unknown check {check!r}")


def attempt(limit: int | None, minutes: float | None) -> int:
    base = home()
    bank = records.read_jsonl(base / "bank.jsonl")
    if not bank:
        print("attempt: no bank yet, run `apprentice.py bank build` first.", file=sys.stderr)
        return 1
    path = base / "attempts.jsonl"
    sha = prompt_sha()
    model = ollama_model()
    done = {(a["task_id"], a["prompt_sha"], a["model"]) for a in records.read_jsonl(path)}
    began = clock()
    written = 0
    path.parent.mkdir(parents=True, exist_ok=True)
    for task in bank:
        if (task["id"], sha, model) in done:
            continue
        if limit is not None and written >= limit:
            break
        if minutes is not None and clock() - began >= minutes * 60:
            print(f"attempt: stopped at the {minutes:g} minute budget after {written} attempts.")
            return 0
        prompt = f"{INSTRUCTIONS}\n\nTASK:\n{task['prompt']}\n"
        t0 = clock()
        try:
            raw = generate(prompt)
        except Unavailable as err:
            print(f"attempt: {err}, so the pass stopped and kept the {written} attempts it had written.",
                  file=sys.stderr)
            return 1
        parsed = parse_reply(raw)
        answer, reason = parsed if parsed else ("", "")
        line = {
            "task_id": task["id"],
            "template": task["template"],
            "prompt_sha": sha,
            "answer": answer,
            "reason": reason,
            "model": model,
            "seconds": round(clock() - t0, 2),
        }
        if parsed is None:
            line["unparsed"] = raw[:500]
        if task["class"] == "verifiable":
            line["correct"] = parsed is not None and grade(task, answer)
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(line, allow_nan=False) + "\n")
        written += 1
    print(f"attempt: wrote {written} attempts")
    return 0


# ---------------------------------------------------------------- score


def _tally(rows: list[dict], sha: str, templates: dict) -> dict[str, list[int]]:
    latest = {}
    for row in rows:
        if row["prompt_sha"] == sha and "correct" in row:
            latest[row["task_id"]] = row
    tally: dict[str, list[int]] = {}
    for task_id, row in latest.items():
        name = templates.get(task_id) or row.get("template") or task_id.split(":")[0]
        for key in (name, "overall"):
            cell = tally.setdefault(key, [0, 0])
            cell[0] += bool(row["correct"])
            cell[1] += 1
    return tally


def _print_model(rows: list[dict], sha: str, templates: dict, model: str) -> None:
    mine = [row for row in rows if row.get("model") == model]
    previous = next((row["prompt_sha"] for row in reversed(mine) if row["prompt_sha"] != sha), None)
    now = _tally(mine, sha, templates)
    before = _tally(mine, previous, templates) if previous else {}
    print(f"model {model}")
    header = f"prompt sha {sha[:8]}"
    if previous:
        header += f", previous {previous[:8]}"
    print(header)
    for name in sorted(now, key=lambda k: (k != "overall", k)):
        got, total = now[name]
        out = f"{name:<20}{got}/{total}  {got / total:.2f}"
        if name in before:
            pg, pt = before[name]
            out += f"   previous {pg}/{pt}  {pg / pt:.2f}   delta {got / total - pg / pt:+.2f}"
        print(out)


def score(only: str | None = None) -> int:
    base = home()
    rows = records.read_jsonl(base / "attempts.jsonl")
    if not rows:
        print("score: no attempts yet.")
        return 0
    templates = {t["id"]: t["template"] for t in records.read_jsonl(base / "bank.jsonl")}
    current = rows[-1]["prompt_sha"]
    models = sorted({row["model"] for row in rows if row["prompt_sha"] == current})
    if only is not None:
        models = [only] if only in models else []
    if not models:
        print("score: no attempts yet.")
        return 0
    for model in models:
        _print_model(rows, current, templates, model)
    return 0


# ---------------------------------------------------------------- pairs


def pairs(graded: Path) -> int:
    base = home()
    bank = {t["id"]: t for t in records.read_jsonl(base / "bank.jsonl")}
    out_path = base / "pairs.jsonl"
    kept = {(p["task_id"], p["graded_by"]): p for p in records.read_jsonl(out_path)}
    stamp = utc_now()
    added = skipped = 0
    for row in records.read_jsonl(graded):
        verdict = row.get("verdict")
        if verdict == "same":
            continue
        task = bank.get(row.get("task_id"))
        if task is None or task["class"] != "open" or verdict not in ("better", "worse"):
            print(f"pairs: skipped {row.get('task_id')!r}, verdict {verdict!r}: "
                  f"{'not an open task in the bank' if task is None or task['class'] != 'open' else 'verdict is not better, same or worse'}")
            skipped += 1
            continue
        local, better = row.get("local_answer"), row.get("better_answer")
        if not isinstance(local, str) or not isinstance(better, str):
            print(f"pairs: skipped {row['task_id']!r}, an answer is missing")
            skipped += 1
            continue
        # The verdict is on the local answer. `better` keeps it, `worse` swaps in the panel's.
        chosen, rejected = (local, better) if verdict == "better" else (better, local)
        key = (row["task_id"], str(row.get("graded_by")))
        added += key not in kept
        kept[key] = {
            "task_id": row["task_id"],
            "prompt": [{"role": "user", "content": task["prompt"]}],
            "chosen": [{"role": "assistant", "content": redact.redact_string(chosen)}],
            "rejected": [{"role": "assistant", "content": redact.redact_string(rejected)}],
            "harness": task["harness"],
            "graded_by": str(row.get("graded_by")),
            "graded_at": stamp,
            "source": "apprentice",
        }
    records.write_jsonl(out_path, [kept[k] for k in sorted(kept)])
    print(f"pairs: {added} new, {len(kept)} in {out_path.name}, {skipped} skipped")
    return 0


# ---------------------------------------------------------------- main


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="apprentice.py", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    bank_cmd = sub.add_parser("bank", help="the task bank")
    bank_sub = bank_cmd.add_subparsers(dest="action", required=True)
    build = bank_sub.add_parser("build", help="write bank.jsonl from the run records")
    build.add_argument("--n", type=int, default=20)
    att = sub.add_parser("attempt", help="ask the local model each task it has not attempted at this prompt")
    att.add_argument("--limit", type=int)
    att.add_argument("--minutes", type=float)
    sc = sub.add_parser("score", help="the verifiable score per model and template")
    sc.add_argument("--model", help="print only this model's block")
    pr = sub.add_parser("pairs", help="turn a graded file into pairs.jsonl")
    pr.add_argument("graded", type=Path)
    args = parser.parse_args(argv)
    if args.command == "bank":
        return bank_build(args.n)
    if args.command == "attempt":
        return attempt(args.limit, args.minutes)
    if args.command == "score":
        return score(args.model)
    return pairs(args.graded)


if __name__ == "__main__":
    sys.exit(main())
