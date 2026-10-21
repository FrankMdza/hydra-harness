"""Record what `apprentice.py pairs` gives, so `pairs.rs` can compare the Rust.

Build ticket 233, the third of the apprentice's port. Build ticket 234 deleted
the Python it records, so it runs only in a checkout taken from before that ticket, from
that checkout's root:

    python3 crates/apprentice/tests/record_pairs.py

It starts itself again with nothing in the environment, then plays each case below in a
scratch directory. A case is one or more steps, most of them `pairs` run twice on the same
graded file. A step plants files and runs `integrations/apprentice/apprentice.py` as a child
with nothing in its environment but what the step names. `pairs` starts no program and
contacts nothing, so there are no stubs.

What a step keeps: the exit code, stdout, stderr, and every file the step wrote or changed,
which is `pairs.jsonl` and the `.tmp` a refused write leaves. A file counts as changed when
its text differs once the stamps below are read, so that a second run in a later second
than the first does not record a change the same run within the second would not. It writes
`crates/apprentice/tests/fixtures/pairs.json`.

Three things are replaced before the fixture is written, so that recording twice gives the
same bytes: the scratch directory is `<root>`, the checkout a traceback names is `<repo>`,
and each `graded_at` the step stamped, which is the wall clock, is `<graded_at>` once it is
checked to be `YYYY-MM-DDTHH:MM:SSZ` and no earlier than the recording. A planted pair
carries a stamp from 2000, which is kept as it is, so a pair the step left alone can be told
from one it wrote again.

Since build ticket 351 the fixture departs from this Python: each step's planted bank has a
sibling `attempts.jsonl` that the Python never reads, and the last three cases were written by
hand, not recorded.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRIPT = REPO / "integrations" / "apprentice" / "apprentice.py"
MARK = "HYDRA_APPRENTICE_RECORDING"
ROOT = "<root>"
ENV = {"HYDRA_GRADER_HOME": "home"}
STAMP = re.compile(r'"graded_at": "([^"]*)"')
OLD = "2000-01-01T00:00:00Z"
SECRET = "sk-ant-" + "a1B2c3D4" * 5
OPENAI = "sk-proj-planted_openai-key-value-0123"
START = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def jsonl(*rows) -> str:
    return "".join(json.dumps(row) + "\n" for row in rows)


def task(ident, cls="open", harness="claude", prompt=None, **more):
    """A bank task as `_make` writes one."""
    row = {"id": ident, "template": str(ident).split(":")[0], "class": cls, "run_id": "r",
           "harness": harness, "prompt": prompt if prompt is not None else f"what did {ident} do?"}
    if cls == "verifiable":
        row.update(check="exact", answer="yes")
    row.update(more)
    return row


OPEN = [task(f"summary:{n}", harness=h) for n, h in enumerate(["claude", "codex", "hydra", "agy"])]
BANK = [*OPEN, task("provider:9", cls="verifiable")]


def line(i, verdict, graded_by="panel-a", **more) -> dict:
    """A graded line as `test_apprentice.py` writes one."""
    row = {"task_id": OPEN[i]["id"], "local_answer": f"local {i}",
           "better_answer": f"better {i}", "verdict": verdict, "graded_by": graded_by}
    row.update(more)
    return row


def pair(task_id, graded_by="panel-a", chosen="c", rejected="r", harness="claude", **more):
    """A pair as an earlier `pairs` wrote one, stamped in 2000."""
    row = {"task_id": task_id, "prompt": [{"role": "user", "content": "old prompt"}],
           "chosen": [{"role": "assistant", "content": chosen}],
           "rejected": [{"role": "assistant", "content": rejected}],
           "harness": harness, "graded_by": graded_by, "graded_at": OLD, "source": "apprentice"}
    row.update(more)
    return row


class Case:
    def __init__(self, rec, name: str):
        self.rec, self.name = rec, name
        self.steps, self.puts = [], {}

    def put(self, path: str, text):
        self.puts[path] = text
        return self

    def bank(self, *rows):
        return self.put("home/apprentice/bank.jsonl", jsonl(*rows))

    def kept(self, *rows):
        return self.put("home/apprentice/pairs.jsonl", jsonl(*rows))

    def graded(self, *rows, name="graded.jsonl"):
        return self.put(name, jsonl(*rows))

    def step(self, *argv, env=None):
        argv = argv or ("pairs", "graded.jsonl")
        self.steps.append({"files": self.puts, "argv": list(argv),
                           "env": dict(ENV if env is None else env)})
        self.puts = {}
        return self

    def twice(self, *argv, env=None):
        return self.step(*argv, env=env).step(*argv, env=env)

    def done(self):
        assert not self.puts, self.name
        self.rec.play(self)


class Recorder:
    def __init__(self):
        self.cases = []

    def case(self, name: str) -> Case:
        assert all(case["name"] != name for case in self.cases), name
        return Case(self, name)

    def play(self, case: Case) -> None:
        steps = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            given = {}
            for step in case.steps:
                for name, text in step["files"].items():
                    path = root / name
                    if text is None:
                        path.mkdir(parents=True, exist_ok=True)
                        continue
                    path.parent.mkdir(parents=True, exist_ok=True)
                    data = text if isinstance(text, bytes) else text.encode("utf-8", "surrogatepass")
                    path.write_bytes(data)
                    given[name] = seen(data, root)

                env = {key: value.replace(ROOT, str(root)) for key, value in step["env"].items()}
                argv = [arg.replace(ROOT, str(root)) for arg in step["argv"]]
                ran = subprocess.run([sys.executable, "-B", str(SCRIPT), *argv],
                                     cwd=root, env=env, stdin=subprocess.DEVNULL,
                                     capture_output=True, timeout=120)
                files = []
                for path in sorted(p for p in root.rglob("*") if p.is_file()):
                    name = str(path.relative_to(root))
                    text = seen(path.read_bytes(), root)
                    if given.get(name) != text:
                        files.append({"path": name, "text": text})
                        given[name] = text
                steps.append({"argv": step["argv"], "env": step["env"],
                              "planted": {k: hexed(v) for k, v in step["files"].items()},
                              "exit": ran.returncode,
                              "stdout": plain(ran.stdout.decode("utf-8"), root),
                              "stderr": plain(ran.stderr.decode("utf-8"), root),
                              "written": files})
        self.cases.append({"name": case.name, "steps": steps})


def plain(text: str, root: Path) -> str:
    return text.replace(str(root), ROOT).replace(str(REPO), "<repo>")


def seen(data: bytes, root: Path):
    """A file as a step is judged by: its text with the stamps read, or its bytes where it
    is not UTF-8. A second run that wrote the same pairs a second later wrote nothing new."""
    try:
        return stamped(plain(data.decode("utf-8"), root))
    except UnicodeDecodeError:
        return data


def stamped(text: str) -> str:
    """Each stamp this recording wrote, checked and read as `<graded_at>`."""
    def one(match):
        stamp = match.group(1)
        if stamp == OLD:
            return match.group(0)
        assert re.fullmatch(r"\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ", stamp), stamp
        assert stamp >= START, stamp
        return '"graded_at": "<graded_at>"'
    return STAMP.sub(one, text)


def hexed(value):
    """A planted file as text, or as hex where it is not UTF-8, or null for a directory."""
    if value is None:
        return None
    if isinstance(value, bytes):
        return {"hex": value.hex()}
    try:
        value.encode("utf-8")
        return value
    except UnicodeEncodeError:
        return {"hex": value.encode("utf-8", "surrogatepass").hex()}


def test_apprentice_cases(rec: Recorder) -> None:
    """The four graded files of `test_apprentice.py`'s `PairsTests`, each run twice."""
    case = rec.case("only better and worse become pairs")
    case.bank(*BANK).graded(line(0, "better"), line(1, "same"), line(2, "worse"))
    case.twice().done()

    case = rec.case("a pair carries the harness, the grader and the time")
    case.bank(*BANK).graded(line(0, "worse"))
    case.twice().done()

    case = rec.case("an unknown task and a bad verdict are named, not written")
    case.bank(*BANK).graded(line(0, "great"), dict(line(1, "better"), task_id="nope:none"))
    case.twice().done()

    case = rec.case("pairs are redacted and rerunning does not duplicate")
    case.bank(*BANK).graded(dict(line(0, "worse"), better_answer=f"use {SECRET}"))
    case.twice().done()


def cases(rec: Recorder) -> None:
    case = rec.case("a second grading replaces the first and keeps its place")
    case.bank(*BANK).graded(line(0, "better"), line(1, "worse"))
    case.step()
    case.graded(line(1, "better", local_answer="local again"), line(3, "worse"))
    case.step().done()

    case = rec.case("pairs already kept, from other graders and older runs")
    case.bank(*BANK)
    case.kept(pair("summary:2", "panel-b"), pair("summary:0", "panel-a"), pair("a:gone", "x"),
              pair("summary:0", "panel-b"))
    case.graded(line(0, "worse"), line(0, "better", graded_by="panel-c"), line(2, "better", "panel-b"))
    case.twice().done()

    case = rec.case("the grader as str() prints it")
    case.bank(*BANK)
    rows = [line(0, "better", graded_by=None), line(1, "better", graded_by=5),
            line(2, "worse", graded_by={"b": [1, None]}), line(3, "worse", graded_by=1.5)]
    del rows[0]["graded_by"]
    case.graded(*rows, line(1, "worse", graded_by="5"), line(3, "better", graded_by=True))
    case.twice().done()

    case = rec.case("verdicts that are not better, same or worse")
    case.bank(*BANK)
    case.graded(*[line(0, verdict) for verdict in [None, 5, ["better"], "Better", "better ", {"a": 1},
                                                   True, 1.5, "", "sam\u00e9"]],
                {k: v for k, v in line(1, "x").items() if k != "verdict"})
    case.twice().done()

    case = rec.case("a same verdict is passed over before the task is looked up")
    case.bank(*BANK)
    case.graded(dict(line(0, "same"), task_id=["a list"]), dict(line(0, "same"), task_id="ghost"),
                {"verdict": "same"}, line(1, "better"))
    case.twice().done()

    case = rec.case("a task id that is a list raises")
    case.bank(*BANK).graded(line(1, "better"), dict(line(0, "better"), task_id=["a"]))
    case.step().done()

    case = rec.case("a task id that is a dict raises")
    case.bank(*BANK).graded(dict(line(0, "worse"), task_id={"a": 1}))
    case.step().done()

    case = rec.case("tasks that are not open, and ids that are not in the bank")
    case.bank(*BANK, task("odd:1", cls="Open"))
    case.graded(dict(line(0, "better"), task_id="provider:9"), dict(line(0, "worse"), task_id="odd:1"),
                dict(line(0, "better"), task_id=None), {k: v for k, v in line(0, "better").items()
                                                         if k != "task_id"},
                dict(line(0, "worse"), task_id=7), dict(line(0, "better"), task_id="ünï \U0001f409"),
                dict(line(0, "better"), task_id="a'quote"), dict(line(0, "better"), task_id="tab\there"),
                dict(line(0, "worse"), task_id="a \ud800 surrogate"))
    case.twice().done()

    case = rec.case("answers that are missing or not text")
    case.bank(*BANK)
    case.graded(dict(line(0, "better"), local_answer=None), dict(line(1, "worse"), better_answer=5),
                {k: v for k, v in line(2, "better").items() if k != "local_answer"},
                dict(line(3, "worse"), local_answer=["x"]), dict(line(0, "worse"), local_answer=""),
                dict(line(1, "better"), better_answer=""))
    case.twice().done()

    case = rec.case("ids equal by Python's equality share a pair")
    case.bank(task(1), task(2.0), task(True, harness="t"), task("3"))
    case.kept(pair(1.0, "p"), pair(3, "p"))
    case.graded({"task_id": True, "local_answer": "a", "better_answer": "b", "verdict": "better",
                 "graded_by": "p"},
                {"task_id": 2, "local_answer": "a", "better_answer": "b", "verdict": "worse",
                 "graded_by": "p"},
                {"task_id": 2.0, "local_answer": "c", "better_answer": "d", "verdict": "better",
                 "graded_by": "q"})
    case.twice().done()

    case = rec.case("ids of text and a number cannot be sorted")
    case.bank(*BANK, task(4))
    case.kept(pair(3, "p"))
    case.graded(line(0, "better"), {"task_id": 4, "local_answer": "a", "better_answer": "b",
                                    "verdict": "worse", "graded_by": "p"}, line(0, "nah"))
    case.step().done()

    case = rec.case("a kept grader of None and a new one of 'None' cannot be sorted")
    case.bank(*BANK)
    case.kept(pair("summary:0", None))
    case.graded({k: v for k, v in line(0, "better").items() if k != "graded_by"})
    case.step().done()

    case = rec.case("a kept grader that is a number sorts with the new texts")
    case.bank(*BANK)
    case.kept(pair("summary:0", 5), pair("summary:0", 3))
    case.graded(line(1, "better"))
    case.step().done()

    case = rec.case("a NaN id stops the write, and the rows before it stay in the tmp file")
    case.bank(task(1), task(float("nan")), task(3))
    case.graded(*[{"task_id": i, "local_answer": "a", "better_answer": "b", "verdict": "better",
                   "graded_by": "p"} for i in [3, float("nan"), 1]])
    case.step().done()

    case = rec.case("two NaN ids are one key")
    case.bank(task(float("nan")))
    case.kept(pair(float("nan"), "p"))
    case.graded({"task_id": float("nan"), "local_answer": "a", "better_answer": "b",
                 "verdict": "better", "graded_by": "p"})
    case.step().done()

    case = rec.case("a bank task with a null id and a line with no task id")
    case.bank(task(None), *BANK)
    case.graded({"local_answer": "a", "better_answer": "b", "verdict": "better"})
    case.step().done()

    case = rec.case("a bank task with a null id and a line with no task id or answer")
    case.bank(task(None), *BANK)
    case.graded({"verdict": "better"})
    case.step().done()

    case = rec.case("the last of two bank tasks with one id wins")
    case.bank(task("dup:1", harness="first"), task("dup:1", harness="second", prompt={"a": [1]}))
    case.graded({"task_id": "dup:1", "local_answer": "a", "better_answer": "b", "verdict": "worse",
                 "graded_by": "p"})
    case.twice().done()

    for name, row in [("without a harness", {k: v for k, v in task("x:1").items() if k != "harness"}),
                      ("without a prompt", {k: v for k, v in task("x:1").items() if k != "prompt"}),
                      ("without a class", {k: v for k, v in task("x:1").items() if k != "class"})]:
        case = rec.case(f"a bank task {name} raises when it is used")
        case.bank(row, *BANK)
        case.graded(line(1, "better"), {"task_id": "x:1", "local_answer": "a", "better_answer": "b",
                                        "verdict": "better", "graded_by": "p"})
        case.step().done()

    for name, rows in [("without an id", [{"class": "open"}]), ("that is a list", [[1]]),
                       ("that is a string", ["a task"]), ("whose id is a list", [task(["a"])])]:
        case = rec.case(f"a bank task {name} raises")
        case.bank(*BANK, *rows).graded(line(0, "better"))
        case.step().done()

    for name, rows in [("without a grader", [{"task_id": "a"}]), ("without a task id", [{"graded_by": "a"}]),
                       ("that is a list", [[1]]), ("whose task id is a list", [pair(["a"])])]:
        case = rec.case(f"a kept pair {name} raises")
        case.bank(*BANK).kept(*rows).graded(line(0, "better"))
        case.step().done()

    case = rec.case("a graded line that is not a dict raises")
    case.bank(*BANK).graded(line(0, "better"), [1], line(1, "better"))
    case.step().done()

    case = rec.case("a graded line that does not parse raises")
    case.bank(*BANK).put("graded.jsonl", jsonl(line(0, "worse")) + "{bad\n")
    case.step().done()

    case = rec.case("a graded file whose last line is torn, and CRLF line ends")
    case.bank(*BANK).put("graded.jsonl", jsonl(line(0, "worse")).replace("\n", "\r\n")
                         + json.dumps(line(1, "better")))
    case.twice().done()

    case = rec.case("a graded file that is not UTF-8")
    case.bank(*BANK).put("graded.jsonl", jsonl(line(0, "worse")).encode() + b"\xff\xfe\n")
    case.step().done()

    case = rec.case("no graded file")
    case.bank(*BANK)
    case.twice("pairs", "missing.jsonl").done()

    case = rec.case("no bank and no graded file")
    case.step("pairs", "missing.jsonl").done()

    case = rec.case("no bank, so every task is unknown")
    case.graded(line(0, "better"), line(1, "worse"))
    case.twice().done()

    case = rec.case("a graded path that is a directory")
    case.bank(*BANK).put("graded.jsonl", None)
    case.step().done()

    case = rec.case("secrets in both answers, and in the prompt, which is kept")
    case.bank(task("s:1", prompt=f"the run used {SECRET}"))
    case.graded({"task_id": "s:1", "local_answer": f"the key is {OPENAI} and {SECRET}.",
                 "better_answer": f"'{SECRET}' with AKIAABCDEFGHIJKLMNOP\n-----BEGIN RSA",
                 "verdict": "worse", "graded_by": "p"},
                {"task_id": "s:1", "local_answer": "ghp_" + "x" * 36, "better_answer": "clean",
                 "verdict": "better", "graded_by": "q"})
    case.twice().done()

    case = rec.case("answers with a surrogate, an emoji and an accent before a key")
    case.bank(task("u:1"))
    case.graded({"task_id": "u:1", "local_answer": "a \ud800 lone surrogate \U0001f409",
                 "better_answer": f"é {SECRET}", "verdict": "better", "graded_by": "p\udfff"})
    case.twice().done()

    case = rec.case("many graders sort as tuples")
    case.bank(*BANK)
    case.graded(*[line(n % 4, "better" if n % 3 else "worse", graded_by=f"g{(n * 37) % 101}")
                  for n in range(150)])
    case.twice().done()

    for name, argv in [("an absolute path", ["pairs", f"{ROOT}/graded.jsonl"]),
                       ("a dot and a doubled slash", ["pairs", ".//graded.jsonl"]),
                       ("a trailing slash", ["pairs", "graded.jsonl/"]),
                       ("an empty path", ["pairs", ""]),
                       ("a path in a subdirectory", ["pairs", "sub/../graded.jsonl"])]:
        case = rec.case(f"the graded file named by {name}")
        case.bank(*BANK).graded(line(0, "better")).put("sub/x", "")
        case.step(*argv).done()

    case = rec.case("the grader home from the root, and from HOME")
    case.put("root/grader/apprentice/bank.jsonl", jsonl(*BANK)).graded(line(0, "better"))
    case.step(env={"HYDRA_ROOT": "root"})
    case.put("h/hydra/grader/apprentice/bank.jsonl", jsonl(*BANK))
    case.step(env={"HOME": f"{ROOT}/h", "HYDRA_ROOT": "", "HYDRA_GRADER_HOME": ""}).done()

    case = rec.case("a grader home that is a file")
    case.put("home", "a file").graded(line(0, "better"))
    case.step().done()

    case = rec.case("pairs.jsonl is a directory")
    case.bank(*BANK).put("home/apprentice/pairs.jsonl/x", "").graded(line(0, "better"))
    case.step().done()


def argv_cases(rec: Recorder) -> None:
    for argv in [["pairs"], ["pairs", "-h"], ["pairs", "--help"], ["pairs", "--h"],
                 ["pairs", "-hx"], ["pairs", "--help=x"], ["pairs", "graded.jsonl", "b"],
                 ["pairs", "-x"], ["pairs", "graded.jsonl", "-x"], ["pairs", "-x", "graded.jsonl"],
                 ["pairs", "--", "-x"], ["pairs", "-1"], ["pairs", "--", "graded.jsonl", "--"],
                 ["pairs", "graded.jsonl", "--"], ["pairs", "--"], ["pairs", "--", "--"],
                 ["pairs", "--x=1", "graded.jsonl"], ["--x", "pairs", "graded.jsonl"],
                 ["pairs", "-"], ["pairs", "a b"], ["pairs", "-h", "graded.jsonl"],
                 ["pairs", "graded.jsonl", "-h"], ["pairs", "graded.jsonl", "b", "-y", "c"],
                 ["pair", "graded.jsonl"], ["pairs", "-.5"], ["pairs", "--graded", "x"],
                 ["pairs", "-h=x"], ["pairs", "-h="], ["pairs", "-h-"], ["pairs", "-hh"],
                 ["pairs", "-hh=x"], ["pairs", "-hh-"], ["pairs", "-h1"], ["pairs", "-hx=y"],
                 ["pairs", "graded.jsonl", "-hx"], ["pairs", "-x", "-h=1"],
                 ["pairs", "graded.jsonl", "--", "b"], ["pairs", "--", "graded.jsonl", "b"],
                 ["pairs", "graded.jsonl", "-x", "--", "b"], ["pairs", "-x", "--", "graded.jsonl"],
                 ["pairs", "--x", "--"], ["pairs", "--", "-h"], ["pairs", "-", "b"],
                 ["-hx"], ["-h=x"], ["-hh"], ["-h-", "pairs", "graded.jsonl"],
                 ["bank", "build", "-hx"], ["bank", "-hx"], ["attempt", "-h=1"], ["score", "-hh"],
                 ["score", "--model", "m", "-hx"]]:
        case = rec.case(f"argv {argv!r}")
        case.bank(*BANK).graded(line(0, "better"))
        case.step(*argv).done()


def main() -> None:
    if os.environ.get(MARK) != "1":
        # Nothing of the recorder's shell reaches the Python: no locale, no `PATH`, no `HOME`.
        os.execve(sys.executable, [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
                  {MARK: "1", "PYTHONDONTWRITEBYTECODE": "1"})
    out = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "fixtures"
    out.mkdir(parents=True, exist_ok=True)
    rec = Recorder()
    test_apprentice_cases(rec)
    cases(rec)
    argv_cases(rec)
    fixture = {"python": sys.version.split()[0], "cases": rec.cases}
    (out / "pairs.json").write_text(json.dumps(fixture, indent=1, ensure_ascii=False) + "\n",
                                    encoding="utf-8")


if __name__ == "__main__":
    main()
