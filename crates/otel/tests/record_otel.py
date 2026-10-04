"""Record what `record_to_otlp.py` gives, so `otel.rs` can compare the Rust.

Build ticket 235, the otel port. From the checkout's root:

    python3 crates/otel/tests/record_otel.py

It starts itself again with nothing in the environment, and then runs
`integrations/otel/record_to_otlp.py` once per case below as a child, from a scratch
directory, with nothing in the child's environment but what the case names. `hydra` is a
stub: a shell script that appends its arguments, copies the rewritten record Hydra was
handed, and prints the `show --events` answer the case planted. No real `hydra` is
started.

A case plants files, runs one command line, and keeps the exit code, stdout, stderr and
every file under the scratch directory that was not planted, which is the `--out` file,
the batch directory, the stub's argv log and the rewritten record. The scratch directory
is written `<root>` wherever it was printed, and the checkout `<repo>`. It writes
`crates/otel/tests/fixtures/otel.json`.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRIPT = REPO / "integrations" / "otel" / "record_to_otlp.py"
CORPUS = REPO / "tests" / "fixtures" / "records"
MARK = "HYDRA_OTEL_RECORDING"
ROOT = "<root>"

CHILD = r"""
import importlib.util, sys
script = sys.argv[1]
sys.argv = ["hydra-otel", *sys.argv[2:]]
spec = importlib.util.spec_from_file_location("recorded_otel", script)
mod = importlib.util.module_from_spec(spec)
sys.modules["recorded_otel"] = mod
spec.loader.exec_module(mod)
raise SystemExit(mod.main() or 0)
"""

STUB = r"""#!/bin/sh
here="${0%/*}"
{
  printf 'argv:'
  for arg in "$@"; do printf ' [%s]' "$arg"; done
  printf '\n'
  if [ -f "$HYDRA_ROOT/runs/$2.jsonl" ]; then
    echo 'record: found'
    /bin/mkdir -p "$here/rewritten"
    /bin/cp "$HYDRA_ROOT/runs/$2.jsonl" "$here/rewritten/$2.jsonl"
  else
    echo 'record: missing'
  fi
} >> "$here/calls"
if [ -f "$here/table/$2.exit" ]; then
  read -r code < "$here/table/$2.exit"
  echo "stub hydra: no answer for $2" >&2
  exit "$code"
fi
exec /bin/cat "$here/table/$2"
"""

SECRET = "sk-proj-" + "A" * 40
OPENAI = "sk-ant-" + "a1B2c3D4" * 5


def ident(n: int) -> str:
    return f"00000000-0000-7000-8000-{n:012d}"


def lines(*rows) -> str:
    return "".join(json.dumps(row) + "\n" for row in rows)


def show(outcome="completed", events=None, counts=None, **more):
    usage = {"usage_source": "provider", "counts": counts or {"input_tokens": 4, "output_tokens": 1270}}
    usage.update(more)
    return json.dumps({"projection": {"outcome": outcome, "usage": usage}, "events": events or []}) + "\n"


TOOLS = [
    {"type": "tool_call", "call_id": "toolu_01", "name": "Write", "seq": 2},
    {"type": "permission_decision", "call_id": "toolu_01", "decision": "deny", "decider": "capability", "seq": 3},
    {"type": "tool_result", "call_id": "toolu_01", "status": "error", "output": "blocked", "seq": 4},
]


def start(run_id, provider="claude", model=None, **fields) -> dict:
    hydra = {"type": "run_start", "command": "run", "policy": "standard", "launch_mode": "recorded",
             "task": "probe"}
    hydra.update(fields)
    return {"hydra": hydra, "model": model, "provider": provider, "run_id": run_id, "seq": 0,
            "ts": "2026-09-05T20:51:39.370Z"}


def end(run_id, seq=1, provider="claude", **fields) -> dict:
    hydra = {"type": "run_end", "exit_code": 0, "refusal": None, "error": None,
             "ended_at": "2026-09-05T20:52:01.905Z"}
    hydra.update(fields)
    return {"hydra": hydra, "provider": provider, "run_id": run_id, "seq": seq,
            "ts": "2026-09-05T20:51:39.370Z"}


def seat(seq, run_id, **fields) -> dict:
    hydra = {"type": "seat_request", "request_id": f"r{seq}", "shape": "review", "seats": 1,
             "task": "look", "decision": "launched", "reason": None, "runs": ["a"]}
    hydra.update(fields)
    return {"hydra": hydra, "provider": "claude", "run_id": run_id, "seq": seq,
            "ts": "2026-09-05T20:51:39.370Z"}


def run_id_of(text: str) -> str:
    return json.loads(text.splitlines()[0])["run_id"]


def corpus() -> list[Path]:
    return sorted(CORPUS.rglob("*.jsonl"))


def convert_env(stub=True) -> dict:
    env = {"TMPDIR": f"{ROOT}/tmp"}
    if stub:
        env["HYDRA_BIN"] = f"{ROOT}/bin/hydra"
    return env


def hexed(value):
    if isinstance(value, bytes):
        return {"hex": value.hex()}
    if isinstance(value, str):
        try:
            value.encode("utf-8")
            return value
        except UnicodeEncodeError:
            return {"hex": value.encode("utf-8", "surrogatepass").hex()}
    return value


def run(root: Path, case: dict) -> dict:
    env = {key: value.replace(ROOT, str(root)) for key, value in case["env"].items()}
    ran = subprocess.run(
        [sys.executable, "-B", "-c", CHILD, str(SCRIPT), *case["argv"]],
        cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True,
    )

    def plain(data: bytes) -> str:
        return data.decode("utf-8").replace(str(root), ROOT).replace(str(REPO), "<repo>")

    return {"exit": ran.returncode, "stdout": plain(ran.stdout), "stderr": plain(ran.stderr)}


def play(case: dict) -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp).resolve()
        planted = dict(case["files"])
        (root / "tmp").mkdir()
        for name, text in planted.items():
            path = root / name
            if text is None:
                path.mkdir(parents=True, exist_ok=True)
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            data = text.encode("utf-8", "surrogatepass") if isinstance(text, str) else text
            path.write_bytes(data)
            if name == "bin/hydra":
                path.chmod(0o755)
        for link, target in case.get("links", {}).items():
            (root / link).symlink_to(target)
        originals = {}
        for name, text in planted.items():
            if text is None:
                continue
            originals[name] = text.encode("utf-8", "surrogatepass") if isinstance(text, str) else text
        result = run(root, case)
        written = []
        for path in sorted(p for p in root.rglob("*") if p.is_file()):
            name = str(path.relative_to(root))
            raw = path.read_bytes()
            if originals.get(name) == raw:
                continue
            try:
                text = raw.decode("utf-8")
            except UnicodeDecodeError:
                written.append({"path": name, "hex": raw.hex()})
            else:
                written.append({
                    "path": name,
                    "text": text.replace(str(root), ROOT).replace(str(REPO), "<repo>"),
                })
        files = {k: hexed(v) for k, v in case["files"].items()}
        return {**case, "files": files, **result, "written": written}


def table_for(text: str, **show_kw) -> dict:
    return {f"bin/table/{run_id_of(text)}": show(**show_kw)}


def stub_files(records: dict, **show_kw) -> dict:
    files = {"bin/hydra": STUB, **records}
    for text in records.values():
        if isinstance(text, str) and text.startswith("{"):
            files.update(table_for(text, **show_kw))
    return files


def cases() -> list[dict]:
    out = []

    def case(name, argv, files=None, env=None, links=None, stub=False, shows=None):
        assert all(c["name"] != name for c in out), name
        files = dict(files or {})
        if stub:
            files = {"bin/hydra": STUB, **files}
        if shows:
            files.update(shows)
        out.append({
            "name": name,
            "argv": argv,
            "env": env if env is not None else convert_env(stub=stub or bool(shows)),
            "files": files,
            "links": links or {},
        })

    for argv in [
        ["-h"], ["--help"], ["--he"], [],
        ["--all", "x"], ["--out", "x"], ["record", "--out-dir", "d"],
        ["--all", "x", "--out-dir", "d", "rec"], ["--all", "x", "--out-dir", "d", "--out", "o"],
        ["--o", "x"], ["--out"], ["--out-dir"], ["--all"],
        ["--n"], ["rec", "extra"], ["--help="], ["-hx"],
        ["--out="], ["--all="], ["--bogus", "-h"], ["-h", "--bogus"],
        ["--", "record"], ["record", "--"], ["--out", "-h"],
        ["--out-d", "d"], ["--all", "x", "--out-dir"],
    ]:
        case(f"argv {argv!r}", argv)

    records = {p.name: p.read_text(encoding="utf-8") for p in corpus()}
    first = next(p for p in corpus() if p.name.endswith("0001.jsonl"))
    first_text = first.read_text(encoding="utf-8")
    first_id = run_id_of(first_text)

    for path in corpus():
        text = path.read_text(encoding="utf-8")
        rid = run_id_of(text)
        shows = table_for(text)
        events = TOOLS if path.name.endswith("0001.jsonl") else None
        if events:
            shows = table_for(text, events=events)
        case(
            f"stdout {path.name}",
            [path.name],
            {path.name: text},
            stub=True,
            shows=shows,
        )
        if path.name.endswith("0001.jsonl"):
            case(
                "out of the first fixture",
                [path.name, "--out", "trace.json"],
                {path.name: text},
                stub=True,
                shows=shows,
            )

    planted = {f"runs/{name}": text for name, text in records.items()}
    shows = {}
    for text in records.values():
        shows.update(table_for(text, events=TOOLS if run_id_of(text) == first_id else None))
    case("batch of the twelve fixtures", ["--all", "runs", "--out-dir", "traces"], planted, stub=True, shows=shows)

    stale_json = '{"stale": true}\n'
    size = len(first_text.encode("utf-8"))
    skip_files = {
        f"runs/{first.name}": first_text,
        f"traces/{first.stem}.json": stale_json,
        f"traces/{first.stem}.size.json": f"{size}\n",
    }
    case("batch skips a stamp that matches", ["--all", "runs", "--out-dir", "traces"], skip_files, stub=True, shows=table_for(first_text))
    skip_files_refresh = {
        f"runs/{first.name}": first_text,
        f"traces/{first.stem}.json": stale_json,
        f"traces/{first.stem}.size.json": "0\n",
    }
    case("batch refreshes a stamp that does not match", ["--all", "runs", "--out-dir", "traces"], skip_files_refresh, stub=True, shows=table_for(first_text, events=TOOLS))

    nested = {"runs/sub/deep.jsonl": first_text}
    case("batch walks a nested directory", ["--all", "runs", "--out-dir", "traces"], nested, stub=True, shows=table_for(first_text))

    session, parent, child = [ident(n) for n in (21, 22, 23)]
    rows = [json.loads(line) for line in first_text.splitlines() if line]
    session_files = {}
    session_shows = {}
    for run_id, parent_id in ((session, None), (parent, session), (child, parent)):
        copied = [dict(row, run_id=run_id, parent_run_id=parent_id) for row in rows]
        copied.insert(-1, dict(copied[0], seq=99, ts=copied[0]["ts"], hydra={"type": "seat_request", "request_id": "request", "decision": "launched", "shape": "review", "runs": [child]}))
        text = lines(*copied)
        session_files[f"{run_id}.jsonl"] = text
        session_shows[f"bin/table/{run_id}"] = show(events=TOOLS)
    for run_id, label in ((session, "session root"), (parent, "middle"), (child, "leaf")):
        case(f"three-level {label}", [f"{run_id}.jsonl"], session_files, stub=True, shows=session_shows)

    long_task = "x" * 1990 + " " + SECRET
    a = ident(201)
    redacted = lines(start(a, task=long_task), end(a))
    case("redaction before task truncation", [f"{a}.jsonl"], {f"{a}.jsonl": redacted}, stub=True, shows=table_for(redacted))

    b = ident(202)
    typed = lines(start(b, task=f"use {OPENAI}"), end(b))
    counts = {"input_tokens": 4, "output_tokens": 1.5, "ok": 3, "flag": True, "note": [1, 2]}
    case(
        "non-int usage counts become strings",
        [f"{b}.jsonl"],
        {f"{b}.jsonl": typed},
        stub=True,
        shows=table_for(typed, outcome=True, counts=counts),
    )

    c = ident(203)
    seats = lines(start(c, launch_mode="interactive"), seat(1, c, runs=[ident(204), ident(205)]), end(c, 2))
    case("a seat request links its runs", [f"{c}.jsonl"], {f"{c}.jsonl": seats}, stub=True, shows=table_for(seats))

    d = ident(206)
    orphan_rows = [start(d), end(d)]
    orphan_rows[0]["parent_run_id"] = ident(999)
    orphan = lines(*orphan_rows)
    case("a missing parent is the session id", [f"{d}.jsonl"], {f"{d}.jsonl": orphan}, stub=True, shows=table_for(orphan))

    e = ident(207)
    loop_a = lines({**start(e), "parent_run_id": ident(208)})
    loop_b = lines({**start(ident(208)), "parent_run_id": e})
    case(
        "a parent cycle",
        [f"{e}.jsonl"],
        {f"{e}.jsonl": loop_a, f"{ident(208)}.jsonl": loop_b},
        stub=True,
        shows={**table_for(loop_a), **table_for(loop_b)},
    )

    case("a missing record", ["nope.jsonl"], stub=True, shows={"bin/table/nope": show()})
    case("an empty record", ["empty.jsonl"], {"empty.jsonl": ""}, stub=True)
    case("a record that is not run_start", ["x.jsonl"], {"x.jsonl": lines({"run_id": "x", "seq": 0, "ts": "2026-09-05T20:51:39.370Z", "hydra": {"type": "run_end"}})}, stub=True)
    case("a garbled line", ["bad.jsonl"], {"bad.jsonl": lines(start(ident(209))) + "{nope\n"}, stub=True, shows=table_for(lines(start(ident(209)))))
    case("a torn tail is ignored", ["torn.jsonl"], {"torn.jsonl": lines(start(ident(210)), end(ident(210))) + '{"hydra": '}, stub=True, shows=table_for(lines(start(ident(210)), end(ident(210)))))
    case("a directory named like a record", ["dir.jsonl"], {"dir.jsonl": None}, stub=True)
    case("hydra exits 1", [first.name], {first.name: first_text}, stub=True, shows={f"bin/table/{first_id}": show(), f"bin/table/{first_id}.exit": "1\n"})
    case("hydra prints nothing", [first.name], {first.name: first_text}, stub=True, shows={f"bin/table/{first_id}": ""})
    case("no hydra on PATH", [first.name], {first.name: first_text}, env={"TMPDIR": f"{ROOT}/tmp", "PATH": f"{ROOT}/empty"}, stub=False)
    case("an empty HYDRA_BIN", [first.name], {first.name: first_text}, env={"TMPDIR": f"{ROOT}/tmp", "HYDRA_BIN": "", "PATH": "/bin:/usr/bin"})
    case(
        "hydra found on PATH",
        [first.name],
        {first.name: first_text, "bin/hydra": STUB, **table_for(first_text, events=TOOLS)},
        env={"TMPDIR": f"{ROOT}/tmp", "PATH": f"{ROOT}/bin"},
    )

    blank = ident(211)
    case("a blank line in the record", [f"{blank}.jsonl"], {f"{blank}.jsonl": lines(start(blank)) + "\n" + lines(end(blank))}, stub=True, shows=table_for(lines(start(blank), end(blank))))

    f = ident(212)
    no_end = lines(start(f))
    case("no run_end uses the last envelope time", [f"{f}.jsonl"], {f"{f}.jsonl": no_end}, stub=True, shows=table_for(no_end))

    g = ident(213)
    nan_row = lines(start(g), {"hydra": {"type": "run_end", "ended_at": "2026-09-05T20:52:01.905Z"}, "run_id": g, "seq": 1, "ts": "2026-09-05T20:51:39.370Z", "extra": float("nan")})
    case("a NaN in the rewritten record", [f"{g}.jsonl"], {f"{g}.jsonl": nan_row}, stub=True, shows=table_for(nan_row))

    return out


def main() -> None:
    if os.environ.get(MARK) != "1":
        os.execve(
            sys.executable,
            [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
            {MARK: "1", "PYTHONDONTWRITEBYTECODE": "1"},
        )
    out = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "fixtures"
    out.mkdir(parents=True, exist_ok=True)
    recorded = [play(case) for case in cases()]
    fixture = {"python": sys.version.split()[0], "cases": recorded}
    (out / "otel.json").write_text(json.dumps(fixture, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{len(recorded)} cases, {(out / 'otel.json').stat().st_size} bytes")


if __name__ == "__main__":
    main()
