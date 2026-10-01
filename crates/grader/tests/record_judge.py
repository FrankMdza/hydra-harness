"""Record what `judge.py` gives, so `differential/judge.rs` can compare the Rust.

Build ticket 218. `record.py` calls `record(out, repo)`; it writes `fixtures/judge.json`.

Each case is a scratch directory and one or more steps. A step plants files, starts a stub
model server on a free port of 127.0.0.1 with the replies the step queues, and runs
`judge.py` with nothing in the environment but what the step names. No real model server is
contacted: every case that reaches the network names the stub's port, and the stub answers
with the bytes written here. `hydra` is a shell stub that answers `show <id>` from a table
of files and writes down how it was called.

What a step keeps: the exit code, stdout, stderr, every file the run wrote or changed, the
name of every file left, and each request the stub server received.

Three things are replaced before the fixture is written, so that recording twice gives the
same bytes: the scratch directory is `<root>`, the stub server's port is `<port>`, and the
checkout a traceback names is `<repo>`. `judged_at` is the wall clock in the Python, so the
child that runs `judge.py` sets `utc_now` to a fixed second.
"""

import json
import socket
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

ROOT = "<root>"
PORT = "<port>"
URL = f"http://127.0.0.1:{PORT}/api/generate"

HYDRA_STUB = r"""#!/bin/sh
here="${0%/*}"
{
  printf 'argv:'
  for arg in "$@"; do printf ' [%s]' "$arg"; done
  printf '\n'
  printf 'root: %s\n' "$HYDRA_ROOT"
} >> "$here/calls"
if [ ! -f "$here/table/$2" ]; then
  echo "stub hydra: no answer for $2" >&2
  exit 1
fi
exec /bin/cat "$here/table/$2"
"""

CHILD = """
import importlib.util, sys
script, now = sys.argv[1:3]
sys.argv = ["judge.py", *sys.argv[3:]]
spec = importlib.util.spec_from_file_location("recorded_judge", script)
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)
module.utc_now = lambda: now
raise SystemExit(module.main())
"""

ENV = {
    "HYDRA_ROOT": "root",
    "HYDRA_GRADER_HOME": "home",
    "HYDRA_BIN": f"{ROOT}/bin/hydra",
    "HYDRA_LOCAL_LLM_URL": URL,
    "HYDRA_LOCAL_MODEL": "stub-model",
}

GOOD = {"needs_correction": True, "failure_class": "agent", "reason": "The diff misses the test."}
FINE = {"reason": "It did what the task asked.", "result_ok": True}
UNEXPECTED = b"HTTP/1.0 599 Unexpected\r\n\r\n"


class Server:
    """Answers each connection with the next queued reply and keeps every request."""

    def __init__(self, replies):
        self.replies = list(replies)
        self.requests = []
        self.sock = socket.socket()
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(16)
        self.sock.settimeout(0.05)
        self.port = self.sock.getsockname()[1]
        self.stop = False
        self.thread = threading.Thread(target=self.serve, daemon=True)
        self.thread.start()

    def serve(self):
        while not self.stop:
            try:
                conn, _ = self.sock.accept()
            except OSError:
                continue
            with conn:
                conn.settimeout(10)
                data = b""
                while b"\r\n\r\n" not in data:
                    more = conn.recv(65536)
                    if not more:
                        break
                    data += more
                head, _, body = data.partition(b"\r\n\r\n")
                lines = head.decode("latin-1").split("\r\n")
                method, path, _ = lines[0].split(" ", 2)
                headers = {}
                for line in lines[1:]:
                    key, _, value = line.partition(":")
                    headers[key.strip().lower()] = value.strip()
                length = int(headers.get("content-length", "0"))
                while len(body) < length:
                    body += conn.recv(65536)
                self.requests.append({
                    "method": method,
                    "path": path,
                    "content_type": headers.get("content-type"),
                    "body": body.decode("utf-8"),
                })
                conn.sendall(self.replies.pop(0) if self.replies else UNEXPECTED)

    def close(self):
        self.stop = True
        self.thread.join()
        self.sock.close()


def free_port() -> int:
    """A port nothing listens on: bound once, then closed."""
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def http(code=200, body=b"", reason="OK", headers=(), version="HTTP/1.0") -> bytes:
    head = f"{version} {code} {reason}\r\n" + "".join(f"{line}\r\n" for line in headers)
    return head.encode() + b"\r\n" + body


def says(answer) -> bytes:
    """A 200 whose `response` is `answer`: text as it is, anything else as JSON."""
    text = answer if isinstance(answer, str) else json.dumps(answer)
    return http(body=json.dumps({"response": text, "done": True}).encode(),
                headers=["Content-Type: application/json"])


def whole(body: bytes) -> bytes:
    return http(body=body)


def jsonl(rows) -> str:
    return "".join(json.dumps(row) + "\n" for row in rows)


def row(ident, task="fix it", outcome="completed", verdict=None, **more) -> dict:
    return {"run_id": ident, "task_text": task, "outcome": outcome, "human_verdict": verdict, **more}


def shown(result="done", stat=None, **more) -> str:
    return json.dumps({"projection": {"result_text": result, "diff_stat": stat, **more}}) + "\n"


class Case:
    def __init__(self, rec, name: str):
        self.rec = rec
        self.name = name
        self.steps = []
        self.puts = [{"path": "bin/hydra", "stub": True}]

    def put(self, path: str, text=None, data=None):
        spec = {"path": path}
        if data is not None:
            spec["hex"] = data.hex()
        else:
            spec["text"] = text
        self.puts.append(spec)
        return self

    def rows(self, *rows, shows=True):
        """The dataset, and for each row that is an object with a plain id, what `hydra
        show` says about it."""
        self.put("home/dataset.jsonl", jsonl(rows))
        for item in rows:
            if shows and isinstance(item, dict) and isinstance(item.get("run_id"), str) \
                    and item["run_id"].isalnum():
                self.put(f"bin/table/{item['run_id']}", shown())
        return self

    def labels(self, *lines):
        return self.put("home/judge-labels.jsonl", jsonl(lines))

    def step(self, *argv, replies=(), env=None, server="up", without=()):
        env = dict(ENV if env is None else env)
        for name in without:
            env.pop(name, None)
        self.steps.append({
            "puts": self.puts, "argv": list(argv), "env": env,
            "replies": [reply.hex() for reply in replies], "server": server,
        })
        self.puts = []
        return self

    def done(self):
        assert not self.puts, self.name
        self.rec.play(self)


class Recorder:
    def __init__(self, repo: Path):
        self.repo = repo
        self.cases = []

    def case(self, name: str) -> Case:
        assert all(case["name"] != name for case in self.cases), name
        return Case(self, name)

    def play(self, case: Case) -> None:
        steps = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            given = {}
            for n, step in enumerate(case.steps):
                for spec in step["puts"]:
                    path = root / spec["path"]
                    path.parent.mkdir(parents=True, exist_ok=True)
                    if spec.get("stub"):
                        data = HYDRA_STUB.encode()
                    elif "hex" in spec:
                        data = bytes.fromhex(spec["hex"])
                    else:
                        data = spec["text"].encode("utf-8")
                    path.write_bytes(data)
                    if spec.get("stub"):
                        path.chmod(0o755)
                    given[spec["path"]] = data
                server = None
                if step["server"] == "up":
                    server = Server(bytes.fromhex(reply) for reply in step["replies"])
                    port = server.port
                else:
                    port = free_port()

                def plain(text: str) -> str:
                    return (text.replace(str(root), ROOT)
                            .replace(f"127.0.0.1:{port}", f"127.0.0.1:{PORT}")
                            .replace(str(self.repo), "<repo>"))

                env = {
                    key: value.replace(ROOT, str(root)).replace(PORT, str(port))
                    for key, value in step["env"].items()
                }
                now = f"2001-01-01T00:00:{n:02d}Z"
                ran = subprocess.run(
                    [
                        sys.executable, "-B", "-c", CHILD,
                        str(self.repo / "integrations" / "grader" / "judge.py"),
                        now, *step["argv"],
                    ],
                    cwd=root, env=env, stdin=subprocess.DEVNULL, capture_output=True,
                    timeout=120,
                )
                requests = []
                if server:
                    server.close()
                    requests = server.requests
                    assert not server.replies, (case.name, n, "replies left over")
                    for request in requests:
                        request["body"] = plain(request["body"])
                names, files = [], []
                for path in sorted(p for p in root.rglob("*") if p.is_file()):
                    name = str(path.relative_to(root))
                    names.append(name)
                    data = path.read_bytes()
                    if given.get(name) != data:
                        files.append({"path": name, "text": plain(data.decode("utf-8"))})
                        given[name] = data
                calls = root / "bin" / "calls"
                if calls.exists():
                    calls.unlink()
                    given.pop("bin/calls", None)
                steps.append({
                    **step,
                    "exit": ran.returncode,
                    "stdout": plain(ran.stdout.decode("utf-8")),
                    "stderr": plain(ran.stderr.decode("utf-8")),
                    "names": names,
                    "files": files,
                    "requests": requests,
                })
        self.cases.append({"name": case.name, "steps": steps})


SECRET_TASK = "sk-ant-planted"
SECRET_RESULT = "ghp_plantedtoken"


def label_cases(rec: Recorder) -> None:
    # The cases of `test_judge.py`, in its order.
    case = rec.case("the prompt carries the task, the result and the diff stat")
    case.rows(row("a", task="rename the flag"), shows=False)
    case.put("bin/table/a", shown("renamed it", {"files": 2, "insertions": 5}))
    case.step("label", replies=[says(FINE)]).done()

    case = rec.case("texts are redacted and cut with a marker")
    case.rows(
        row("a", task="t" * 3500 + " " + SECRET_TASK),
        row("b", task="use " + SECRET_TASK),
        # The cut counts code points: one of the three faces is kept. Counted in UTF-16
        # units none would be, and in bytes the cut would fall inside the accents.
        row("c", task="é" * 2999 + "\U0001f600" * 3),
        row("e", task="x" * 2990 + " " + SECRET_TASK + " and more"),
        shows=False,
    )
    case.put("bin/table/a", shown("r" * 5000 + " " + SECRET_RESULT))
    case.put("bin/table/b", shown("key " + SECRET_RESULT))
    case.put("bin/table/c", shown("\U0001f600" + "r" * 3998 + "é\U0001f600"))
    case.put("bin/table/e", shown("fine", {"note": SECRET_RESULT + " in a stat"}))
    case.step("label", replies=[says(GOOD)] * 4).done()

    case = rec.case("one line per run and a second pass appends nothing")
    case.rows(row("a"), row("b"))
    case.step("label", replies=[says(GOOD), says(FINE)])
    case.step("label").done()

    case = rec.case("a new model or prompt labels again")
    case.rows(row("a"))
    case.step("label", replies=[says(GOOD)])
    case.step("label", replies=[says(GOOD)], env={**ENV, "HYDRA_LOCAL_MODEL": "other"})
    case.rows(row("a", task="a different task"))
    case.step("label", replies=[says(GOOD)])
    case.put("bin/table/a", shown("done differently"))
    case.step("label", replies=[says(GOOD)])
    case.put("bin/table/a", shown("done differently", {"files": 1}))
    case.step("label", replies=[says(GOOD)])
    case.step("label").done()

    case = rec.case("unparsed answers are kept, cut and counted")
    case.rows(*(row(ident) for ident in "abcdefghij"))
    case.step("label", replies=[
        says("not json at all " + "x" * 600),
        says({"needs_correction": "yes", "failure_class": "agent", "reason": "r"}),
        says({"needs_correction": True, "failure_class": "weather", "reason": "r"}),
        says(GOOD),
        says({"reason": 7, "result_ok": True}),
        says({"reason": "r", "result_ok": 1, "needs_correction": False, "failure_class": "none"}),
        says([1, 2]),
        says("é" * 501),
        says({"needs_correction": True, "failure_class": "task", "reason": "\ud800 lone"}),
        says({"reason": "both", "result_ok": False, "needs_correction": False, "failure_class": "x"}),
    ]).done()

    case = rec.case("rate limited and gated rows are not sent")
    case.rows(row("a", outcome="rate_limited"), row("b", outcome="gated"), row("c"))
    case.step("label", replies=[says(GOOD)])
    case.step("label").done()

    case = rec.case("the server down stops the pass with one sentence")
    case.rows(row("r", outcome="gated"), row("a"), row("b"))
    case.step("label", server="down").done()

    case = rec.case("a model that is not pulled keeps the lines already written")
    case.rows(row("a"), row("b"), row("c"))
    case.step("label", replies=[says(GOOD), http(404, b'{"error": "x"}', "Not Found")]).done()

    case = rec.case("limit and only unlabelled")
    case.rows(row("a", verdict="fixed"), row("b"), row("c"), row("d"))
    case.step("label", "--only-unlabelled", "--limit", "2", replies=[says(GOOD)] * 2)
    case.step("label", "--only-unlabelled", replies=[says(GOOD)])
    case.step("label", "--limit", "0")
    case.step("label", "--limit", "-1")
    case.step("label", "--limit", str(10**50), replies=[says(GOOD)])
    case.step("label", "--limit", "-" + str(10**50)).done()

    case = rec.case("a limit counts the rule's lines and not the skipped ones")
    case.rows(row("a", outcome="gated"), row("b"), row("c"), row("d", outcome="rate_limited"))
    case.step("label", "--lim=2", replies=[says(GOOD)])
    case.step("label", "--limit", "1", replies=[says(GOOD)])
    case.step("label").done()

    case = rec.case("a run whose show fails is still judged on its task")
    case.rows(row("a"), shows=False)
    case.step("label", replies=[says(GOOD)]).done()


def reply_cases(rec: Recorder) -> None:
    def one(name, reply, rows=None):
        case = rec.case(name)
        case.rows(*(rows or [row("a"), row("b")]))
        case.step("label", replies=[reply])
        return case

    def two(name, reply):
        case = rec.case(name)
        case.rows(row("a"), row("b"))
        case.step("label", replies=[reply, says(GOOD)]).done()

    one("a 500 is the status", http(500, b"boom", "Internal Server Error")).done()
    one("a 302 with nowhere to go is the status", http(302, b"", "Found")).done()
    one("a 304 is the status", http(304, b"", "Not Modified")).done()
    one("a server that hangs up is not answering", b"").done()
    one("a line that is no status line is raised", b"SSH-2.0-OpenSSH_9.9\r\n").done()
    one("a body shorter than announced is raised",
        http(body=b"abc", headers=["Content-Length: 50"], version="HTTP/1.1")).done()

    body = json.dumps({"response": json.dumps(FINE), "done": True}).encode()
    two("a body of the announced length is read",
        http(body=body + b"ignored", headers=[f"Content-Length: {len(body)}"], version="HTTP/1.1"))
    chunks = b"".join(
        f"{len(part):x}\r\n".encode() + part + b"\r\n" for part in (body[:7], body[7:40], body[40:])
    ) + b"0\r\n\r\n"
    two("a chunked body is read",
        http(body=chunks, headers=["Transfer-Encoding: chunked"], version="HTTP/1.1"))
    two("a 100 before the reply is passed over",
        b"HTTP/1.1 100 Continue\r\n\r\n" + says(FINE))
    two("a 201 is an answer", http(201, body, "Created"))
    two("a 204 has no body", http(204, b"", "No Content", version="HTTP/1.1"))

    two("an empty body is unparsed", whole(b""))
    two("a reply that is a list is its own text", whole(b"[1]"))
    two("a reply with no response is its own text", whole(b'{"done": true}'))
    two("a reply that is the answer is read as one", whole(json.dumps(FINE).encode()))
    two("a reply that is not UTF-8 is read with replacements",
        whole(b"\xff\xfe\xe2\x82 abc \xc3\x28 \xed\xa0\x80 \xf4\x90\x80\x80 \xf0\x9f"))
    two("a response that is a list is cut as a list", whole(b'{"response": [1, "two", {"3": 4}]}'))
    one("a response that is a number is raised", whole(b'{"response": 5}')).done()
    one("a response that is null is raised", whole(b'{"response": null}')).done()
    one("a response that is an object is raised", whole(b'{"response": {"result_ok": true}}')).done()
    one("a response that is true is raised", whole(b'{"response": true}')).done()
    one("a reply with too long an integer is raised", whole(b"1" * 5000)).done()
    one("a response with too long an integer is raised", says("1" * 5000)).done()

    case = rec.case("a model named rule keeps no unparsed text")
    case.rows(row("a"), row("b", outcome="gated"))
    env = {**ENV, "HYDRA_LOCAL_MODEL": "rule"}
    case.step("label", replies=[says("not json")], env=env)
    case.step("label", env=env).done()

    case = rec.case("a model with no name is sent as nothing")
    case.rows(row("a"))
    case.step("label", replies=[says(FINE)], env={**ENV, "HYDRA_LOCAL_MODEL": ""}).done()

    case = rec.case("a model name that is not ASCII")
    case.rows(row("a"), row("b"))
    case.step("label", replies=[says(FINE), http(404, b"", "Not Found")],
              env={**ENV, "HYDRA_LOCAL_MODEL": "modèle \"q\""}).done()

    case = rec.case("a URL with a query and a fragment")
    case.rows(row("a"))
    case.step("label", replies=[says(FINE)],
              env={**ENV, "HYDRA_LOCAL_LLM_URL": f"http://127.0.0.1:{PORT}/v1/x?keep=1#frag"}).done()

    case = rec.case("a URL with no path")
    case.rows(row("a"))
    case.step("label", replies=[says(FINE)],
              env={**ENV, "HYDRA_LOCAL_LLM_URL": f"http://127.0.0.1:{PORT}"}).done()


def show_cases(rec: Recorder) -> None:
    def table(name, data: bytes, reply=True, **more):
        case = rec.case(name)
        case.rows(row("a", **more), shows=False)
        case.put("bin/table/a", data=data)
        case.step("label", replies=[says(GOOD)] if reply else []).done()

    table("what show prints is not JSON", b"not json\n")
    table("what show prints has no projection",
          json.dumps({"result_text": "top level", "diff_stat": {"b": 1, "a": 2}}).encode())
    table("an empty projection is the answer itself",
          json.dumps({"projection": {}, "result_text": "beside it"}).encode())
    table("what show prints is a list", b"[1]\n", reply=False)
    table("what show prints is an empty list", b"[]\n", reply=False)
    table("a projection that is text", b'{"projection": "text"}\n', reply=False)
    table("a projection that is a list", b'{"projection": [1]}\n', reply=False)
    table("what show prints is not UTF-8", b'{"projection": {"result_text": "\xff"}}\n', reply=False)
    table("what show prints has too long an integer", b"1" * 5000, reply=False)
    table("what show prints ends its lines with carriage returns",
          b'{"projection":\r\n {"result_text": "done"}}\r\n')
    table("a result that is not text", shown(5).encode(), reply=False)
    table("a result that is empty", shown("").encode())
    table("a result that is null", shown(None).encode())
    table("a stat that is not an object", shown("done", [3, "ghp_inalist rest", None]).encode())
    table("a stat that is zero", shown("done", 0).encode())
    table("a stat that is text", shown("done", "12 files").encode())
    stat = {
        "z": 1, "a": {"y": [1.5, NAN, {"d": 1, "c": True}], "é": None, "B": -0.0},
        "Z": "ghp_secret tail", "ghp_inakey": 10**30, "\U0001f600": "é",
    }
    table("a stat is printed with its keys sorted", shown("done", stat).encode())
    table("a task that is not text", shown().encode(), reply=False, task=5)
    table("a task that is a list", shown().encode(), reply=False, task=["do"])
    table("a task that is null", shown().encode(), task=None)
    table("a task that is zero", shown().encode(), task=0)
    table("a task with a lone surrogate", shown().encode(), reply=False, task="\ud800 lone")
    table("a result with a lone surrogate", shown("\udfff lone").encode(), reply=False)

    case = rec.case("a row with no task")
    case.rows({"run_id": "a"}, shows=False)
    case.step("label", replies=[says(GOOD)]).done()

    case = rec.case("no HYDRA_BIN and no hydra to find")
    case.rows(row("a"))
    case.step("label", replies=[says(GOOD)], without=["HYDRA_BIN"]).done()

    case = rec.case("an empty HYDRA_BIN")
    case.rows(row("a"))
    case.step("label", replies=[says(GOOD)], env={**ENV, "HYDRA_BIN": ""}).done()

    case = rec.case("a hydra that cannot be run")
    case.rows(row("a"))
    case.step("label", replies=[says(GOOD)], env={**ENV, "HYDRA_BIN": f"{ROOT}/home/dataset.jsonl"}).done()

    case = rec.case("the root is named through a dot and a part that is not there")
    case.rows(row("a"))
    case.step("label", replies=[says(GOOD)],
              env={**ENV, "HYDRA_ROOT": "./missing/../root/./deeper/.."}).done()

    case = rec.case("no root is named")
    case.rows(row("a"))
    case.step("label", env={**ENV, "HOME": f"{ROOT}/house"},
              without=["HYDRA_ROOT", "HYDRA_GRADER_HOME"])
    case.put("house/hydra/grader/dataset.jsonl", jsonl([row("a"), row("b")]))
    case.step("label", replies=[says(GOOD), says(FINE)], env={**ENV, "HOME": f"{ROOT}/house"},
              without=["HYDRA_ROOT", "HYDRA_GRADER_HOME"]).done()


NAN = float("nan")


def shape_cases(rec: Recorder) -> None:
    def rows(name, *items, argv=("label",), replies=(), labels=None, again=False):
        case = rec.case(name)
        case.rows(*items)
        if labels is not None:
            case.put("home/judge-labels.jsonl", labels)
        case.step(*argv, replies=replies)
        if again:
            case.step(*argv)
        case.done()

    rows("an empty dataset")
    case = rec.case("no home yet")
    case.step("label", env={**ENV, "HYDRA_GRADER_HOME": "not/yet/there"}).done()
    case = rec.case("a home that is a file")
    case.put("home", "in the way")
    case.step("label").done()

    rows("a row with no run id", {"task_text": "t"})
    rows("a row that is a list", [])
    rows("a row that is a list, only unlabelled", [], argv=("label", "--only-unlabelled"))
    rows("a row that is text", "a")
    rows("a row that is null, after one that is fine", row("a"), None, replies=[says(GOOD)])
    rows("a run id that is a number", row(7))
    rows("a run id that is null", row(None))
    rows("a run id that is a list", row(["a"]))
    rows("a run id with a null byte", row("a\u0000b"))
    rows("a run id with a lone surrogate", row("\ud800"))
    rows("a run id that is empty", row(""), replies=[says(GOOD)])
    rows("a run id with a space and a dash", row("-a b"), replies=[says(GOOD)])
    rows("an outcome that is a list", row("a", outcome=["gated"]))
    rows("an outcome that is a number", row("a", outcome=1), replies=[says(GOOD)])
    rows("the rule takes a run id of any kind",
         row(7, outcome="gated"), row(7.0, outcome="gated"), row(True, outcome="rate_limited"),
         row(None, outcome="gated"), row("7", outcome="gated"), row(1.5, outcome="gated"),
         row(10**40, outcome="gated"), again=True)
    rows("the rule refuses a run id that is a list", row("a", outcome="gated"), row([], outcome="gated"))
    rows("the rule cannot write a run id that is not a number",
         row("a", outcome="gated"), row(NAN, outcome="gated"))
    rows("the rule cannot write an infinite run id to a new file", row(float("inf"), outcome="gated"))
    rows("a human verdict of any kind is a verdict",
         row("a", verdict=False), row("b", verdict=0), row("c", verdict=""), row("d", verdict=[]),
         row("e"), argv=("label", "--only-unlabelled"), replies=[says(GOOD)])

    line = {"run_id": "a", "judge_model": "stub-model", "prompt_sha256": "0" * 64}
    rows("a label line with no model", row("a"), labels=jsonl([{"run_id": "a", "prompt_sha256": None}]))
    rows("a label line with no sha", row("a"), labels=jsonl([{"run_id": "a", "judge_model": "m"}]))
    rows("a label line with a list for a model", row("a"), labels=jsonl([{**line, "judge_model": []}]))
    rows("a label line with an object for a run id", row("a"), labels=jsonl([{**line, "run_id": {}}]))
    rows("a label line that is a list", row("a"), labels=jsonl([[1]]))
    rows("a label line that is text", row("a"), labels=jsonl(["a"]))
    rows("a label line that is garbled", row("a"), labels='{"run_id": \n')
    rows("label lines of another model, another sha and odd kinds are not this one",
         row("a"), row("g", outcome="gated"),
         labels=jsonl([
             {**line, "judge_model": "other"}, line, {**line, "prompt_sha256": None},
             {"run_id": "g", "judge_model": "rule", "prompt_sha256": "x"},
             {"run_id": "g", "judge_model": "rule", "prompt_sha256": 0},
             {"run_id": 1, "judge_model": 2, "prompt_sha256": 3.5},
         ]), replies=[says(GOOD)])
    rows("a torn tail is appended to", row("a"), row("b", outcome="gated"),
         labels=jsonl([line]) + '{"run_id": "torn', replies=[says(GOOD)], again=True)

    case = rec.case("a dataset with a garbled line")
    case.put("home/dataset.jsonl", jsonl([row("a")]) + "not json\n")
    case.step("label").done()
    case = rec.case("a dataset with a torn tail and blank ends")
    case.put("home/dataset.jsonl", jsonl([row("a", outcome="gated")]) + '{"run_id": "torn')
    case.step("label").done()


def agree_cases(rec: Recorder) -> None:
    def judged(ident, need, model="m", **more):
        return {"run_id": ident, "needs_correction": need, "failure_class": "agent", "reason": "r",
                "judge_model": model, "prompt_sha256": "0" * 64,
                "judged_at": "2026-09-29T00:00:00Z", **more}

    def agree(name, rows, lines, labels=None):
        case = rec.case("agree: " + name)
        if rows is not None:
            case.put("home/dataset.jsonl", jsonl(rows))
        if labels is None and lines is not None:
            labels = jsonl(lines)
        if labels is not None:
            case.put("home/judge-labels.jsonl", labels)
        case.step("agree", server="down").done()

    plan = [("a", "fixed", True), ("b", "fixed", False), ("c", "applied", False),
            ("d", "applied", False), ("e", "applied", True), ("f", None, True), ("g", "fixed", None)]
    agree("agree counts and prints its table",
          [row(ident, verdict=verdict) for ident, verdict, _ in plan],
          [judged(ident, need, **({"unparsed": "junk"} if need is None else {})) for ident, _, need in plan])
    for count in (49, 50, 51):
        agree(f"agree at {count} runs",
              [row(f"r{n}", verdict="fixed") for n in range(count)],
              [{"run_id": f"r{n}", "needs_correction": True, "judge_model": "m"} for n in range(count)])
    # Ratios whose third decimal is a tie or near one.
    for agreed, both in ((1, 8), (3, 8), (5, 8), (7, 8), (1, 3), (2, 3), (1, 40), (3, 40), (39, 40), (0, 4)):
        agree(f"agree on {agreed} of {both}",
              [row(f"r{n}", verdict="applied") for n in range(both)],
              [judged(f"r{n}", n >= agreed) for n in range(both)])
    agree("agree with nothing", None, None)
    agree("agree with a dataset and no labels", [row("a", verdict="fixed")], None)
    agree("agree with labels and no dataset", None, [judged("a", True), judged("b", None)])
    agree("the rule's lines are not the judge's",
          [row("a", verdict="fixed"), row("b", verdict="applied")],
          [judged("a", True, model="rule"), judged("b", False), judged("b", None, model="rule"),
           {"run_id": "a", "needs_correction": None}])
    agree("the last line of a run is the one counted",
          [row("a", verdict="fixed"), row("b", verdict="applied"), row("a", verdict="applied")],
          [judged("a", True), judged("b", True), judged("a", False, model="other"), judged("b", None)])
    agree("verdicts and needs of other kinds are not counted",
          [row("a", verdict="Fixed"), row("b", verdict=1), row("c", verdict="fixed"),
           row("d", verdict="applied"), row("e", verdict=None), row(7, verdict="fixed"),
           row(8.0, verdict="applied")],
          [judged("a", True), judged("b", True), judged("c", 1), judged("d", "false"),
           judged("e", False), judged(7.0, True), judged(8, False), judged(True, True),
           judged("zz", False)])
    agree("a verdict that is a list", [row("a", verdict=["fixed"])], [judged("a", True)])
    agree("a verdict that is a list beside a need that is null", [row("a", verdict={})], [judged("a", None)])
    agree("a dataset row with no run id", [{"human_verdict": "fixed"}], [judged("a", True)])
    agree("a dataset row that is a list", [[1]], [judged("a", True)])
    agree("a dataset run id that is a list", [row([])], [judged("a", True)])
    agree("a label line with no run id", [row("a")], [{"needs_correction": True}])
    agree("a rule line with no run id", [row("a")], [{"judge_model": "rule"}])
    agree("a label line that is a list", [row("a")], [[1]])
    agree("a label run id that is a list", [row("a")], [judged([], True)])
    agree("a garbled label line", [row("a")], None, labels="not json\n")
    agree("a torn label tail", [row("a", verdict="fixed")], None,
          labels=jsonl([judged("a", True)]) + '{"run_id": "a", "needs_corr')


ARGV = [
    [], ["bad"], ["--help"], ["-h"], ["label", "--help"], ["label", "-h"], ["agree", "--help"],
    ["label", "extra"], ["label", "--limit"], ["label", "--limit", "x"], ["label", "--limit=1_0"],
    ["label", "--lim", "1"], ["label", "--l", "1"], ["label", "--only"], ["label", "--o"],
    ["label", "--only-unlabelled=1"], ["label", "--only-unlabelled="], ["label", "--", "x"],
    ["label", "--"], ["--foo", "label"], ["--", "label"], ["label", "--limit", "--", "1"],
    ["label", "--limit", "-1"], ["label", "--limit", "--only-unlabelled"],
    ["label", "--limit", "1", "--limit", "0"], ["label", "--limit", " ２ "],
    ["label", "--limit", "1.5"], ["label", "--limit", ""], ["label", "--limit="],
    ["label", "--only-unlabelled", "--only-unlabelled"], ["label", "--help=x"], ["label", "-x"],
    ["label", "-1"], ["label", "--", "--limit", "1"], ["label", "a b"], ["label", "--li", "3", "x", "y"],
    ["agree", "--limit", "1"], ["agree", "x"], ["agree", "--only-unlabelled"], ["agree", "--"],
    ["agree", "--help=x"], ["label", "agree"], ["agree", "label"], ["Label"], ["--limit", "1", "label"],
    ["--only-unlabelled", "label"], ["label", "--limit", "1" + "0" * 5000], ["label", "--", "--"],
    ["label", "--limit", "+3"], ["label", "--limit", "0x10"], ["label", "--unknown=1", "--limit", "1"],
]


def argv_cases(rec: Recorder) -> None:
    for argv in ARGV:
        case = rec.case("argv " + repr(argv))
        case.rows(row("a", outcome="gated"))
        case.step(*argv, server="down").done()


def record(out: Path, repo: Path) -> None:
    rec = Recorder(repo)
    label_cases(rec)
    reply_cases(rec)
    show_cases(rec)
    shape_cases(rec)
    agree_cases(rec)
    argv_cases(rec)
    value = {"version": sys.version, "stub": HYDRA_STUB, "cases": rec.cases}
    (out / "judge.json").write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")
    steps = sum(len(case["steps"]) for case in rec.cases)
    print(f"judge: {len(rec.cases)} cases, {steps} steps", file=sys.stderr)


if __name__ == "__main__":
    HERE = Path(__file__).resolve().parent
    record(Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else HERE / "fixtures", HERE.parents[2])
