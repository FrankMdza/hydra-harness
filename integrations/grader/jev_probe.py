#!/usr/bin/env python3
"""Nothing under src/ changes, nothing reads ~/hydra/runs/, and the script sends nothing but committed diffs."""

from __future__ import annotations

import argparse
import json
import math
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Iterator

MODEL = "jev-latest"
STATE_TOKEN_BUDGET = 32_000
DIFF_PREFIXES = ("src/", "tests/", "integrations/")
MIN_TICKET = 112
BUILD_DIR = Path(".scratch/hydra/build")

RUN_RE = re.compile(r"\brun\s+[0-9a-f]{8}-[0-9a-f]{4}", re.IGNORECASE)

QUESTIONS: dict[str, str] = {
    "needs_correction": "Does the diff need a correction before it is applied?",
    "unmeasured_number": "Does the diff state a number it could not have measured?",
    "scope_creep": "Does the diff change behaviour the task did not ask for?",
    "piecewise_data_loss": "Can the diff lose data that arrives in pieces?",
}

SCORE_QUESTION = "needs_correction"


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(out.stdout.strip())


def ticket_number(path: Path) -> int | None:
    m = re.match(r"^(\d+)-", path.name)
    return int(m.group(1)) if m else None


def read_outcome(text: str) -> str | None:
    if not RUN_RE.search(text):
        return None
    lower = text.lower()
    if re.search(
        r"session\s+(?:changed|fixed|reworded)|added\s+something\s+to\s+the\s+diff",
        lower,
    ):
        return "changed"
    if "applied the diff as written" in lower or re.search(
        r"\bapplied\s+as\s+written\b", lower
    ):
        return "as_written"
    return None


def extract_outcome(body: str) -> str | None:
    marker = "## Outcome"
    start = body.find(marker)
    if start < 0:
        return None
    rest = body[start + len(marker) :]
    nxt = rest.find("\n## ")
    section = rest if nxt < 0 else rest[:nxt]
    return read_outcome(section)


def ticket_status(body: str) -> str | None:
    for line in body.splitlines():
        if line.startswith("Status:"):
            return line.split(":", 1)[1].strip().lower()
    return None


def iter_build_tickets(root: Path) -> Iterator[Path]:
    build = root / BUILD_DIR
    for path in sorted(build.glob("*.md")):
        num = ticket_number(path)
        if num is None or num < MIN_TICKET:
            continue
        yield path


def find_build_commit(repo: Path, number: int) -> str | None:
    prefix = f"Build {number}:"
    out = subprocess.run(
        ["git", "log", "--format=%H", "-1", f"--grep=^{prefix}"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    line = out.stdout.strip()
    return line or None


def commit_diff(repo: Path, commit: str) -> str:
    out = subprocess.run(
        [
            "git",
            "show",
            commit,
            "--pretty=format:",
            "--",
            "src/",
            "tests/",
            "integrations/",
        ],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    return out.stdout


def estimate_tokens(text: str) -> int:
    return max(1, len(text) // 4)


def longest_question_tokens() -> int:
    return max(estimate_tokens(q) for q in QUESTIONS.values())


def state_budget_ok(diff: str) -> bool:
    return estimate_tokens(diff) + longest_question_tokens() <= STATE_TOKEN_BUDGET


def cmd_build(out_path: Path) -> int:
    repo = repo_root()
    rows: list[dict[str, Any]] = []
    skips: list[str] = []
    counts = {"changed": 0, "as_written": 0}

    for path in iter_build_tickets(repo):
        num = ticket_number(path)
        assert num is not None
        body = path.read_text(encoding="utf-8")
        status = ticket_status(body)
        if status != "resolved":
            skips.append(f"{num}: status {status or 'missing'}")
            continue
        label = extract_outcome(body)
        if label is None:
            skips.append(f"{num}: outcome not labelled")
            continue
        commit = find_build_commit(repo, num)
        if commit is None:
            skips.append(f"{num}: no Build {num}: commit")
            continue
        diff = commit_diff(repo, commit)
        if not diff.strip():
            skips.append(f"{num}: empty diff under src/, tests/, integrations/")
            continue
        rows.append(
            {
                "ticket": num,
                "label": label,
                "diff": diff,
                "commit": commit,
            }
        )
        counts[label] += 1

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")

    print(f"changed: {counts['changed']}")
    print(f"as_written: {counts['as_written']}")
    for line in skips:
        print(f"skip: {line}")
    return 0


def cmd_patches(labels_path: Path, out_path: Path) -> int:
    """Rows from the agents' own diffs, before the session's review.

    `build` reads the `Build NNN:` commit, which is the diff after the session corrected it, so
    "does this need a correction" has no true answer there. The session kept each agent's diff as
    it came back; `labels_path` is JSON, a list of {ticket, label, patch, note}, labelled by hand
    from each ticket's `## Outcome`. Map 81-b, build 134.
    """
    labels = json.loads(labels_path.read_text(encoding="utf-8"))
    counts = {"changed": 0, "as_written": 0}
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("w", encoding="utf-8") as handle:
        for row in labels:
            diff = Path(row["patch"]).expanduser().read_text(encoding="utf-8")
            handle.write(json.dumps({"ticket": row["ticket"], "label": row["label"], "diff": diff},
                                    ensure_ascii=False) + "\n")
            counts[row["label"]] += 1
    print(f"changed: {counts['changed']}")
    print(f"as_written: {counts['as_written']}")
    return 0


def api_base() -> str:
    return os.environ.get("TYPESAFE_API_BASE", "https://api.typesafe.ai").rstrip("/")


def jev_request(diff: str) -> dict[str, Any]:
    questions = {
        key: {"type": "noul", "instructions": text} for key, text in QUESTIONS.items()
    }
    return {"state": diff, "model": MODEL, "questions": questions}


def post_jev(payload: dict[str, Any]) -> dict[str, Any]:
    key = os.environ.get("TYPESAFE_API_KEY")
    if not key:
        raise SystemExit("TYPESAFE_API_KEY is not set")
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        f"{api_base()}/v1/systemone",
        data=data,
        headers={
            "Authorization": f"Bearer {key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read().decode("utf-8"))


def parse_noul_answers(response: dict[str, Any]) -> dict[str, float]:
    answers = response.get("answers") or {}
    out: dict[str, float] = {}
    for key in QUESTIONS:
        entry = answers.get(key) or {}
        if entry.get("type") != "noul":
            raise ValueError(f"missing noul answer for {key}")
        out[key] = float(entry["noul"])
    return out


def cmd_ask(in_path: Path, out_path: Path) -> int:
    skipped = 0
    sent = 0
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with in_path.open(encoding="utf-8") as src, out_path.open(
        "w", encoding="utf-8"
    ) as dst:
        for line in src:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            diff = row.get("diff") or ""
            if not state_budget_ok(diff):
                skipped += 1
                continue
            response = post_jev(jev_request(diff))
            probs = parse_noul_answers(response)
            dst.write(
                json.dumps(
                    {
                        "ticket": row.get("ticket"),
                        "label": row.get("label"),
                        "model": response.get("model", MODEL),
                        "answers": probs,
                    },
                    ensure_ascii=False,
                )
                + "\n"
            )
            sent += 1
    print(f"asked: {sent}")
    print(f"skipped_over_limit: {skipped}")
    return 0


def label_yes(label: str) -> float:
    return 1.0 if label == "changed" else 0.0


def brier_score(pairs: list[tuple[float, float]]) -> float:
    if not pairs:
        return math.nan
    return sum((p - y) ** 2 for p, y in pairs) / len(pairs)


def reliability_bins(pairs: list[tuple[float, float]], bins: int = 5) -> list[dict[str, Any]]:
    width = 1.0 / bins
    table: list[dict[str, Any]] = []
    for i in range(bins):
        lo = i * width
        hi = 1.0 if i == bins - 1 else (i + 1) * width
        bucket = [(p, y) for p, y in pairs if lo <= p < hi or (i == bins - 1 and p == 1.0)]
        if not bucket:
            table.append(
                {
                    "bin": f"[{lo:.1f},{hi:.1f})",
                    "n": 0,
                    "mean_pred": None,
                    "fraction_yes": None,
                }
            )
            continue
        mean_pred = sum(p for p, _ in bucket) / len(bucket)
        frac = sum(y for _, y in bucket) / len(bucket)
        table.append(
            {
                "bin": f"[{lo:.1f},{hi:.1f})" if i < bins - 1 else f"[{lo:.1f},1.0]",
                "n": len(bucket),
                "mean_pred": mean_pred,
                "fraction_yes": frac,
            }
        )
    return table


def accuracy_at_threshold(pairs: list[tuple[float, float]], threshold: float = 0.5) -> float:
    if not pairs:
        return math.nan
    correct = sum(1 for p, y in pairs if (p >= threshold) == (y >= 0.5))
    return correct / len(pairs)


def rank_auc(pairs: list[tuple[float, float]]) -> float:
    """How often a `changed` diff scores above an `as_written` one, ties counting half.

    Calibration and ranking can part ways: on 2026-09-26 Jev put every diff below 0.75 and failed
    the 0.5 threshold, yet ranked the four clean diffs at the bottom.
    """
    pos = [p for p, y in pairs if y >= 0.5]
    neg = [p for p, y in pairs if y < 0.5]
    if not pos or not neg:
        return math.nan
    wins = sum(1.0 if a > b else 0.5 if a == b else 0.0 for a in pos for b in neg)
    return wins / (len(pos) * len(neg))


def cmd_score(path: Path) -> int:
    pairs: list[tuple[float, float]] = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            label = row.get("label")
            answers = row.get("answers") or {}
            if label not in ("changed", "as_written"):
                continue
            if SCORE_QUESTION not in answers:
                continue
            pairs.append((float(answers[SCORE_QUESTION]), label_yes(label)))

    print(
        "About thirty labelled build diffs make every number below loose; treat them as directional."
    )
    print(f"question: {SCORE_QUESTION}")
    print(f"n: {len(pairs)}")
    print(f"brier: {brier_score(pairs):.6f}")
    print(f"accuracy_at_0.5: {accuracy_at_threshold(pairs):.6f}")
    print(f"rank_auc: {rank_auc(pairs):.6f}  (0.5 is chance; with few as_written rows it is loose)")
    if pairs:
        base = sum(y for _, y in pairs) / len(pairs)
        print(f"base_rate_yes: {base:.6f}  (always answering the majority label scores {max(base, 1 - base):.6f})")
        print(f"brier_of_the_base_rate: {sum((base - y) ** 2 for _, y in pairs) / len(pairs):.6f}")
    print("reliability:")
    for row in reliability_bins(pairs):
        mean_pred = row["mean_pred"]
        frac = row["fraction_yes"]
        mean_s = f"{mean_pred:.4f}" if mean_pred is not None else "-"
        frac_s = f"{frac:.4f}" if frac is not None else "-"
        print(f"  {row['bin']} n={row['n']} mean_pred={mean_s} fraction_yes={frac_s}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Jev probe on committed build diffs")
    sub = parser.add_subparsers(dest="command", required=True)

    build_p = sub.add_parser("build", help="Label build tickets and capture committed diffs")
    build_p.add_argument("out", type=Path)

    patches_p = sub.add_parser("patches", help="Rows from the agents' own diffs and hand labels")
    patches_p.add_argument("labels", type=Path)
    patches_p.add_argument("out", type=Path)

    ask_p = sub.add_parser("ask", help="Ask Jev about each diff in a jsonl file")
    ask_p.add_argument("inp", type=Path)
    ask_p.add_argument("out", type=Path)

    score_p = sub.add_parser("score", help="Score calibration for needs_correction")
    score_p.add_argument("answers", type=Path)

    args = parser.parse_args(argv)
    if args.command == "build":
        return cmd_build(args.out)
    if args.command == "patches":
        return cmd_patches(args.labels, args.out)
    if args.command == "ask":
        return cmd_ask(args.inp, args.out)
    if args.command == "score":
        return cmd_score(args.answers)
    return 1


if __name__ == "__main__":
    sys.exit(main())
