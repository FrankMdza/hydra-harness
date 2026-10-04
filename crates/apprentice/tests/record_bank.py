"""Record what `apprentice.py bank build` gives, so `bank.rs` can compare the Rust.

Build ticket 231, the first of the apprentice's port. Build ticket 234 deleted
the Python it records, so it runs only in a checkout taken from before that ticket, from
that checkout's root:

    python3 crates/apprentice/tests/record_bank.py

It starts itself again with nothing in the environment, and then runs
`integrations/apprentice/apprentice.py` once per case below as a child, from a scratch
directory, with nothing in the child's environment but what the case names. A case plants
files, runs one command line, and keeps the exit code, stdout, stderr and every file under
the scratch directory that was not planted, which is `bank.jsonl` and the `.tmp` a refused
write leaves. It writes `crates/apprentice/tests/fixtures/bank.json`.

`bank build` starts no program and reads no clock, so there are no stubs and nothing is
frozen but the records: the twelve of `tests/fixtures/records/` are copied into the fixture
once as `corpus`, so a new fixture record does not change a recorded bank. The scratch
directory is written `<root>` wherever it was printed, and the checkout `<repo>`, which is
where a traceback names `apprentice.py`.
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRIPT = REPO / "integrations" / "apprentice" / "apprentice.py"
CORPUS = REPO / "tests" / "fixtures" / "records"
MARK = "HYDRA_APPRENTICE_RECORDING"
ROOT = "<root>"

PREAMBLE = (
    "Hydra launched this run and wrote its instructions at spawn.\n\n"
    "Work inside the policy, and if the task cannot be done inside it, say so and stop.\n"
)
OLD_PREAMBLE = (
    "Hydra launched this run and wrote its instructions at spawn. An older wording.\n"
    "If the task cannot be done, say so and stop.\n\n"
)
SECRET = "sk-ant-" + "a1B2c3D4" * 5
OPENAI = "sk-proj-planted_openai-key-value-0123"


def ident(n: int) -> str:
    return f"00000000-0000-7000-8000-{n:012d}"


def lines(*rows) -> str:
    return "".join(json.dumps(row) + "\n" for row in rows)


def start(run_id, provider="claude", model=None, **fields) -> dict:
    hydra = {"type": "run_start", "command": "run", "policy": "standard", "launch_mode": "recorded",
             "task": "probe"}
    hydra.update(fields)
    return {"hydra": hydra, "model": model, "provider": provider, "run_id": run_id, "seq": 0,
            "ts": "2026-09-05T20:51:39.370Z"}


def end(run_id, seq=1, provider="claude", **fields) -> dict:
    hydra = {"type": "run_end", "exit_code": 0, "refusal": None, "error": None}
    hydra.update(fields)
    return {"hydra": hydra, "provider": provider, "run_id": run_id, "seq": seq,
            "ts": "2026-09-05T20:52:01.905Z"}


def frame(run_id, seq, raw) -> dict:
    return {"provider": "claude", "run_id": run_id, "seq": seq, "raw": raw}


def assistant(*content) -> dict:
    return {"type": "assistant", "message": {"content": list(content)}}


def user(*content) -> dict:
    return {"type": "user", "message": {"content": list(content)}}


def seat(seq, run_id, **fields) -> dict:
    hydra = {"type": "seat_request", "request_id": f"r{seq}", "shape": "review", "seats": 1,
             "task": "look", "decision": "launched", "reason": None, "runs": ["a"]}
    hydra.update(fields)
    return {"hydra": hydra, "provider": "claude", "run_id": run_id, "seq": seq}


def synthesized() -> dict:
    """Records beside the corpus: seat requests, refusals, an interactive start, secrets, a
    long task, the two preambles, and the shapes `facts` reads around."""
    out = {}

    def put(name, *rows, raw=None):
        out[f"runs/{name}.jsonl"] = raw if raw is not None else lines(*rows)

    a = ident(101)
    put(a, start(a, launch_mode="interactive", task=None),
        seat(1, a, seats=3, runs=["a", "b", "c"]),
        seat(2, a, shape="x", seats=5, task="no", decision="refused", reason="no shape", runs=[]),
        seat(3, a, seats="2", runs=None),
        seat(4, a, seats=2.7, runs="not a list"),
        seat(5, a, seats=True, runs={"a": 1}),
        seat(6, a, seats=None, runs=None),
        seat(7, a, seats="٣", runs=None),
        end(a, 8))
    b = ident(102)
    put(b, start(b, task=f"use the key {SECRET} to deploy, and {OPENAI} too"), end(b))
    c = ident(103)
    put(c, start(c, model="opus", task="fix the gate"),
        frame(c, 1, assistant({"type": "tool_use", "id": "t1", "name": "Bash", "input": {}},
                              {"type": "tool_use", "id": "t2", "name": "Read"},
                              {"type": "tool_use", "id": 7, "name": "Seven"})),
        frame(c, 2, user({"type": "tool_result", "tool_use_id": "t1", "is_error": True,
                          "content": "hydra/standard: the shell may not name ~/hydra/runs"},
                         {"type": "tool_result", "tool_use_id": "t2", "is_error": True,
                          "content": [{"type": "text", "text": "hydra/standard: read refused"},
                                      {"type": "text", "text": "second é part"}, "not a dict"]},
                         {"type": "tool_result", "tool_use_id": "t2", "is_error": True,
                          "content": "an error that is not the gate's"},
                         {"type": "tool_result", "tool_use_id": 7.0, "is_error": True,
                          "content": [{"type": "image"}, {"text": "hydra/x: by a float id"}]},
                         {"type": "tool_result", "tool_use_id": "t1", "is_error": False,
                          "content": "hydra/ but not an error"},
                         "a block that is no dict")),
        frame(c, 3, assistant({"type": "text", "text": "first words"})),
        frame(c, 4, user({"type": "text", "text": "a user's text is not final"})),
        frame(c, 5, assistant({"type": "text", "text": "Done. The gate refused twice   and I stopped."})),
        end(c, 6, exit_code=1))
    d = ident(104)
    put(d, start(d, task="x", model=None),
        frame(d, 1, {"type": "system", "model": 5}),
        frame(d, 2, {"type": "system", "model": "claude-from-raw"}),
        frame(d, 3, user({"type": "tool_result", "tool_use_id": "nobody", "is_error": 1,
                          "content": "hydra/strict: no tool named"})),
        end(d, 4, exit_code=False))
    e = ident(105)
    put(e, start(e, task="late model", model=""),
        *[frame(e, n, {"type": "system"}) for n in range(1, 6)],
        frame(e, 6, {"type": "system", "model": "too-late"}),
        end(e, 7, refusal={"code": "policy"}, exit_code=0))
    f = ident(106)
    put(f, start(f, provider="codex", model="gpt-5", policy=None, task=OLD_PREAMBLE + "  the old wording's task \n"),
        frame(f, 1, assistant({"type": "text", "text": "codex frames are not read"})),
        end(f, 2, provider="codex", exit_code=0.0))
    g = ident(107)
    put(g, start(g, task=PREAMBLE.rstrip("\n") + "\n\nthe current preamble's task", policy=0))
    h = ident(108)
    put(h, start(h, task="long " + ("wérd " * 700) + "end", policy="strict"), end(h, exit_code=3))
    i = ident(109)
    put(i, start(i, model="m\udfff", task="a lone surrogate \ud800 and an emoji \U0001f409",
                 policy="lone \ud800 then a dragon \U0001f409"), end(i))
    put("not-a-run", start("x", command="doctor"))
    r = ident(110)
    put(r, start(r, command="resume", launch_mode="interactive", task="   "),
        seat(1, r, decision="launched", runs=[]), end(r, 2))
    put("numeric-id", start(5, model="m", task="numbered"), end(5))
    put("same-id-again", start(b, model="m2", task="the same run id as another"), end(b))
    put("garbled", start(ident(111)), raw=lines(start(ident(111))) + "{nope\n")
    put("line-zero-list", raw="[1, 2]\n")
    put("torn", raw=lines(start(ident(112), task="torn tail")) + '{"hydra": ')
    put("no-provider", start(ident(113), provider=""))
    out["runs/not-a-record.txt"] = "ignored\n"
    out["runs/.hidden.jsonl"] = lines(start(ident(114), task="a hidden file is listed"))
    return out


def corpus() -> dict:
    return {f"runs/{p.name}": p.read_text(encoding="utf-8") for p in sorted(CORPUS.glob("*/*.jsonl"))}


def run(root: Path, case: dict) -> dict:
    env = {key: value.replace(ROOT, str(root)) for key, value in case["env"].items()}
    ran = subprocess.run(
        [sys.executable, "-B", str(SCRIPT), *case["argv"]],
        cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True,
    )

    def plain(data: bytes) -> str:
        return data.decode("utf-8").replace(str(root), ROOT).replace(str(REPO), "<repo>")

    return {"exit": ran.returncode, "stdout": plain(ran.stdout), "stderr": plain(ran.stderr)}


def play(case: dict, base: dict) -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp).resolve()
        with_corpus = case.pop("corpus")
        planted = dict(base) if with_corpus else {}
        planted.update(case["files"])
        for name, text in planted.items():
            path = root / name
            if text is None:
                path.mkdir(parents=True, exist_ok=True)
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode("utf-8", "surrogatepass") if isinstance(text, str) else text)
        for link, target in case.get("links", {}).items():
            (root / link).symlink_to(target)
        result = run(root, case)
        written = []
        for path in sorted(p for p in root.rglob("*") if p.is_file()):
            name = str(path.relative_to(root))
            if name not in planted:
                written.append({"path": name, "text": path.read_bytes().decode("utf-8")})
        return {**case, "corpus": with_corpus, **result,
                "written": written}


def cases(more: dict) -> list[dict]:
    env = {"HYDRA_ROOT": "root", "HYDRA_GRADER_HOME": "home"}
    out = []

    def case(name, argv, files=None, env=env, corpus=True, links=None):
        assert all(c["name"] != name for c in out), name
        out.append({"name": name, "argv": argv, "env": env, "corpus": corpus,
                    "files": {f"root/{k}" if not k.startswith("/") else k[1:]: v
                              for k, v in (files or {}).items()},
                    "links": links or {}})

    preamble = {"config/preamble.md": PREAMBLE}
    case("the corpus and the synthesized records, default n", ["bank", "build"], {**more, **preamble})
    for n in ["4", "1", "0", "-1", "500", " 7 ", "1_0", "٣"]:
        case(f"n of {n!r}", ["bank", "build", "--n", n], more)
    case("n written with an equals sign", ["bank", "build", "--n=3"], more)
    case("the corpus alone", ["bank", "build", "--n", "200"])
    case("the corpus and no preamble file", ["bank", "build", "--n", "500"], more)
    case("a preamble with CRLF line ends", ["bank", "build", "--n", "500"],
         {**more, "config/preamble.md": PREAMBLE.replace("\n", "\r\n") + "\n\n\n"})
    case("no runs directory", ["bank", "build"], corpus=False)
    case("an empty runs directory", ["bank", "build"], {"runs": None}, corpus=False)
    case("the root and home spelled with slashes and dots", ["bank", "build", "--n", "2"],
         env={"HYDRA_ROOT": "./root//", "HYDRA_GRADER_HOME": "home/./deep//"})
    case("no root named, so the home directory", ["bank", "build", "--n", "2"],
         {"/h/hydra/runs/x.jsonl": lines(start(ident(1), task="under HOME"))},
         env={"HOME": f"{ROOT}/h"}, corpus=False)
    case("an empty root and grader home are not named", ["bank", "build", "--n", "2"],
         {"/h/hydra/runs/x.jsonl": lines(start(ident(1), task="under HOME"))},
         env={"HOME": f"{ROOT}/h", "HYDRA_ROOT": "", "HYDRA_GRADER_HOME": ""}, corpus=False)
    case("the grader home under the root", ["bank", "build", "--n", "2"],
         env={"HYDRA_ROOT": f"{ROOT}/root"})

    def broken(name, *rows, raw=None, files=None, links=None):
        record = {"runs/broken.jsonl": raw if raw is not None else lines(*rows)}
        case(name, ["bank", "build"], {**record, **(files or {})}, corpus=False, links=links)

    x = ident(900)
    broken("a later line that is a list", start(x), [1])
    broken("a hydra field that is a list on a later line", start(x), {"hydra": [1]})
    broken("a hydra field that is a string on line 0", {**start(x), "hydra": "run"})
    broken("a task that is a number", start(x, task=5))
    broken("a task that is a number, with a preamble", start(x, task=5), files=preamble)
    broken("seats that do not read as an int", start(x, launch_mode="interactive"), seat(1, x, seats="two", runs=None))
    broken("seats that are a list", start(x, launch_mode="interactive"), seat(1, x, seats=[1], runs=None))
    broken("seats that are infinite", start(x, launch_mode="interactive"), seat(1, x, seats=1e400, runs=None))
    broken("final text that is a number", start(x, task="t"), frame(x, 1, assistant({"type": "text", "text": 5})))
    broken("a tool id that is a list", start(x), frame(x, 1, assistant({"type": "tool_use", "id": [1], "name": "B"})))
    broken("a tool result id that is a dict", start(x),
           frame(x, 1, user({"type": "tool_result", "tool_use_id": {}, "is_error": True, "content": "hydra/x"})))
    broken("an error text that is a number", start(x),
           frame(x, 1, user({"type": "tool_result", "is_error": True, "content": [{"text": 5}]})))
    broken("a policy that is NaN", start(x, policy=float("nan")))
    broken("a provider that is a number", start(x, provider=3, model=None))
    broken("a run id with a lone surrogate", start("\ud800"))
    broken("a garbled line, skipped", raw=lines(start(x)) + "{bad\n")
    broken("invalid UTF-8, skipped", raw=lines(start(x)).encode() + b"\xff\n")
    broken("a preamble that is not UTF-8", start(x), files={"config/preamble.md": b"\xff\xfe"})
    broken("a directory named like a record", start(x), files={"runs/d.jsonl": None})
    broken("a link to nothing named like a record", start(x), links={"root/runs/l.jsonl": "nowhere"})
    case("a grader home that is a file", ["bank", "build"], {"/home": "a file"}, corpus=False)

    for argv in [[], ["-h"], ["--help"], ["bank"], ["bank", "-h"], ["bank", "build", "-h"],
                 ["bank", "build", "--he"], ["bogus"], ["bank", "bild"], ["bank", "build", "--n"],
                 ["bank", "build", "--n", "x"], ["bank", "build", "--n", "1.5"],
                 ["bank", "build", "--n", "-x"], ["bank", "build", "extra"],
                 ["bank", "build", "--n", "2", "--x"], ["--x", "bank", "build"],
                 ["bank", "--x", "build"], ["bank", "build", "--", "x"], ["bank", "build", "--"],
                 ["--", "bank", "build"], ["bank", "build", "--n", "2", "--n", "1"],
                 ["bank", "build", "-n", "2"], ["bank", "build", "--n=", "2"]]:
        case(f"argv {argv!r}", argv, corpus=False)
    return out


def main() -> None:
    if os.environ.get(MARK) != "1":
        # Nothing of the recorder's shell reaches the Python: no locale, no `PATH`, no `HOME`.
        os.execve(sys.executable, [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
                  {MARK: "1", "PYTHONDONTWRITEBYTECODE": "1"})
    out = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "fixtures"
    out.mkdir(parents=True, exist_ok=True)
    base = {f"root/{k}": v for k, v in corpus().items()}
    more = synthesized()
    recorded = []
    for case in cases(more):
        recorded.append(play(case, base))

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

    for case in recorded:
        case["files"] = {k: hexed(v) for k, v in case["files"].items()}
    fixture = {"python": sys.version.split()[0], "corpus": base, "cases": recorded}
    (out / "bank.json").write_text(json.dumps(fixture, indent=1, ensure_ascii=False) + "\n",
                                   encoding="utf-8")


if __name__ == "__main__":
    main()
