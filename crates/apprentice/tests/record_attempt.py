"""Record what `apprentice.py attempt` and `score` give, so `attempt.rs` can compare the Rust.

Build ticket 232, the second of the apprentice's port. Build ticket 234 deleted
the Python it records, so it runs only in a checkout taken from before that ticket, from
that checkout's root:

    python3 crates/apprentice/tests/record_attempt.py

It starts itself again with nothing in the environment, then plays each case below in a
scratch directory. A case is one or more steps. A step plants files, starts the stub model
server of `crates/grader/tests/record_judge.py` on a free port of 127.0.0.1 with the replies
the step queues (or names a port nothing listens on), and runs
`integrations/apprentice/apprentice.py` as a child with nothing in its environment but what
the step names. No real model server is contacted. `attempt` and `score` start no program,
so there are no shell stubs.

What a step keeps: the exit code, stdout, stderr, every file the step wrote or changed, and
each request the stub server received. It writes `crates/apprentice/tests/fixtures/attempt.json`.

Four things are replaced before the fixture is written, so that recording twice gives the
same bytes: the scratch directory is `<root>`, the stub server's port is `<port>`, the
checkout a traceback names is `<repo>`, and the `seconds` of each attempt line, which is the
wall clock, is `<seconds>` once it is checked to be a number rounded to two places. The
child also gets `PYTHONHASHSEED=0`, which is not part of a step's environment: `score`
sorts a set of models, and the order a set of text and a number is listed in, which decides
the words of the `TypeError` that sort raises, follows the hash of the text.

One case is a stated difference: a URL that names another host. The Python connects, here
to a name that does not resolve, and the Rust refuses it by name (map 95, question C). The
case is marked `differs` and `attempt.rs` checks the Rust's sentence instead.
"""

import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
SCRIPT = REPO / "integrations" / "apprentice" / "apprentice.py"
MARK = "HYDRA_APPRENTICE_RECORDING"
ROOT = "<root>"
PORT = "<port>"
URL = f"http://127.0.0.1:{PORT}/api/generate"


def _judge_recorder():
    spec = importlib.util.spec_from_file_location(
        "record_judge", REPO / "crates" / "grader" / "tests" / "record_judge.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


judge = _judge_recorder()
Server, free_port, http, says = judge.Server, judge.free_port, judge.http, judge.says

ENV = {
    "HYDRA_GRADER_HOME": "home",
    "HYDRA_LOCAL_LLM_URL": URL,
    "HYDRA_LOCAL_MODEL": "stub-model",
}
SECONDS = re.compile(r'"seconds": (-?[0-9.e+-]+)')


def task(ident, template="t", cls="verifiable", check="exact", answer="yes", prompt=None, **more):
    row = {"id": ident, "template": template, "class": cls, "prompt": prompt or f"question {ident}"}
    if cls == "verifiable":
        row.update(check=check, answer=answer)
    row.update(more)
    return row


def jsonl(*rows) -> str:
    return "".join(json.dumps(row) + "\n" for row in rows)


def answer(value, reason="because") -> bytes:
    return says({"reason": reason, "answer": value})


def response(value) -> bytes:
    """A 200 whose `response` is `value` as JSON, whatever its type."""
    return http(body=json.dumps({"response": value, "done": True}).encode())


def tried(task_id, model="stub-model", sha=None, correct=None, template="t", **more) -> dict:
    row = {"task_id": task_id, "template": template, "prompt_sha": sha or SHA, "answer": "a",
           "reason": "", "model": model, "seconds": 0.5}
    if correct is not None:
        row["correct"] = correct
    row.update(more)
    return row


def instructions_sha() -> str:
    spec = importlib.util.spec_from_file_location("recorded_apprentice", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module.prompt_sha()


SHA = instructions_sha()
OLD = "0123456789abcdef" * 4


class Case:
    def __init__(self, rec, name: str, differs: bool = False):
        self.rec, self.name, self.differs = rec, name, differs
        self.steps, self.puts = [], {}

    def put(self, path: str, text: str):
        self.puts[path] = text
        return self

    def bank(self, *rows):
        return self.put("home/apprentice/bank.jsonl", jsonl(*rows))

    def attempts(self, *rows):
        return self.put("home/apprentice/attempts.jsonl", jsonl(*rows))

    def step(self, *argv, replies=(), env=None, server="up", without=()):
        env = dict(ENV if env is None else env)
        for name in without:
            env.pop(name, None)
        self.steps.append({"files": self.puts, "argv": list(argv), "env": env,
                           "replies": [reply.hex() for reply in replies], "server": server})
        self.puts = {}
        return self

    def done(self):
        assert not self.puts, self.name
        self.rec.play(self)


class Recorder:
    def __init__(self):
        self.cases = []

    def case(self, name: str, differs: bool = False) -> Case:
        assert all(case["name"] != name for case in self.cases), name
        return Case(self, name, differs)

    def play(self, case: Case) -> None:
        steps = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            given = {}
            for step in case.steps:
                for name, text in step["files"].items():
                    path = root / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(text.encode("utf-8"))
                    given[name] = path.read_bytes()
                server = None
                if step["server"] == "up":
                    server = Server(bytes.fromhex(reply) for reply in step["replies"])
                    port = server.port
                else:
                    port = free_port()

                def plain(text: str) -> str:
                    return (text.replace(str(root), ROOT)
                            .replace(f"127.0.0.1:{port}", f"127.0.0.1:{PORT}")
                            .replace(str(REPO), "<repo>"))

                env = {key: value.replace(ROOT, str(root)).replace(PORT, str(port))
                       for key, value in step["env"].items()}
                env["PYTHONHASHSEED"] = "0"
                ran = subprocess.run([sys.executable, "-B", str(SCRIPT), *step["argv"]],
                                     cwd=root, env=env, stdin=subprocess.DEVNULL,
                                     capture_output=True, timeout=120)
                requests = []
                if server:
                    server.close()
                    requests = server.requests
                    assert not server.replies, (case.name, "replies left over")
                    for request in requests:
                        request["body"] = plain(request["body"])
                files = []
                for path in sorted(p for p in root.rglob("*") if p.is_file()):
                    name = str(path.relative_to(root))
                    data = path.read_bytes()
                    if given.get(name) != data:
                        text = plain(data.decode("utf-8"))
                        for seconds in SECONDS.findall(text):
                            assert re.fullmatch(r"\d+\.\d{1,2}", seconds), seconds
                        files.append({"path": name, "text": SECONDS.sub('"seconds": "<seconds>"', text)})
                        given[name] = data
                steps.append({**step, "exit": ran.returncode,
                              "stdout": plain(ran.stdout.decode("utf-8")),
                              "stderr": plain(ran.stderr.decode("utf-8")),
                              "files": files, "planted": step["files"], "requests": requests})
                del steps[-1]["files"]
                steps[-1]["written"] = files
        self.cases.append({"name": case.name, "differs": case.differs, "steps": steps})


EXACT = task("exact:a", template="provider", answer="Claude")
ONE_OF = task("one_of:b", template="outcome", check="one_of", answer=["completed", "failed"])
NUMBER = task("number:c", template="turns", check="number", answer=7)
OPEN = task("open:d", template="summary", cls="open", prompt="say what the run did")


def attempt_cases(rec: Recorder) -> None:
    case = rec.case("each check, and an open task with no grade")
    case.bank(EXACT, ONE_OF, NUMBER, OPEN)
    case.step("attempt", replies=[answer("`claude`."), answer("Failed"), answer(" 7 ."),
                                  answer("it fixed the gate", reason=5)])
    case.step("attempt").done()

    case = rec.case("wrong answers are graded false")
    case.bank(EXACT, ONE_OF, NUMBER)
    case.step("attempt", replies=[answer("cursor"), answer("crashed"), answer("8")]).done()

    numbers = ["7", "-7", "07", "٧", "7.0", "7 . ", " 7\x1c", "- 7", "7..", "", "+7",
               "1_0", "x7", "７"]
    case = rec.case("a number answer read as the regex and int() read it")
    case.bank(*[task(f"number:{n}", check="number", answer=7 if n != "-7" else -7)
                for n in range(len(numbers))])
    case.step("attempt", replies=[answer(text) for text in numbers]).done()

    wants = [7, "7", " 7 ", "٧", 7.9, -0.0, True, "1_0", 10]
    case = rec.case("the bank's number answer read as int() reads it")
    case.bank(*[task(f"want:{n}", check="number", answer=want) for n, want in enumerate(wants)])
    case.step("attempt", replies=[answer(text) for text in
                                  ["7", "7", "7", "7", "7", "0", "1", "10", "10"]]).done()

    for name, want in [("null", None), ("text", "seven"), ("NaN", float("nan")),
                       ("infinite", float("inf")), ("a list", [7])]:
        case = rec.case(f"a number answer of {name} in the bank raises")
        case.bank(task("n:1", check="number", answer=want))
        case.step("attempt", replies=[answer("7")]).done()

    case = rec.case("one_of over a string, a dict and a list of mixed values")
    case.bank(task("o:1", check="one_of", answer="abc"),
              task("o:2", check="one_of", answer={"Yes": 1, "no": 2}),
              task("o:3", check="one_of", answer=[1, None, "Maybe."]),
              task("o:4", check="one_of", answer=[]))
    case.step("attempt", replies=[answer("B"), answer("yes"), answer("'maybe'"),
                                  answer("x")]).done()

    case = rec.case("one_of over a number raises")
    case.bank(task("o:1", check="one_of", answer=3))
    case.step("attempt", replies=[answer("3")]).done()

    case = rec.case("exact compares text the norm strips and lowers")
    pairs = [("  `Claude`. ", "claude"), ("\"x.\".", "x"), ("ΟΔΟΣ", "οδος"), ("İ", "i̇"),
             ("3", 3), ("True", True), ("None", None), ("[1]", [1]), ("1.5", 1.5)]
    case.bank(*[task(f"e:{n}", answer=want) for n, (_, want) in enumerate(pairs)])
    case.step("attempt", replies=[answer(text) for text, _ in pairs]).done()

    case = rec.case("an unknown check raises")
    case.bank(task("u:1", check="regex"))
    case.step("attempt", replies=[answer("yes")]).done()

    case = rec.case("a check that is not text raises")
    case.bank(task("u:1", check=["exact"]))
    case.step("attempt", replies=[answer("yes")]).done()

    case = rec.case("answers of each type")
    values = [3, 3.0, 1e16, -0.0, 12345678901234567890123456789012345678901234567890, True,
              None, [1], {"a": 1}, "text"]
    case.bank(*[task(f"v:{n}", cls="open") for n in range(len(values))])
    case.step("attempt", replies=[answer(value) for value in values]).done()

    case = rec.case("replies the parser does not take")
    raw = ["not json", "[1, 2]", '{"reason": "no answer"}', '{"answer": NaN}',
           '{"answer": "a", "reason": null}', "\"just a string\"", "x" * 700]
    case.bank(*[task(f"r:{n}") for n in range(len(raw))])
    case.step("attempt", replies=[says(text) for text in raw]).done()

    case = rec.case("a response that is not text")
    case.bank(*[task(f"r:{n}") for n in range(3)])
    case.step("attempt", replies=[response(list(range(600))), response([]),
                                  response(["a", float("nan")])]).done()

    for name, value in [("a number", 5), ("null", None), ("a dict", {"a": 1}), ("true", True)]:
        case = rec.case(f"a response that is {name} raises")
        case.bank(task("r:1"))
        case.step("attempt", replies=[response(value)]).done()

    case = rec.case("a reply body that is not JSON, or has no response")
    case.bank(task("b:1"), task("b:2"), task("b:3"), task("b:4", cls="open"))
    case.step("attempt", replies=[http(body=b"plain words"), http(body=b'{"done": true}'),
                                  http(body=b"\xff\xfe{}"),
                                  http(body=b'{"reason": "r", "answer": "yes"}')]).done()

    case = rec.case("a reply whose answer has an integer past 4300 digits raises")
    case.bank(task("big:1"))
    case.step("attempt", replies=[says('{"answer": ' + "9" * 4301 + "}")]).done()

    case = rec.case("the prompt carries any value, a surrogate and an emoji")
    case.bank(task("p:1", prompt="ünïcode \ud800 and \U0001f409"), task("p:2", prompt=5),
              task("p:3", prompt={"a": [1, None]}), task("p:4", prompt=True))
    case.step("attempt", replies=[answer("yes")] * 4).done()

    case = rec.case("a task id the attempts already hold, by Python's equality")
    case.bank(task(1), task(2.0), task("x"), task(True, template="b"))
    case.attempts(tried(1.0), tried(2), tried("x", model="other"), tried("x", sha=OLD))
    case.step("attempt", replies=[answer("yes")]).done()

    case = rec.case("a duplicated task is asked twice")
    case.bank(task("d:1"), task("d:1"))
    case.step("attempt", replies=[answer("yes"), answer("no")]).done()

    for argv, replies in [(["--limit", "1"], 1), (["--limit", "0"], 0), (["--limit", "-1"], 0),
                          (["--limit", "99999999999999999999999999999999999999999"], 3),
                          (["--minutes", "0"], 0), (["--minutes", "-0"], 0),
                          (["--minutes", "-1.5e10"], 0), (["--minutes", "1e-300"], None),
                          (["--minutes", "1e9"], 3), (["--minutes", "inf"], 3),
                          (["--minutes", "nan"], 3), (["--limit", "1", "--minutes", "0"], 0),
                          (["--minutes", "0", "--limit", "0"], 0), (["--l=2", "--m", "30"], 2)]:
        case = rec.case(f"attempt {' '.join(argv)}")
        case.bank(task("l:1"), task("l:2"), task("l:3"))
        if replies is None:
            # A budget of a fraction of a nanosecond: the clock has moved by the time it is read.
            case.attempts(tried("l:1"), tried("l:2"), tried("l:3"))
            replies = 0
        case.step("attempt", *argv, replies=[answer("yes")] * replies).done()

    case = rec.case("the budget is checked only for a task still to ask")
    case.bank(task("l:1"))
    case.attempts(tried("l:1"))
    case.step("attempt", "--minutes", "0").done()

    case = rec.case("no bank")
    case.step("attempt", server="down").done()

    case = rec.case("an empty bank")
    case.put("home/apprentice/bank.jsonl", "")
    case.step("attempt", server="down").done()

    case = rec.case("a bank whose last line is torn")
    case.put("home/apprentice/bank.jsonl", jsonl(task("t:1")) + '{"id": ')
    case.step("attempt", replies=[answer("yes")]).done()

    case = rec.case("a bank line that does not parse raises")
    case.put("home/apprentice/bank.jsonl", jsonl(task("t:1")) + "{bad\n")
    case.step("attempt", server="down").done()

    case = rec.case("the model is not there")
    case.bank(task("m:1"), task("m:2"))
    case.step("attempt", replies=[answer("yes"), http(404, b'{"error": "model not found"}',
                                                      "Not Found")]).done()

    case = rec.case("the server answers 500 first")
    case.bank(task("m:1"))
    case.step("attempt", replies=[http(500, b"boom", "Internal Server Error")]).done()

    case = rec.case("nothing listens on the port")
    case.bank(task("m:1"))
    case.step("attempt", server="down").done()

    case = rec.case("a status line that is not one raises")
    case.bank(task("m:1"))
    case.step("attempt", replies=[b"SSH-2.0-OpenSSH_9.0\r\n"]).done()

    case = rec.case("the model and the URL come from the environment")
    case.bank(task("m:1"))
    case.step("attempt", replies=[answer("yes")], without=["HYDRA_LOCAL_MODEL"])
    case.step("attempt", replies=[answer("yes")], env={**ENV, "HYDRA_LOCAL_MODEL": ""})
    case.step("attempt", replies=[answer("yes")],
              env={**ENV, "HYDRA_LOCAL_LLM_URL": f"http://localhost:{PORT}/other?x=1"}).done()

    case = rec.case("a URL that names another host", differs=True)
    case.bank(task("m:1"))
    case.step("attempt", server="down",
              env={**ENV, "HYDRA_LOCAL_LLM_URL": "http://hydra-stub.invalid:9/api/generate"}).done()

    case = rec.case("a NaN in the bank is refused when the line is written")
    case.bank(task(float("nan")))
    case.step("attempt", replies=[answer("yes")]).done()

    for name, rows in [
        ("an attempt without a model", [{"task_id": "a", "prompt_sha": SHA}]),
        ("an attempt that is a list", [[1]]),
        ("an attempt whose task id is a list", [tried(["a"])]),
    ]:
        case = rec.case(f"{name} raises")
        case.bank(task("a"))
        case.attempts(*rows)
        case.step("attempt", server="down").done()

    for name, row in [("without an id", {"template": "t", "class": "open", "prompt": "p"}),
                      ("with a list id", task(["a"])),
                      ("without a prompt", {"id": "a", "template": "t", "class": "open"}),
                      ("without a class", {"id": "a", "template": "t", "prompt": "p"}),
                      ("without a template", {"id": "a", "class": "open", "prompt": "p"}),
                      ("that is a string", "a task")]:
        replies = [answer("yes")] if name in ("without a class", "without a template") else []
        case = rec.case(f"a task {name} raises")
        case.bank(row)
        case.step("attempt", replies=replies, server="up" if replies else "down").done()

    case = rec.case("attempts.jsonl is a directory")
    case.bank(task("a"))
    case.put("home/apprentice/attempts.jsonl/x", "")
    case.step("attempt", replies=[answer("yes")]).done()


def score_cases(rec: Recorder) -> None:
    bank = [task("provider:1", template="provider"), task("provider:2", template="provider"),
            task("turns:1", template="turns"), task("odd:1", template="")]

    case = rec.case("score with no attempts")
    case.step("score", server="down").done()

    case = rec.case("score with an empty attempts file")
    case.put("home/apprentice/attempts.jsonl", "")
    case.step("score", server="down").done()

    case = rec.case("score after an attempt pass")
    case.bank(EXACT, ONE_OF, NUMBER, OPEN)
    case.step("attempt", replies=[answer("Claude"), answer("nope"), answer("7"), answer("words")])
    case.step("score", server="down")
    case.step("score", "--model", "stub-model", server="down")
    case.step("score", "--model", "other", server="down").done()

    def scored(name, *argv, rows, tasks=bank):
        case = rec.case(name)
        case.bank(*tasks)
        case.attempts(*rows)
        case.step("score", *argv, server="down").done()

    two_prompts = [
        tried("provider:1", sha=OLD, correct=False), tried("provider:2", sha=OLD, correct=True),
        tried("turns:1", sha=OLD, correct=False),
        tried("provider:1", model="old-only", sha=OLD, correct=True),
        tried("provider:1", correct=True), tried("provider:1", correct=False),
        tried("provider:2", correct=True), tried("turns:1", correct=True),
        tried("odd:1", correct=True, template="from-row"),
        tried("gone:1", correct=False, template=""), tried("open:1"),
        tried("provider:1", model="b-model", correct=True),
    ]
    scored("the current prompt is the last attempt's, even an old one", rows=[
        *two_prompts, tried("provider:1", model="old-only", sha=OLD, correct=True)])
    scored("two prompts, two models, and the template from the bank, the row and the id",
           rows=two_prompts)
    scored("score --model picks one block", "--model", "b-model", rows=two_prompts)
    scored("score --model for a model only the old prompt has", "--model", "old-only",
           rows=two_prompts)
    scored("score --mo= is an empty model", "--mo=", rows=two_prompts)

    eighths = [tried(f"provider:{n}", correct=n < 1) for n in range(8)]
    eighths += [tried(f"provider:{n}", sha=OLD, correct=n < 3) for n in range(8)]
    eighths.append(tried("provider:0", correct=False, model="z"))
    scored("an eighth rounds to even, and a delta is signed", rows=eighths, tasks=[])

    scored("a template called overall counts twice", rows=[
        tried("a", correct=True, template="overall"), tried("b", correct=False)], tasks=[])
    scored("templates that are numbers and true sort with the rest", rows=[
        tried("a", correct=True, template=3), tried("b", correct=True, template=1.5),
        tried("c", correct=False, template=True), tried("d", correct=1, template=2)], tasks=[])
    scored("templates of text and a number cannot be sorted", rows=[
        tried("a", correct=True, template=3), tried("b", correct=True, template="x")], tasks=[])
    scored("models that are numbers", rows=[tried("a", model=2, correct=True),
                                            tried("a", model=1.5, correct=False)], tasks=[])
    scored("models of text and a number cannot be sorted", rows=[
        tried("a", model=2, correct=True), tried("a", model="m", correct=False)], tasks=[])
    scored("a previous prompt that is empty is no previous", rows=[
        tried("a", sha="", correct=True), tried("a", correct=False)], tasks=[])
    scored("a prompt sha that is a list prints its head", rows=[
        tried("a", sha=list(range(10)), correct=True)], tasks=[])
    scored("a prompt sha that is a number raises after the model line", rows=[
        tried("a", sha=5, correct=True)], tasks=[])
    scored("the last attempt without a prompt sha raises", rows=[
        tried("a", correct=True), {"task_id": "a", "model": "m"}], tasks=[])
    scored("an attempt without a model at the current prompt raises", rows=[
        {"task_id": "a", "prompt_sha": SHA}, tried("a", correct=True)], tasks=[])
    scored("an attempt without a model at another prompt is passed over", rows=[
        {"task_id": "a", "prompt_sha": OLD, "correct": True}, tried("a", correct=True)], tasks=[])
    scored("a task id that is a number and no template anywhere raises", rows=[
        tried(5, correct=True, template="")], tasks=[])
    scored("a task id the bank names as a number", rows=[
        tried(1.0, correct=True, template=""), tried(True, correct=False, template="")],
        tasks=[task(1, template="from-bank")])
    scored("a bank task without a template raises", rows=[tried("a", correct=True)],
           tasks=[{"id": "a"}])
    scored("a task id that is a list raises", rows=[tried(["a"], correct=True)], tasks=[])
    scored("ids with a colon and without", rows=[
        tried("tpl:x:y", correct=True, template=None), tried("plain", correct=False, template=None)],
        tasks=[])


def argv_cases(rec: Recorder) -> None:
    bank = [task("a:1")]
    for argv in [["attempt", "-h"], ["attempt", "--he"], ["score", "-h"], ["score", "--h"],
                 ["attempt", "--limit"], ["attempt", "--limit", "x"],
                 ["attempt", "--limit", "1.5"], ["attempt", "--minutes", "x"],
                 ["attempt", "--minutes"], ["attempt", "--m", "-x"], ["attempt", "-m", "1"],
                 ["attempt", "extra"], ["attempt", "--", "x"], ["attempt", "--"],
                 ["attempt", "--limit=", "1"], ["attempt", "--limit", "٠"],
                 ["attempt", "--minutes", "1_0", "--limit", " 0 "],
                 ["attempt", "--minutes", "-.5", "--limit", "0"],
                 ["attempt", "--minutes", "٣", "--limit", "0"],
                 ["attempt", "--x", "--limit", "0"], ["score", "--model"],
                 ["score", "x"], ["score", "--model", "-x"], ["score", "--model", "-1"],
                 ["score", "--", "--model"], ["score", "--model=a=b"],
                 ["--x", "score"]]:
        case = rec.case(f"argv {argv!r}")
        case.bank(*bank)
        case.step(*argv, server="down").done()


def main() -> None:
    if os.environ.get(MARK) != "1":
        # Nothing of the recorder's shell reaches the Python: no locale, no `PATH`, no `HOME`.
        os.execve(sys.executable, [sys.executable, str(Path(__file__).resolve()), *sys.argv[1:]],
                  {MARK: "1", "PYTHONDONTWRITEBYTECODE": "1"})
    out = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "fixtures"
    out.mkdir(parents=True, exist_ok=True)
    rec = Recorder()
    attempt_cases(rec)
    score_cases(rec)
    argv_cases(rec)
    fixture = {"python": sys.version.split()[0], "prompt_sha": SHA, "cases": rec.cases}
    (out / "attempt.json").write_text(json.dumps(fixture, indent=1, ensure_ascii=False) + "\n",
                                      encoding="utf-8")


if __name__ == "__main__":
    main()
