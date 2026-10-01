"""The part of `record.py` that records `labels.py` and `grader.py` (build ticket 214).

`record.py` loads this file and calls `record`. It is not run on its own.

Two fixtures come out of it, both under `fixtures/collect/`.

`labels.json` is `labels.py` function by function. `show_goal`, `committed_events` and
`labels_for_goal` run on every goal log of `crates/puppets/tests/fixtures/read/`, copied here
with what `puppets.py show` printed for it, and on answers written by hand for each refusal.
`collect` and `collect_llm` run on small homes. `hydra-puppets` is a stub: a shell script that
appends its arguments and its environment to a file and prints the answer the case planted.

`scenarios.json` is `grader.py` command by command. A scenario is a scratch directory and a
list of steps. A step plants or removes files and then runs one command as a child, with
nothing in its environment but what the step names, from the scratch directory. What comes
back is the exit code, stdout, stderr and every file under the directory that was not planted,
which is the dataset, the cursor, the cache, the scores and what the two stubs were called
with. `hydra` is a stub too, answering `show <id> --events` from a table of files.

Three things are frozen. `records.load_aliases()` reads the frozen `aliases.json`. The clock:
`utc_now` returns a made-up second of 2001 that is different in every step, so a row says
which step wrote it. And the scratch directory's own path is written `<root>` wherever it was
printed or passed.

One step calls `collect()` with a runs directory that is not named `runs`, which no command
can do. There `hydra` is handed a scratch directory holding a link, and its path is written
`<tmp>`.
"""

import contextlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from unittest import mock

ROOT = "<root>"

# What a stub writes down before it answers: its arguments, the environment it was started
# with, and for `hydra` whether the record is where `HYDRA_ROOT` says. The shell adds `PWD`,
# `SHLVL` and `_` to its own environment, so those are left out. So are the two variables
# CPython adds to its own when it starts with no locale, which every child then inherits:
# `LC_CTYPE=C.UTF-8` (PEP 538) and, on macOS, `__CF_USER_TEXT_ENCODING`.
STUB_HEAD = r"""#!/bin/sh
here="${0%/*}"
note() {
  printf 'argv:'
  for arg in "$@"; do printf ' [%s]' "$arg"; done
  printf '\n'
  /usr/bin/env | /usr/bin/sort | while IFS= read -r line; do
    case "$line" in
      PWD=*|OLDPWD=*|SHLVL=*|_=*) ;;
      LC_CTYPE=C.UTF-8|__CF_USER_TEXT_ENCODING=*) ;;
      *) printf 'env: %s\n' "$line" ;;
    esac
  done
}
"""

HYDRA_STUB = STUB_HEAD + r"""{
  note "$@"
  if [ -f "$HYDRA_ROOT/runs/$2.jsonl" ]; then echo 'record: found'; else echo 'record: missing'; fi
} >> "$here/calls"
if [ -f "$here/table/$2.kill" ]; then kill -9 $$; fi
if [ -f "$here/table/$2.exit" ]; then
  read -r code < "$here/table/$2.exit"
  echo "stub hydra: no answer for $2" >&2
  exit "$code"
fi
exec /bin/cat "$here/table/$2"
"""

PUPPETS_STUB = STUB_HEAD + r"""note "$@" >> "$here/puppets-calls"
if [ -f "$here/shows/$3.kill" ]; then kill -9 $$; fi
if [ -f "$here/shows/$3.err" ]; then /bin/cat "$here/shows/$3.err" >&2; fi
if [ -f "$here/shows/$3.exit" ]; then
  read -r code < "$here/shows/$3.exit"
  exit "$code"
fi
exec /bin/cat "$here/shows/$3"
"""

STUBS = {"hydra": HYDRA_STUB, "puppets": PUPPETS_STUB}

# The child: `grader.py` loaded as a module, its alias table and its clock frozen, then its
# `main` run as the script would run it, or `collect` called with the arguments given.
CHILD = """
import importlib.util, json, sys
from pathlib import Path
frozen, script, now, mode = sys.argv[1:5]
rest = sys.argv[5:]
sys.argv = ["grader.py", *rest]
spec = importlib.util.spec_from_file_location("recorded_grader", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
load = module.records.load_aliases
module.records.load_aliases = lambda path=None: load(frozen)
module.utc_now = lambda: now
if mode == "lib":
    kw = json.loads(rest[0])
    print(module.collect(
        runs=Path(kw["runs"]), home=Path(kw["home"]), hydra=kw["hydra"],
        jobs=kw["jobs"], rebuild=kw["rebuild"],
    ))
    raise SystemExit(0)
raise SystemExit(module.main())
"""


def hexed(data) -> str:
    return (data.encode("utf-8") if isinstance(data, str) else data).hex()


def dump_lines(*rows) -> bytes:
    return "".join(json.dumps(row) + "\n" for row in rows).encode()


def blob(value: dict) -> bytes:
    return value["text"].encode("utf-8") if "text" in value else bytes.fromhex(value["hex"])


def plant(root: Path, spec: dict, out: Path, models: dict) -> bytes:
    """Write one file of a case and return its bytes."""
    path = root / spec["path"]
    path.parent.mkdir(parents=True, exist_ok=True)
    if "stub" in spec:
        data = STUBS[spec["stub"]].encode()
    elif "model" in spec:
        data = models[spec["model"]].encode()
    elif "aliases" in spec:
        data = (out / "data" / "aliases.json").read_bytes()
    else:
        data = bytes.fromhex(spec["hex"])
    path.write_bytes(data)
    path.chmod(0o755 if spec.get("exec") else 0o644)
    return data


def templated(env: dict, root: Path) -> dict:
    return {key: value.replace(ROOT, str(root)) for key, value in env.items()}


def plain(text: str, root: Path) -> str:
    return text.replace(str(root), ROOT)


# ----------------------------------------------------------------------------- labels


def task(ident, cls="build", grade="low", text="do it", run_id=None, human_result=None, **more) -> dict:
    return {
        "id": ident, "class": cls, "grade": grade, "text": text, "run_id": run_id,
        "human_result": human_result, "attempts": 1, **more,
    }


def state(goal_id, tasks=(), plans=(), text="ship it", raw=None) -> bytes:
    """What `show` prints. `raw` replaces a key with any value, or with `...` drops it."""
    shown = {
        "goal_id": goal_id,
        "goal": {"text": text, "repo": "/repo"},
        "revision": 0,
        "plans": list(plans),
        "tasks": {str(t["id"]): t for t in tasks},
        "approvals": [],
    }
    for key, value in (raw or {}).items():
        if value is ...:
            del shown[key]
        else:
            shown[key] = value
    return (json.dumps(shown, indent=2) + "\n").encode()


def log(*events) -> bytes:
    return dump_lines(*({"seq": n + 1, **event} for n, event in enumerate(events)))


def answered(goal: str, stdout: bytes, log_bytes=None, code=None, stderr=None, kill=False) -> list[dict]:
    files = [{"path": f"bin/shows/{goal}", "hex": hexed(stdout)}]
    if code is not None:
        files.append({"path": f"bin/shows/{goal}.exit", "hex": hexed(f"{code}\n")})
    if stderr is not None:
        files.append({"path": f"bin/shows/{goal}.err", "hex": hexed(stderr)})
    if kill:
        files.append({"path": f"bin/shows/{goal}.kill", "hex": ""})
    if log_bytes is not None:
        files.append({"path": f"puppets/{goal}.jsonl", "hex": hexed(log_bytes)})
    return files


HELPER = {"path": "bin/hydra-puppets", "stub": "puppets", "exec": True}
HELPER_ENV = {"HYDRA_PUPPETS_BIN": f"{ROOT}/bin/hydra-puppets"}


def shows() -> list[dict]:
    """Answers written by hand: every refusal of `show_goal`, and the shapes of a state and
    of a log that `labels_for_goal` reads without `show` having checked them."""
    t1 = task("t1", run_id="run-a", human_result="applied")
    t2 = task("t2", "security_review", "high", "audit the gate", "run-c")
    cases = []

    def case(name, stdout, log_bytes=b"", env=None, helper=HELPER, **how):
        files = answered("g", stdout, log_bytes, **how)
        if helper is not None:
            files.append(helper)
        cases.append({"name": name, "goal": "g", "files": files, "env": HELPER_ENV if env is None else env})

    case("two tasks and a plan", state("g", [t1, t2], [{"run_id": "run-p", "provider": "claude"}]))
    case("a goal with no log", state("g", raw={"goal": None}), None)
    case("a state that is a list", b"[1]\n")
    case("a state that is text", b'"g"\n')
    case("a state that is null", b"null\n")
    for key in ("goal", "tasks", "plans"):
        case(f"a state with no {key}", state("g", raw={key: ...}))
    case("a state about another goal", state("other"))
    case("a state with no goal id", state("g", raw={"goal_id": ...}))
    case("a state whose goal id is a number", state("g", raw={"goal_id": 7}))
    case("a state whose goal id is a list", state("g", raw={"goal_id": ["g", "café", "\ud800"]}))
    case("a goal that is a number", state("g", raw={"goal": 7}))
    case("a goal that is an empty object", state("g", raw={"goal": {}}))
    case("a goal that is a list", state("g", raw={"goal": ["text"]}))
    case("a goal whose text is a number", state("g", [t1], [{"run_id": "run-p"}], raw={"goal": {"text": 7}}))
    case("a goal whose text is null", state("g", plans=[{"run_id": "run-p"}], raw={"goal": {"text": None}}))
    case("tasks that are a list", state("g", raw={"tasks": [t1]}))
    case("tasks that are null", state("g", raw={"tasks": None}))
    case("a task that is a list", state("g", raw={"tasks": {"t1": ["id"]}}))
    for field in ("id", "class", "grade", "text", "run_id", "human_result"):
        lacking = {k: v for k, v in t1.items() if k != field}
        case(f"a task with no {field}", state("g", raw={"tasks": {"t1": lacking}}))
    case("plans that are an object", state("g", raw={"plans": {"run_id": "x"}}))
    case("plans that are null", state("g", raw={"plans": None}))
    case("a plan that is text", state("g", raw={"plans": ["run-p"]}))
    case("a plan with no run id", state("g", plans=[{"run_id": "run-p"}, {"provider": "claude"}]))
    case("a plan whose run id is null", state("g", plans=[{"run_id": None}, {"run_id": 7}, {"run_id": 7.0}]))
    case("a plan whose run id is a list", state("g", plans=[{"run_id": ["x"]}]))
    case("a task whose run id is a list", state("g", [task("t1", run_id=["x"])]))
    case("a task whose run id is an object", state("g", [task("t1", run_id={"x": 1})]))
    case(
        "tasks with no run, or one that is not text",
        state("g", [task("t1"), task("t2", run_id=""), task("t3", run_id=0), task("t4", run_id=12), task("t5", run_id=True)]),
    )
    case("a task whose id is a list", state("g", raw={"tasks": {"x": task(["t1"], run_id="run-a")}}))
    # `show` prints a task under `str(id)`: the ids 3 and "3" are one key twice.
    twice = (
        b'{"goal_id": "g", "goal": {"text": "ship"}, "plans": [], "tasks": {'
        + b'"3": ' + json.dumps(task(3, run_id="run-int", text="by number")).encode()
        + b', "3": ' + json.dumps(task("3", run_id="run-str", text="by text")).encode()
        + b', "3.0": ' + json.dumps(task(3.0, run_id="run-float", text="by float")).encode()
        + b', "True": ' + json.dumps(task(True, run_id="run-true")).encode()
        + b', "1": ' + json.dumps(task(1, run_id="run-one", human_result="fixed")).encode()
        + b', "NaN": ' + json.dumps(task(float("nan"), run_id="run-nan-1")).encode()
        + b', "NaN": ' + json.dumps(task(float("nan"), run_id="run-nan-2")).encode()
        + b"}}\n"
    )
    case(
        "ids that are one key twice, and reviews of each",
        twice,
        log(
            {"event": "review", "task": 3, "judge_run_ids": ["judge-3"]},
            {"event": "review", "task": "3", "judge_run_ids": ["judge-text"]},
            {"event": "review", "task": 1.0, "judge_run_ids": ["judge-one"]},
            {"event": "review", "task": float("nan"), "judge_run_ids": ["judge-nan"]},
            {"event": "review", "task": None, "judge_run_ids": ["judge-none"]},
            {"event": "review", "judge_run_ids": ["judge-absent"]},
        ),
    )
    case(
        "a state with tasks written twice",
        b'{"goal_id": "x", "goal_id": "g", "goal": 7, "goal": null, "tasks": [], "plans": [], "tasks": {"t1": '
        + json.dumps(t1).encode() + b', "t1": ' + json.dumps(dict(t1, run_id="run-b", run_id2=1)).encode() + b"}}\n",
    )
    case(
        "a task with a field written twice",
        b'{"goal_id": "g", "goal": null, "plans": [], "tasks": {"t1": {"id": "t1", "class": "a", "class": "b", '
        b'"grade": null, "text": "x", "run_id": "run-a", "run_id": "run-z", "human_result": null}}}\n',
    )

    judged = state("g", [t1, t2], [{"run_id": "run-p"}])
    case(
        "judges of the plan and of each task",
        judged,
        log(
            {"event": "goal", "text": "ship it"},
            {"event": "panel_verdict", "judge_run_ids": ["judge-1", "judge-2"]},
            {"event": "review", "task": "t1", "judge_run_ids": ["judge-3", "judge-1"]},
            {"event": "review", "task": "nobody", "judge_run_ids": ["judge-4"]},
            {"event": "review", "task": "t2"},
            {"event": "panel_verdict"},
            {"event": "review", "task": "t2", "judge_run_ids": ["run-a"]},
        ),
    )
    case("judges that are text", judged, log({"event": "panel_verdict", "judge_run_ids": "ab\U0001f600"}))
    case("judges that are an object", judged, log({"event": "review", "task": "t1", "judge_run_ids": {"k1": 1, "k2": 2}}))
    case("judges that are a number", judged, log({"event": "panel_verdict", "judge_run_ids": 7}))
    case("judges that are null", judged, log({"event": "review", "task": "t1", "judge_run_ids": None}))
    case("a judge that is a list", judged, log({"event": "panel_verdict", "judge_run_ids": [["x"]]}))
    case("judges that are numbers", judged, log({"event": "panel_verdict", "judge_run_ids": [1, 1.0, True, None, "1"]}))
    case("a review of a task that is a list", judged, log({"event": "review", "task": ["t1"], "judge_run_ids": ["j"]}))
    case("a review of a task that is a list, with no judges", judged, log({"event": "review", "task": ["t1"]}))
    case("an event that is a list", judged, b"[1, 2]\n")
    case("an event that is text", judged, b'"review"\n')
    case("an event whose name is a list", judged, log({"event": ["review"], "judge_run_ids": ["j"]}))
    review = json.dumps({"event": "review", "task": "t1", "judge_run_ids": ["judge-torn"]}).encode()
    case("a log with a torn tail", judged, log({"event": "panel_verdict", "judge_run_ids": ["j"]}) + review)
    case("a log with a carriage return before the newline", judged, review + b"\r\n")
    case("a log with a blank line", judged, review + b"\n\n")
    case("a log with a garbled line", judged, review + b"\nnot json\n")
    case("a log that is not utf-8", judged, b'{"event": "caf\xe9"}\n')
    case("a log in utf-16", judged, (review.decode() + "\n").encode("utf-16-le"))
    case("no log at all", judged, None)

    case("an answer that is not json", b"not json\n")
    case("an answer that is empty", b"")
    case("an answer that is not utf-8", b'{"goal_id": "caf\xe9"}\n')
    case("an answer with more after it", state("g") + b"{}\n")
    case("an answer in utf-16", state("g", [t1]).decode().encode("utf-16"))
    case("an answer with a NaN", state("g", [task("t1", run_id="run-a", grade=float("nan"))]))
    case("a helper that exits 3", b"", code=3, stderr=b"  hydra-puppets: g: line 2 is not JSON\n\n")
    case("a helper that exits 1 and says nothing", state("g"), code=1)
    case("a helper that exits 2 and says something that is not utf-8", b"", code=2, stderr=b"caf\xe9 \xff\xfe\n")
    case("a helper that exits 0 and says something", state("g", [t1]), code=0, stderr=b"a warning\n")
    case("a helper that is killed", b"", kill=True)
    case("no helper", state("g"), helper=None)
    case("a helper that cannot be run", state("g"), helper={"path": "bin/hydra-puppets", "stub": "puppets"})
    case("a helper that is a directory", state("g"), helper={"path": "bin/hydra-puppets/inside", "hex": ""})
    case("a helper named by a relative path", state("g"), env={"HYDRA_PUPPETS_BIN": "bin/hydra-puppets"})
    case("a helper beside hydra", state("g", [t1]), env={"HYDRA_BIN": f"{ROOT}/bin/hydra", "PATH": "/nowhere"})
    case(
        "a helper named, and another beside hydra",
        state("g", [t1]),
        env={"HYDRA_PUPPETS_BIN": f"{ROOT}/bin//./hydra-puppets", "HYDRA_BIN": "/nowhere/hydra", "EXTRA": "kept"},
    )
    return cases


def record_labels(out: Path, repo: Path, labels) -> None:
    cases = []
    for path in sorted((repo / "crates/puppets/tests/fixtures/read").glob("*.json")):
        recorded = json.loads(path.read_text(encoding="utf-8"))
        goal = recorded["goal"]
        first = recorded["steps"][0]
        assert first["args"] == ["show", goal], path.name
        files = [{"path": file["path"], "hex": hexed(blob(file["bytes"]))} for file in recorded["seed"]]
        files += answered(
            goal,
            blob(first["stdout"]),
            code=first["code"] or None,
            stderr=blob(first["stderr"]) or None,
        )
        files.append(HELPER)
        cases.append({
            "name": f"{path.stem}: {recorded['label']}", "goal": goal, "files": files,
            "env": HELPER_ENV, "fold": recorded["labels"],
        })
    assert len(cases) > 150, len(cases)
    cases += shows()

    for case in cases:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            for spec in case["files"]:
                plant(root, spec, out, {})
            home = root / "puppets"
            goal = case["goal"]
            with mock.patch.dict(os.environ, templated(case["env"], root), clear=True):
                try:
                    shown = labels.show_goal(goal, home)
                    found = labels.labels_for_goal(shown, labels.committed_events(home / f"{goal}.jsonl"))
                    # Every pair in order, as text, so NaN equals NaN.
                    result = {"pairs": json.dumps([[run_id, row] for run_id, row in found.items()])}
                except Exception as err:  # noqa: BLE001
                    result = {"raises": type(err).__name__}
                    if isinstance(err, labels.LabelsError):
                        result["message"] = plain(str(err), root)
            calls = root / "bin" / "puppets-calls"
            result["calls"] = plain(calls.read_text(), root) if calls.exists() else None
        fold = case.pop("fold", None)
        if fold is not None and "pairs" in result:
            # What the Python fold gave before `labels.py` asked the helper (build 196).
            assert fold == {"pairs": result["pairs"]}, case["name"]
        case["result"] = result

    # `collect`: every goal log of a home, in the order of the names.
    t1 = task("t1", run_id="run-a", human_result="applied")
    homes = []

    def home_case(name, files, env=None):
        homes.append({"name": name, "files": files + [HELPER], "env": dict(HELPER_ENV, **(env or {"HYDRA_ROOT": ROOT}))})

    both = (
        answered("b", state("b", [task("t1", run_id="run-a", text="from b")]), b"")
        + answered("a", state("a", [t1], [{"run_id": "run-p"}]), log({"event": "panel_verdict", "judge_run_ids": ["j"]}))
        + answered(".real", state(".real", [task("t9", run_id="run-dot")]), b"")
        + answered("A.jsonl", state("A.jsonl", [task("t1", run_id="run-twice")]), b"")
    )
    extra = [
        {"path": "puppets/.jsonl", "hex": hexed(b'{"event": "goal"}\n')},
        {"path": "puppets/notes.txt", "hex": hexed(b"not a log\n")},
        {"path": "puppets/a.jsonl.tmp", "hex": ""},
        {"path": "puppets/deep/c.jsonl", "hex": ""},
    ]
    home_case("four goals and a file with no goal id", both + extra)
    home_case(
        "a home named on its own",
        [dict(f, path=f["path"].replace("puppets/", "else/where/")) for f in both],
        env={"HYDRA_PUPPETS_HOME": f"{ROOT}/else//where/", "HYDRA_ROOT": "/nowhere"},
    )
    home_case("a home under the root, named empty", both, env={"HYDRA_PUPPETS_HOME": "", "HYDRA_ROOT": f"{ROOT}/"})
    home_case("no home", [])
    home_case("a home that is a file", [{"path": "puppets", "hex": hexed(b"x")}])
    home_case(
        "a goal that is a directory",
        answered("d", state("d", [t1])) + [{"path": "puppets/d.jsonl/inside", "hex": ""}],
    )
    home_case(
        "the second goal is refused",
        answered("a", state("a", [t1]), b"") + answered("b", b"", b"", code=3, stderr=b"damaged\n")
        + answered("c", state("c"), b""),
    )
    home_case(
        "the helper has no answer for a goal",
        answered("a", state("a", [t1]), b"") + [{"path": "puppets/b.jsonl", "hex": ""}],
    )
    home_case(
        "a log the helper showed and Python cannot read",
        answered("a", state("a", [t1]), b"not json\n") + answered("b", state("b"), b""),
    )
    for case in homes:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            for spec in case["files"]:
                plant(root, spec, out, {})
            said = io.StringIO()
            with mock.patch.dict(os.environ, templated(case["env"], root), clear=True):
                with contextlib.redirect_stderr(said):
                    try:
                        found = labels.collect()
                        result = {"pairs": json.dumps([[run_id, row] for run_id, row in found.items()])}
                    except Exception as err:  # noqa: BLE001
                        result = {"raises": type(err).__name__}
                        if isinstance(err, labels.LabelsError):
                            result["message"] = plain(str(err), root)
            calls = root / "bin" / "puppets-calls"
            result["calls"] = plain(calls.read_text(), root) if calls.exists() else None
            result["stderr"] = plain(said.getvalue(), root)
        case["result"] = result

    # `collect_llm`: the class files a model wrote.
    one = {"run_id": "run-a", "task_class": "build", "grade": "low", "area": "rust", "kind": "port"}
    llm = []

    def llm_case(name, files: dict):
        llm.append({"name": name, "files": [{"path": f"home/{path}", "hex": hexed(data)} for path, data in files.items()]})

    llm_case("no home", {})
    llm_case(
        "two files, the later name wins",
        {
            "task-classes-2026-09-29.jsonl": dump_lines(one, {"run_id": "run-b", "task_class": "review"}),
            "task-classes-2026-09-30.jsonl": dump_lines(dict(one, task_class="fix", unsure=True)),
            "task-classes-.jsonl": dump_lines({"run_id": "run-first", "task_class": "bare"}),
            "task-classes.jsonl": dump_lines({"run_id": "run-no", "task_class": "not matched"}),
            "task-classes-x.json": dump_lines({"run_id": "run-no", "task_class": "not matched"}),
            "Task-classes-x.jsonl": dump_lines({"run_id": "run-no", "task_class": "not matched"}),
            "models/task-classes-y.jsonl": dump_lines({"run_id": "run-no", "task_class": "not matched"}),
        },
    )
    llm_case(
        "lines that are skipped",
        {
            "task-classes-a.jsonl": b"\n   \n\t\nnot json\n[1, 2]\n\"text\"\n7\nnull\n{}\n"
            + dump_lines({"run_id": ""}, {"run_id": None}, {"run_id": 0}, {"task_class": "no id"})
            + dump_lines(one)
            + b'{"run_id": "run-torn", "task_class": "kept without a newline"}',
        },
    )
    seps = "\r\n", "\r", "\x0b", "\x0c", "\x1c", "\x1d", "\x1e", "\x85", " ", " ", "\x1f", " ", "\t"
    llm_case(
        "every character that ends a line of text",
        {
            "task-classes-a.jsonl": "".join(
                json.dumps({"run_id": f"run-{n}", "task_class": "x"}) + sep for n, sep in enumerate(seps)
            ).encode("utf-8"),
        },
    )
    llm_case(
        "run ids that are not text",
        {"task-classes-a.jsonl": dump_lines({"run_id": 3, "v": 1}, {"run_id": 3.0, "v": 2}, {"run_id": True, "v": 3}, {"run_id": "3", "v": 4})},
    )
    llm_case("a run id that is a list", {"task-classes-a.jsonl": dump_lines(one, {"run_id": ["x"]})})
    llm_case("a run id that is an object", {"task-classes-a.jsonl": dump_lines({"run_id": {"x": 1}})})
    llm_case("a file that is not utf-8", {"task-classes-a.jsonl": dump_lines(one) + b'{"run_id": "caf\xe9"}\n'})
    llm_case("a file with a byte order mark", {"task-classes-a.jsonl": b"\xef\xbb\xbf" + dump_lines(one, dict(one, run_id="run-b"))})
    llm_case("a class file that is a directory", {"task-classes-a.jsonl/inside": b""})
    llm_case("non-ascii text and a NaN", {"task-classes-a.jsonl": '{"run_id": "café \U0001f600", "grade": NaN, "task_class": "señal"}\n'.encode()})
    for case in llm:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            for spec in case["files"]:
                plant(root, spec, out, {})
            try:
                found = labels.collect_llm(root / "home")
                result = {"pairs": json.dumps([[run_id, row] for run_id, row in found.items()])}
            except Exception as err:  # noqa: BLE001
                result = {"raises": type(err).__name__}
        case["result"] = result

    # Where the helper and the homes are, by environment.
    places = []
    for env in [
        {"HOME": "/users/x"},
        {"HOME": "/users/x", "HYDRA_ROOT": "/named/root"},
        {"HOME": "/users/x", "HYDRA_ROOT": "relative/./root//"},
        {"HOME": "/users/x", "HYDRA_ROOT": "", "HYDRA_PUPPETS_HOME": ""},
        {"HOME": "/users/x", "HYDRA_ROOT": "/named/root", "HYDRA_PUPPETS_HOME": "/named/puppets"},
        {"HOME": "/users/x", "HYDRA_PUPPETS_HOME": "relative/puppets/"},
        {"HOME": "/users/x", "HYDRA_PUPPETS_BIN": "/opt/x//bin/./hydra-puppets/"},
        {"HOME": "/users/x", "HYDRA_PUPPETS_BIN": "/opt/../helper", "HYDRA_BIN": "/opt/bin/hydra"},
        {"HOME": "/users/x", "HYDRA_PUPPETS_BIN": "helper"},
        {"HOME": "/users/x", "HYDRA_PUPPETS_BIN": "./bin/helper", "HYDRA_BIN": "/opt/bin/hydra"},
        {"HOME": "/users/x", "HYDRA_PUPPETS_BIN": "", "HYDRA_BIN": "/opt/bin/hydra"},
        {"HOME": "/users/x", "HYDRA_BIN": "/opt//bin/./hydra/"},
        {"HOME": "/users/x", "HYDRA_BIN": "/hydra"},
        {"HOME": "/users/x", "HYDRA_BIN": "/"},
        {"HOME": "/users/x", "HYDRA_BIN": "/opt/bin/../hydra"},
        {"HOME": "/users/x", "HYDRA_BIN": "hydra"},
        {"HOME": "/users/x", "HYDRA_BIN": "bin/hydra"},
        {"HOME": "/users/x", "HYDRA_BIN": ""},
    ]:
        with mock.patch.dict(os.environ, env, clear=True):
            try:
                helper = {"path": str(labels.helper_bin())}
                # With nothing configured the helper is the checkout's release build, and the
                # checkout is wherever the recorder ran.
                if helper["path"] == str(repo / "target" / "release" / "hydra-puppets"):
                    helper = {"checkout": True}
            except labels.LabelsError as err:
                helper = {"message": str(err)}
            places.append({"env": env, "puppets_home": str(labels.puppets_home()), "helper": helper})

    (out / "collect").mkdir(exist_ok=True)
    write(out / "collect" / "labels.json", {
        "stubs": STUBS, "shows": cases, "homes": homes, "llm": llm, "places": places,
    })


def readable(value):
    """The fixture with a planted file's bytes as text where they are text, which is most of
    them and half the size."""
    if isinstance(value, list):
        return [readable(item) for item in value]
    if not isinstance(value, dict):
        return value
    out = {key: readable(item) for key, item in value.items()}
    if "path" in out and isinstance(out.get("hex"), str):
        try:
            text = bytes.fromhex(out["hex"]).decode("utf-8")
        except UnicodeDecodeError:
            return out
        del out["hex"]
        out["text"] = text
    return out


def write(path: Path, value) -> None:
    path.write_text(json.dumps(readable(value), indent=1) + "\n", encoding="utf-8")


# --------------------------------------------------------------------------- scenarios

PREAMBLE = (
    "Hydra launched this run and wrote its instructions at spawn.\n\n"
    "The policy this run was launched under bounds what you can do. Work inside it, and if "
    "the task cannot be done inside it,\nsay so and stop.\n"
)
OLD_PREAMBLE = (
    "Hydra launched this run and wrote its instructions at spawn. An older wording.\n"
    "If the task cannot be done, say so and stop.\n\n"
)
SECRET = "sk-proj-planted_openai-key-value-0123"


def record_lines(ident, task_text="do the thing", model="sonnet", provider="claude", command="run",
                 policy="standard", ended="2026-09-24T00:00:10Z", **first) -> bytes:
    envelope = {"run_id": ident, "provider": provider, "model": model, "ts": "2026-09-24T00:00:00Z"}
    envelope.update(first)
    rows = [{**envelope, "hydra": {"type": "run_start", "command": command, "policy": policy, "task": task_text}}]
    if ended:
        rows.append({**envelope, "hydra": {"type": "run_end", "ended_at": ended, "exit_code": 0}})
    return dump_lines(*rows)


def answer(outcome="completed", **counts) -> bytes:
    counts = counts or {"raw_input": 1, "output_tokens": 2}
    full = {
        "projection": {"outcome": outcome, "usage": {"counts": counts, "cost": 0.5}, "task": "never cached"},
        "events": [{"type": "text", "text": "an event the cache must not keep " + SECRET}],
    }
    return (json.dumps(full) + "\n").encode()


class Scenario:
    def __init__(self, rec, name: str, env=None):
        self.rec = rec
        self.name = name
        self.env = {
            "HYDRA_ROOT": "root",
            "HYDRA_BIN": f"{ROOT}/bin/hydra",
            "HYDRA_PUPPETS_DATA": f"{ROOT}/data",
        } if env is None else env
        self.steps = []
        self.puts = []
        self.removes = []
        self.put("bin/hydra", stub="hydra", exec=True)
        self.put("bin/hydra-puppets", stub="puppets", exec=True)
        self.put("data/aliases.json", aliases=True)

    def put(self, path: str, data=None, **spec):
        if data is not None:
            spec["hex"] = hexed(data)
        self.puts.append({"path": path, **spec})
        return self

    def remove(self, path: str):
        self.removes.append(path)
        return self

    def run_record(self, ident, table=None, name=None, **how):
        """A record under `root/runs/` and what the stub `hydra` answers about it."""
        self.put(f"root/runs/{name or ident}.jsonl", record_lines(ident, **how))
        self.put(f"bin/table/{ident}", answer() if table is None else table)
        return self

    def goal(self, goal_id, shown: bytes, log_bytes=b"", **how):
        for spec in answered(goal_id, shown, log_bytes, **how):
            self.puts.append(dict(spec, path=spec["path"].replace("puppets/", "root/puppets/")))
        return self

    def step(self, argv, env=None, lib=None):
        self.steps.append({
            "puts": self.puts, "removes": self.removes, "argv": argv,
            "env": self.env if env is None else env, "lib": lib,
        })
        self.puts, self.removes = [], []
        return self

    def done(self):
        assert not self.puts and not self.removes, self.name
        self.rec.play(self)


class Recorder:
    def __init__(self, out: Path, repo: Path):
        self.out = out
        self.repo = repo
        self.scenarios = []
        baseline = json.loads((out / "baseline.json").read_text(encoding="utf-8"))
        # The models build ticket 213 recorded, which is what a pass scores with.
        self.models = {
            case["name"]: file["text"]
            for case in baseline
            for file in case["written"]
            if file["path"].endswith(".json")
        }

    def scenario(self, name: str, env=None) -> Scenario:
        assert all(s["name"] != name for s in self.scenarios), name
        return Scenario(self, name, env)

    def play(self, scenario: Scenario) -> None:
        steps = []
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            given = {}
            for n, step in enumerate(scenario.steps):
                for spec in step["puts"]:
                    given[spec["path"]] = plant(root, spec, self.out, self.models)
                for path in step["removes"]:
                    (root / path).unlink()
                    given.pop(path, None)
                now = f"2001-01-01T00:{n // 60:02d}:{n % 60:02d}Z"
                lib = step["lib"]
                if lib:
                    argv = ["lib", json.dumps(lib).replace(ROOT, str(root))]
                else:
                    argv = ["main", *step["argv"]]
                ran = subprocess.run(
                    [
                        sys.executable, "-B", "-c", CHILD,
                        str(self.out / "data" / "aliases.json"),
                        str(self.repo / "integrations" / "grader" / "grader.py"),
                        now, *argv,
                    ],
                    cwd=root,
                    env=templated(step["env"], root),
                    stdin=subprocess.DEVNULL,
                    capture_output=True,
                )
                files = []
                for path in sorted(p for p in root.rglob("*") if p.is_file()):
                    name = str(path.relative_to(root))
                    data = path.read_bytes()
                    if given.get(name) != data:
                        text = plain(data.decode("utf-8"), root)
                        if lib:
                            text = re.sub(r"(?m)^env: HYDRA_ROOT=(?!<root>).*$", "env: HYDRA_ROOT=<tmp>", text)
                        files.append({"path": name, "text": text})
                for calls in ("bin/calls", "bin/puppets-calls"):
                    if (root / calls).exists():
                        (root / calls).unlink()
                steps.append({
                    **step,
                    "now": now,
                    "exit": ran.returncode,
                    "stdout": plain(ran.stdout.decode("utf-8"), root),
                    # A traceback names the files of the checkout the recorder ran in.
                    "stderr": plain(ran.stderr.decode("utf-8"), root).replace(str(self.repo), "<repo>"),
                    "files": files,
                })
        self.scenarios.append({"name": scenario.name, "steps": steps})


def model(label: str) -> dict:
    return {"model": f"train {label} on main"}


def scenarios(rec: Recorder) -> None:
    t1 = task("t1", "implementation", "low", "fix the tests " + SECRET, "run-a", "applied")
    t2 = task("t2", "security_review", "high", "audit the gate", "run-c")
    shown = state("g", [t1, t2], [{"run_id": "run-p"}])
    goal_log = log(
        {"event": "goal", "text": "ship it"},
        {"event": "panel_verdict", "judge_run_ids": ["run-j"]},
        {"event": "review", "task": "t2", "judge_run_ids": ["run-k"]},
    )

    def home(s: Scenario) -> Scenario:
        """The home `test_grader.py` builds, and more: three runs a goal names, a plan, two
        judges, one run nobody names, and records the scan leaves out."""
        s.put("root/config/preamble.md", PREAMBLE)
        s.run_record("run-a", task_text=PREAMBLE + "\nfix the failing tests")
        s.run_record("run-b", answer("failed", raw_input=10, output_tokens=0), task_text="nobody labels this " + SECRET, model="Opus 5.5")
        s.run_record("run-c", task_text=OLD_PREAMBLE + "audit it", provider="codex", model="gpt-5.1-codex")
        s.run_record("run-p", answer("completed"), task_text=PREAMBLE + "\npropose a plan", provider="agy", model=None)
        s.run_record("run-j", task_text="judge the plans café \U0001f600", command="resume", policy="judge")
        s.run_record("run-k", answer("rate_limited"), task_text="", ended=None)
        s.put("root/runs/evolve.jsonl", record_lines("run-e", command="evolve"))
        s.put("root/runs/garbled.jsonl", b"not json\n")
        s.put("root/runs/empty.jsonl", b"")
        s.put("root/runs/notes.txt", b"not a record\n")
        s.goal("g", shown, goal_log)
        return s

    # ------------------------------------------------------------ the passes
    s = home(rec.scenario("once, once again, a changed record, a class file, rebuild, backfill"))
    s.step(["once", "--jobs", "1"])
    s.step(["once", "--jobs", "1"])
    s.put("root/runs/run-b.jsonl", record_lines("run-b", task_text="nobody labels this", model="Opus 5.5") + b'{"torn": ')
    s.put("bin/table/run-b", answer("completed", raw_input=11, output_tokens=5))
    s.step(["once", "--jobs=1"])
    s.put("root/runs/run-b.jsonl", record_lines("run-b", task_text="nobody labels this", model="Opus 5.5") + b'{"torn": 1}\n')
    s.step(["once", "--j", "1"])
    classes = dump_lines(
        {"run_id": "run-b", "task_class": "docs", "grade": "low", "area": "readme", "kind": "edit", "unsure": True},
        {"run_id": "run-a", "task_class": "ignored, puppets names it", "area": "x", "kind": "y"},
        {"run_id": "run-nobody", "task_class": "no such run"},
    )
    s.put("root/grader/task-classes-2026-09-29.jsonl", classes)
    s.step(["once", "--jobs", "1"])
    s.step(["once", "--jobs", "1"])
    s.put("root/grader/task-classes-2026-09-30.jsonl", dump_lines({"run_id": "run-b", "kind": "edit", "area": "readme", "grade": "low", "task_class": "docs"}))
    s.step(["once", "--jobs", "1"])
    s.put("root/grader/task-classes-2026-09-30.jsonl", dump_lines({"run_id": "run-b", "task_class": "docs", "grade": "low", "area": "readme", "kind": "edit"}))
    s.step(["once", "--jobs", "1"])
    s.put("root/grader/ticket-verdicts.json", b'{"run-b": "fixed", "run-a": "unknown", "run-nobody": "applied"}\n')
    s.step(["once", "--jobs", "1"])
    s.step(["rebuild", "--jobs", "1"])
    s.put("root/grader/models/outcome.json", **model("outcome"))
    s.put("root/grader/models/verdict.json", **model("verdict"))
    s.put("root/grader/models/notes.txt", b"not a model\n")
    s.step(["once", "--jobs", "1"])
    s.step(["backfill"])
    s.step(["backfill"])
    s.put("root/grader/models/task.json", **model("task"))
    s.run_record("run-d", task_text=PREAMBLE + "\na new run")
    s.step(["once", "--jobs", "1"])
    s.step(["backfill"])
    s.remove("root/grader/models/outcome.json").remove("root/grader/models/verdict.json").remove("root/grader/models/task.json")
    s.step(["rebuild", "--jobs", "1"])
    s.put("root/grader/models/task.json", **model("task"))
    s.goal("g", state("g", [dict(t1, human_result="fixed"), t2], [{"run_id": "run-p"}]), goal_log)
    s.step(["once", "--jobs", "1"])
    s.step(["rebuild", "--jobs", "1"])
    s.remove("root/runs/run-c.jsonl")
    s.step(["once", "--jobs", "1"])
    s.put("root/runs/run-a.jsonl", record_lines("run-a", task_text=PREAMBLE + "\nfix the failing tests", ended="2026-09-24T01:00:00.5+00:00"))
    s.step(["once", "--jobs", "4"])
    s.done()

    s = home(rec.scenario("the homes named on their own"), )
    env = {
        "HYDRA_ROOT": "root/",
        "HYDRA_GRADER_HOME": "else/grader",
        "HYDRA_PUPPETS_HOME": "root/puppets",
        "HYDRA_BIN": "bin/hydra",
        "HYDRA_PUPPETS_BIN": f"{ROOT}/bin/hydra-puppets",
        "HYDRA_PUPPETS_DATA": f"{ROOT}/data",
        "PATH": "/usr/bin:/bin",
        "LANG": "en_US.UTF-8",
    }
    s.put("else/grader/models/outcome.json", **model("outcome"))
    s.put("root/puppets/.jsonl", b'{"event": "goal"}\n')
    s.step(["once", "--jobs", "1"], env=env)
    s.step(["once"], env=env)
    s.step(["backfill"], env=env)
    s.step(["rebuild", "--jobs", "-3"], env=env)
    s.step(["once", "--jobs", "1"])
    s.done()

    s = rec.scenario("hydra is found on PATH")
    env = {"HYDRA_ROOT": "root", "PATH": f"/nowhere:{ROOT}/bin", "HYDRA_PUPPETS_BIN": f"{ROOT}/bin/hydra-puppets", "HYDRA_PUPPETS_DATA": f"{ROOT}/data"}
    s.run_record("run-a")
    s.step(["once", "--jobs", "1"], env=env)
    s.step(["once", "--jobs", "1"], env=dict(env, HYDRA_BIN=""))
    # A cached answer needs no `hydra`, whatever names it.
    s.step(["rebuild", "--jobs", "1"], env=dict(env, HYDRA_BIN=""))
    s.run_record("run-a", task_text="changed")
    s.step(["once", "--jobs", "1"], env=dict(env, HYDRA_BIN=""))
    s.step(["once", "--jobs", "1"], env=dict(env, HYDRA_BIN="", PATH="/nowhere:/nowhere/else"))
    s.step(["once", "--jobs", "1"], env=dict(env, PATH="/nowhere"))
    s.step(["once", "--jobs", "1"], env={key: value for key, value in env.items() if key != "PATH"})
    s.step(["once", "--jobs", "1"], env=dict(env, PATH=f"{ROOT}/bin"))
    s.done()

    s = rec.scenario("nothing to collect")
    s.step(["once", "--jobs", "1"])
    s.step(["rebuild"])
    s.step(["backfill"])
    s.put("root/grader/models/outcome.json", **model("outcome"))
    s.step(["backfill"])
    s.put("root/runs/evolve.jsonl", record_lines("run-e", command="evolve"))
    s.step(["once"])
    s.put("root/grader/dataset.jsonl", b"")
    s.step(["backfill"])
    s.done()

    s = rec.scenario("two records with one run id, and names that sort")
    s.run_record("run-a", name="b-second", task_text="the second file")
    s.put("root/runs/a-first.jsonl", record_lines("run-a", task_text="the first file"))
    s.run_record("run-B")
    s.run_record("run-é", task_text="non-ascii id")
    s.run_record("Run-z")
    s.put("root/grader/models/outcome.json", **model("outcome"))
    s.step(["once", "--jobs", "1"])
    s.step(["once", "--jobs", "1"])
    s.done()

    s = rec.scenario("a runs directory with another name, through collect()")
    s.put("records/run-a.jsonl", record_lines("run-a", task_text=PREAMBLE + "\nfix it"))
    s.put("bin/table/run-a", answer())
    s.put("root/config/preamble.md", PREAMBLE)
    lib_env = {"HYDRA_ROOT": f"{ROOT}/root", "HYDRA_PUPPETS_BIN": f"{ROOT}/bin/hydra-puppets", "KEPT": "yes"}
    lib = {"runs": f"{ROOT}/records", "home": f"{ROOT}/home", "hydra": f"{ROOT}/bin/hydra", "jobs": 1, "rebuild": False}
    s.step([], env=lib_env, lib=lib)
    s.step([], env=lib_env, lib=lib)
    s.step([], env=lib_env, lib=dict(lib, rebuild=True, jobs=3))
    s.put("deep/runs/run-b.jsonl", record_lines("run-b"))
    s.put("bin/table/run-b", answer("failed"))
    s.step([], env=lib_env, lib=dict(lib, runs=f"{ROOT}/deep/runs", home=f"{ROOT}/home2"))
    s.done()

    # ------------------------------------------------------- passes that fail
    s = home(rec.scenario("a goal the helper refuses, then one it shows"))
    s.goal("h", b"", b"", code=3, stderr=b"hydra-puppets: h: line 1 is not JSON\n")
    s.step(["once", "--jobs", "1"])
    s.step(["rebuild", "--jobs", "1"])
    s.step(["watch", "--interval", "-1", "--jobs", "1"])
    s.step(["backfill"])
    s.goal("h", state("other"), b"")
    s.remove("bin/shows/h.exit").remove("bin/shows/h.err")
    s.step(["once", "--jobs", "1"])
    s.goal("h", state("h", [task("t1", text=7, run_id="run-b")]), b"")
    s.step(["once", "--jobs", "1"])
    s.goal("h", state("h", [task("t1", run_id="run-b")]), b"not json\n")
    s.step(["once", "--jobs", "1"])
    s.goal("h", state("h", [task("t1", run_id="run-b")]), b"")
    s.step(["once", "--jobs", "1"])
    s.done()

    s = home(rec.scenario("no helper"))
    s.remove("bin/hydra-puppets")
    s.step(["once", "--jobs", "1"])
    s.step(["watch", "--interval=-1"])
    env = dict(s.env, HYDRA_PUPPETS_BIN="bin/hydra-puppets")
    s.step(["once", "--jobs", "1"], env=env)
    s.done()

    s = rec.scenario("hydra show fails on the second of three")
    s.run_record("run-a").run_record("run-b").run_record("run-c")
    s.put("bin/table/run-b.exit", b"3\n")
    s.step(["once", "--jobs", "1"])
    s.step(["watch", "--interval", "-1", "--jobs", "1"])
    s.remove("bin/table/run-b.exit").put("bin/table/run-b.kill", b"")
    s.step(["once", "--jobs", "1"])
    s.step(["watch", "--jobs", "1", "--int", "-5"])
    s.remove("bin/table/run-b.kill").remove("bin/table/run-b")
    s.step(["once", "--jobs", "1"])
    s.put("bin/table/run-b", answer())
    s.step(["watch", "--interval", "-1", "--jobs", "1"])
    s.step(["watch", "--interval", "-1", "--jobs", "1"])
    s.done()

    for name, table in [
        ("not json", b"not json\n"),
        ("nothing", b""),
        ("not utf-8", b'{"projection": "caf\xe9"}\n'),
        ("a list", b"[1]\n"),
        ("a number", b"7\n"),
        ("null", b"null\n"),
        ("a projection that is a list", b'{"projection": ["completed"]}\n'),
        ("a projection that is text", b'{"projection": "completed"}\n'),
        ("a usage that is a list", b'{"projection": {"outcome": "failed", "usage": [1]}}\n'),
        ("counts that are a list", b'{"projection": {"outcome": "failed", "usage": {"counts": [1, 2]}}}\n'),
        ("counts that are text", b'{"projection": {"usage": {"counts": "many"}}}\n'),
        ("a NaN among the counts", b'{"projection": {"outcome": "completed", "usage": {"counts": {"raw_input": NaN}}}}\n'),
        ("an outcome that is infinite", b'{"projection": {"outcome": Infinity}}\n'),
        ("no projection", b'{"events": []}\n'),
        ("a projection that is null", b'{"projection": null}\n'),
        ("a projection that is empty", b'{"projection": {}, "projection": 0}\n'),
        ("a usage that is null, counts that are zero", b'{"projection": {"outcome": ["a", {"b": 1.5}], "usage": {"counts": 0}}}\n'),
        ("carriage returns between the tokens", b'{"projection":\r\n{"outcome":\r"completed"}}\r\n'),
        ("more after the answer", answer() + answer()),
    ]:
        s = rec.scenario(f"hydra show prints {name}")
        s.run_record("run-a", table)
        s.put("root/grader/models/outcome.json", **model("outcome"))
        s.step(["once", "--jobs", "1"])
        s.step(["once", "--jobs", "1"])
        s.done()

    s = rec.scenario("no hydra")
    s.run_record("run-a")
    s.step(["once", "--jobs", "1"], env=dict(s.env, HYDRA_BIN=f"{ROOT}/bin/nothing"))
    s.step(["once", "--jobs", "1"], env=dict(s.env, HYDRA_BIN="bin/nothing"))
    s.step(["once", "--jobs", "1"], env=dict(s.env, HYDRA_BIN=f"{ROOT}/bin"))
    s.step(["watch", "--interval", "-1", "--jobs", "1"], env=dict(s.env, HYDRA_BIN=f"{ROOT}/bin/nothing", HYDRA_PUPPETS_BIN=f"{ROOT}/bin/hydra-puppets"))
    s.done()

    # Records a writer could leave.
    for name, lines in [
        ("a run id that is a number", record_lines(12)),
        ("a run id that is true", record_lines(True)),
        ("a run id that is a float", record_lines(1.5)),
        ("a run id that is a list", record_lines(["run-x"])),
        ("a run id that is an object", record_lines({"id": "run-x"})),
        ("a task that is a number", record_lines("run-x", task_text=7)),
        ("a task that is a list", record_lines("run-x", task_text=["do", "it"])),
        ("a task that is zero", record_lines("run-x", task_text=0)),
        ("a task that is null", record_lines("run-x", task_text=None)),
        ("a provider that is a list", record_lines("run-x", provider=["claude"])),
        ("a provider that is a number", record_lines("run-x", provider=7)),
        ("a model that is a list", record_lines("run-x", model=["sonnet"])),
        ("a policy that is an object", record_lines("run-x", policy={"name": "standard"})),
        ("a hydra field that is text", dump_lines({"run_id": "run-x", "provider": "claude", "hydra": "run"})),
        ("an end with no offset after a start with one", record_lines("run-x", ended="2026-09-24T00:00:10")),
        ("an end that does not read", record_lines("run-x", ended="soon")),
        ("a NaN in the first line", record_lines("run-x", weight=float("nan"))),
        ("a run id with a slash", record_lines("deep/run-x")),
        ("a run id that starts with a dash", record_lines("--events")),
        ("a line that is not utf-8", b'{"run_id": "caf\xe9"}\n'),
        ("a line 0 that is a list", b"[1]\n"),
    ]:
        s = rec.scenario(f"a record with {name}")
        s.run_record("run-a")
        s.put("root/runs/run-odd.jsonl", lines)
        for ident in ("run-x", "12", "True", "1.5", "--events"):
            s.put(f"bin/table/{ident}", answer())
        s.put("root/grader/models/outcome.json", **model("outcome"))
        s.step(["once", "--jobs", "1"])
        s.step(["rebuild", "--jobs", "1"])
        s.step(["once", "--jobs", "1"])
        s.done()
    s = rec.scenario("a record that is a directory")
    s.run_record("run-a")
    s.put("root/runs/dir.jsonl/inside", b"")
    s.step(["once", "--jobs", "1"])
    s.done()

    # The grader's own files, as somebody left them.
    def own(name: str, path: str, data: bytes, argv=("once", "--jobs", "1"), second=("rebuild", "--jobs", "1"), prime=False):
        s = rec.scenario(name)
        s.run_record("run-a").run_record("run-b")
        s.put("root/grader/models/outcome.json", **model("outcome"))
        if prime:
            s.step(["once", "--jobs", "1"])
            s.run_record("run-c")
        s.put(path, data)
        s.step(list(argv))
        s.step(list(second))
        s.done()

    for name, data in [
        ("a list", b'["run-a"]\n'), ("null", b"null\n"), ("text", b'"run-a"\n'), ("not json", b"{\n"),
        ("not utf-8", b'{"caf\xe9": 1}\n'), ("empty", b""), ("a NaN", b'{"run-z": NaN}\n'),
        ("stale fingerprints of every type", b'{"run-a": 7, "run-b": null, "run-gone": "kept"}\n'),
    ]:
        own(f"a cursor that is {name}", "root/grader/cursor.json", data)
    for name, data in [
        ("a list", b"[]\n"), ("null", b"null\n"), ("not json", b"nope"),
        ("an entry that is null", b'{"run-a": null}\n'),
        ("an entry that is a list", b'{"run-a": ["fingerprint"]}\n'),
        ("an entry with no fingerprint", b'{"run-a": {}, "run-gone": {"fingerprint": "x", "answer": {"kept": NaN}}}\n'),
        ("an entry of a run that is gone", b'{"run-gone": {"fingerprint": "x", "answer": {"kept": true}}}\n'),
    ]:
        own(f"a cache that is {name}", "root/grader/show-cache.json", data)
    row = {"run_id": "run-old", "provider": "claude", "model": "opus", "policy": "standard", "task_text": "an old row"}
    for name, data in [
        ("a row with no run id", dump_lines({"provider": "claude"})),
        ("a row that is a list", b"[1]\n"),
        ("a row that is text", b'"run-a"\n'),
        ("a row whose run id is a list", dump_lines({"run_id": ["x"]})),
        ("a row whose run id is a number", dump_lines({"run_id": 7}, row)),
        ("rows whose run ids are numbers that sort", dump_lines({"run_id": 7}, {"run_id": 2.5}, {"run_id": True})),
        ("a garbled line", b"not json\n"),
        ("a torn tail", dump_lines(row) + b'{"run_id": "run-torn"'),
        ("a row of a run that is gone, and one twice", dump_lines(row, dict(row, run_id="run-a"), dict(row, task_text="the later one"))),
        ("a row with a NaN", dump_lines(dict(row, weight=float("nan")))),
        ("an old row with odd fields", dump_lines(dict(row, provider=7, model=["x"], task_text=None))),
        ("an old row whose task is a number", dump_lines(dict(row, task_text=7))),
    ]:
        own(f"a dataset with {name}", "root/grader/dataset.jsonl", data, second=("backfill",))
    for name, data in [
        ("a list that names the run", b'["run-a"]\n'),
        ("a list that names another", b'["run-z", 7]\n'),
        ("text that holds the run id", b'"run-a and run-b"\n'),
        ("text that does not", b'"nothing"\n'),
        ("null", b"null\n"),
        ("a number", b"7\n"),
        ("verdicts of every type", b'{"run-a": null, "run-b": ["fixed", 1.5]}\n'),
        ("not json", b"[\n"),
    ]:
        own(f"ticket verdicts that are {name}", "root/grader/ticket-verdicts.json", data)
    for name, data in [
        ("a run id that is a list", dump_lines({"run_id": ["run-a"]})),
        ("bytes that are not utf-8", b'{"run_id": "caf\xe9"}\n'),
        ("a class for each run, one unsure", dump_lines(
            {"run_id": "run-a", "task_class": 7, "grade": ["low"], "area": {"a": 1}, "kind": None, "unsure": 1},
            {"run_id": "run-b", "unsure": 0, "extra": "never read"},
        )),
        ("a NaN, which a digest takes and a row does not", b'{"run_id": "run-a", "grade": NaN}\n'),
        ("a label written twice in another order", dump_lines({"run_id": "run-a", "task_class": "x", "b": {"z": 1, "a": 2}})),
    ]:
        own(f"a class file with {name}", "root/grader/task-classes-x.jsonl", data, second=("once", "--jobs", "1"))
    base = json.loads(rec.models["train outcome on main"])
    for name, data in [
        ("a list", b"[1]\n"), ("null", b"null\n"), ("not json", b"nope\n"),
        ("no version", json.dumps({k: v for k, v in base.items() if k != "version"}).encode()),
        ("no label", json.dumps({k: v for k, v in base.items() if k != "label"}).encode()),
        ("no weights", json.dumps({k: v for k, v in base.items() if k != "weights"}).encode()),
        ("a NaN weight", json.dumps(dict(base, weights=[float("nan")] * len(base["weights"]))).encode()),
        ("a label that is NaN", json.dumps(dict(base, label=float("nan"))).encode()),
        ("a version that is a list", json.dumps(dict(base, version=["v1"])).encode()),
        ("a version that is a number", json.dumps(dict(base, version=7, label=None)).encode()),
    ]:
        own(f"a model that is {name}", "root/grader/models/a-first.json", data, second=("backfill",))
    own("a model that is a directory", "root/grader/models/a-first.json/inside", b"", second=("backfill",))
    done = [
        {"run_id": "run-a", "label": "outcome", "p": 0.25, "model_version": base["version"], "scored_at": "2000-01-01T00:00:00Z"},
        {"run_id": "run-b", "label": "outcome", "p": 0.5, "model_version": "an older version", "scored_at": "2000-01-01T00:00:00Z"},
        {"run_id": 7, "model_version": 7.0},
    ]
    for name, data in [
        ("a run scored and a run scored by an older model", dump_lines(*done)),
        ("a line with no model version", dump_lines({"run_id": "run-a"})),
        ("a line with no run id", dump_lines({"model_version": "v"})),
        ("a line that is a list", b"[1]\n"),
        ("a run id that is a list", dump_lines({"run_id": ["x"], "model_version": "v"})),
        ("a garbled line", dump_lines(done[0]) + b"not json\n"),
        ("a torn tail", dump_lines(done[0]) + b'{"run_id": "run-b", "model_version": "' + base["version"].encode()),
    ]:
        own(f"scores with {name}", "root/grader/scores.jsonl", data, second=("backfill",))
    own("a preamble that is not utf-8", "root/config/preamble.md", b"caf\xe9\n")
    own("a preamble that is a directory", "root/config/preamble.md/inside", b"")
    own("a preamble with carriage returns and no last newline", "root/config/preamble.md", b"Line one.\r\nLine two.")

    # A third record after a first pass: only it is asked about, and the cache keeps the rest.
    own("a new record after a pass", "root/grader/task-classes-x.jsonl", dump_lines({"run_id": "run-c", "task_class": "new"}), prime=True, second=("once", "--jobs", "1"))

    # ------------------------------------------------------- the command line
    s = rec.scenario("arguments")
    for argv in [
        [], ["-h"], ["--help"], ["once", "-h"], ["watch", "--help"], ["backfill", "-h"], ["rebuild", "--he"],
        ["x"], ["onc"], ["Once"], ["--foo"], ["--foo", "once"], ["-x", "backfill", "y"],
        ["once", "y"], ["once", "--jobs"], ["once", "--jobs", "x"], ["once", "--jobs", ""], ["once", "--jobs", "1.5"],
        ["once", "--jobs", "--jobs"], ["once", "--jobs=", "1"], ["once", "--interval", "5"], ["once", "--", "--jobs"],
        ["once", "--jobs", "1", "--foo", "bar"], ["once", "--jobs", "1", "2"], ["once", "-j", "1"],
        ["backfill", "--jobs", "2"], ["backfill", "x"], ["rebuild", "--jobs", "é"], ["rebuild", "--jobs", "1 2"],
        ["watch"], ["watch", "--jobs", "2"], ["watch", "--interval"], ["watch", "--interval", "x"],
        ["watch", "--interval", "5", "z"], ["watch", "--i"], ["watch", "--in", "1.0"], ["watch", "--interval=", "--jobs", "1"],
        ["watch", "--interval", "-x"], ["watch", "--interval", "5", "--jobs", "0x10"], ["once", "--help=x"],
        ["once", "--jobs", "１２", "--jobs", "x"], ["once", "--jo=1", "--j=q"], ["--", "once", "x"], ["once", "--jobs", "-"],
        # These parse, and there is nothing to collect.
        ["once", "--jobs", " 2 "], ["once", "--jobs", "1_0"], ["once", "--jobs", "-1"], ["once", "--jobs", "+3"],
        ["once", "--jobs", "１２"], ["once", "--jobs", "0"], ["once", "--jobs", "9" * 40], ["once", "--jobs", "-" + "9" * 40],
        ["once", "--jobs", "1", "--jobs", "2"], ["rebuild", "--j=7"], ["--", "once"], ["once", "--"],
        ["watch", "--interval", "-1"], ["watch", "--jobs", "1", "--interval=-7"], ["watch", "--interval", "9223372037"],
        ["watch", "--interval", "9" * 40], ["watch", "--interval", "-" + "9" * 40], ["watch", "--interval", "5", "--interval", "-1"],
    ]:
        s.step(argv)
    s.done()


def record(out: Path, repo: Path, labels) -> None:
    record_labels(out, repo, labels)
    rec = Recorder(out, repo)
    scenarios(rec)
    write(out / "collect" / "scenarios.json", {"stubs": STUBS, "scenarios": rec.scenarios})
