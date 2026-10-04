"""The part of `record.py` that records `baseline.py`'s `train` and `score` (build ticket 213).

`record.py` loads this file and calls `record`. It is not run on its own.

Each case is one command line. The Python runs it as a child with nothing in the environment
but what the case names, in a scratch directory that holds the case's files under relative
paths, so no recorded byte names the directory. What comes back is the exit code, stdout,
stderr and every file the command wrote. `fixtures/baseline.json` holds the cases and
`fixtures/baseline/` the datasets, which are generated here from a fixed seed.

Two things are frozen. `records.load_aliases()` reads the live `aliases.json` of the checkout,
so the child replaces it with a reader of `fixtures/data/aliases.json`. And every case names
`HYDRA_ROOT` or `HYDRA_GRADER_HOME`, so no default path reaches the real `~/hydra`.
"""

import importlib.util
import json
import math
import random
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

# The child: `baseline.py` loaded as a module, its alias table frozen, its `main` run as the
# script would run it. `sys.argv[0]` is what `argparse` names the program by.
CHILD = """
import importlib.util, sys
frozen, script = sys.argv[1], sys.argv[2]
sys.argv = ["baseline.py", *sys.argv[3:]]
spec = importlib.util.spec_from_file_location("recorded_baseline", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
load = module.records.load_aliases
module.records.load_aliases = lambda path=None: load(frozen)
raise SystemExit(module.main())
"""

PREAMBLE = "Hydra launched this run and wrote its instructions at spawn.\n\n"
WORDS = (
    "port the grader read the ticket fix failing test in worktree add a fixture for record "
    "contract and say what changed café naïve \U0001f600 résumé señal"
).split()
PAIRS = [
    ("claude", "opus"),
    ("claude", "sonnet"),
    ("claude", None),
    ("codex", "sol"),
    ("codex", "astra"),
    ("cursor", "composer-2.5"),
    ("agy", "gemini-pro"),
    ("agy", None),
]
LEAN = {"claude": -0.6, "codex": 0.3, "cursor": 0.1, "agy": 1.1}
FAILURES = ["failed"] * 5 + ["crashed"] * 2 + ["timed_out"] * 3 + ["rate_limited"] * 3 + ["gated"]


def millis(text: str) -> int:
    return int(datetime.fromisoformat(text).replace(tzinfo=timezone.utc).timestamp() * 1000)


def run_id(rng: random.Random, at: int) -> str:
    h = f"{at:012x}7{rng.getrandbits(12):03x}{8 + rng.getrandbits(2):x}{rng.getrandbits(60):015x}"
    return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"


def sigmoid(z: float) -> float:
    return 1.0 / (1.0 + math.exp(-z))


def task_text(rng: random.Random, chars: int) -> str:
    text = ""
    while len(text) < chars:
        text += rng.choice(WORDS) + " "
    return text[:chars]


def row(rng: random.Random, at: int, chars=None, mu=5.0) -> dict:
    provider, model = rng.choice(PAIRS)
    if chars is None:
        chars = min(20000, int(math.exp(rng.gauss(mu, 1.3))))
    text = task_text(rng, chars)
    if rng.random() < 0.6:
        text = PREAMBLE + text
    size = math.log1p(len(text)) - 5.0
    out = {
        "run_id": run_id(rng, at),
        "provider": provider,
        "model": model,
        "policy": rng.choice(["standard", "standard", "read-only", "judge", None]),
        "task_text": text,
    }
    draw = rng.random()
    if draw < 0.04:
        out["outcome"] = "stopped"
    elif draw < 0.07:
        out["outcome"] = None
    elif draw < 0.09:
        pass
    elif rng.random() < sigmoid(-0.3 + LEAN[provider] + 0.5 * size):
        out["outcome"] = rng.choice(FAILURES)
    else:
        out["outcome"] = "completed"
    draw = rng.random()
    if draw < 0.8:
        lean = 0.9 if model in ("sonnet", "composer-2.5", None) else -0.4
        out["human_verdict"] = "fixed" if rng.random() < sigmoid(lean + 0.4 * size) else "applied"
    elif draw < 0.9:
        out["human_verdict"] = None
    elif draw < 0.95:
        out["human_verdict"] = "unknown"
    out["tokens"] = {"output_tokens": rng.randrange(5000)}
    out["duration_seconds"] = round(rng.random() * 900, 3)
    return out


def dataset(
    seed: int, count: int, first="2026-09-05", last="2026-10-02", chars=None, mu=5.0
) -> list[dict]:
    rng = random.Random(seed)
    start, end = millis(first), millis(last)
    times = sorted(rng.randrange(start, end) for _ in range(count))
    return [row(rng, at, chars, mu) for at in times]


def squares_differ(baseline, rows: list[dict]) -> tuple[bool, bool]:
    """Whether `x ** 2` and `x * x` part ways on this dataset, in the scale's deviation and
    in the Brier score of the constant base rate. They differ on about one float in a
    thousand, so most datasets cannot tell a port that multiplies from one that calls `pow`."""
    test_from = baseline.day("2026-09-23")
    labelled = [r for r in rows if baseline.target(r, "outcome") is not None]
    fit, test = baseline.split(labelled, test_from)
    for part in (fit, test):
        ys = [baseline.target(r, "outcome") for r in part]
        if min(sum(ys), len(ys) - sum(ys)) < baseline.MIN_MINORITY:
            return False, False
    values = [baseline.raw_features(r)["log_task_chars"] for r in fit]
    mean = sum(values) / len(values)
    scale = sum((v - mean) ** 2 for v in values) != sum((v - mean) * (v - mean) for v in values)
    base = sum(baseline.target(r, "outcome") for r in fit) / len(fit)
    ys = [baseline.target(r, "outcome") for r in test]
    brier = sum((base - y) ** 2 for y in ys) != sum((base - y) * (base - y) for y in ys)
    return scale, brier


def dump(rows) -> bytes:
    return "".join(json.dumps(r) + "\n" for r in rows).encode()


def odd_dataset() -> list[dict]:
    """Rows a writer could leave and the Python still trains on: providers that are numbers
    and compare equal, text that is a list or an object, a lone surrogate in a model's name,
    run ids spelled every way `int(text, 16)` reads."""
    rng = random.Random(2132)
    rows = dataset(2131, 180)
    providers = [1, 1.0, True, "café", "\ud800lone", None, 0, "claude"]
    for n, r in enumerate(rows):
        kind = n % 9
        if kind == 0:
            r["provider"] = providers[(n // 9) % len(providers)]
        elif kind == 1:
            r["model"] = [3, 3.0, False, "\U0001f600", "x=y", 1e16][(n // 9) % 6]
        elif kind == 2:
            r["policy"] = [7, 2.5, True, "política"][(n // 9) % 4]
        elif kind == 3:
            r["task_text"] = [
                ["a", "b", "Hydra launched this run"],
                {"Hydra launched this run": 1, "k": 2},
                None,
                "",
                [],
                {},
                0,
                False,
                ["x"] * 40,
            ][(n // 9) % 9]
        elif kind == 4:
            del r["task_text"]
        elif kind == 5:
            plain = r["run_id"].replace("-", "")
            r["run_id"] = [
                plain,
                plain.upper(),
                "0x" + plain,
                " " + r["run_id"],
                "+" + r["run_id"],
                plain[:4] + "_" + plain[4:],
                "0X_" + plain,
                " " + plain,
                "".join(chr(0xFF10 + int(c)) if c.isdigit() else c for c in plain[:12]) + plain[12:],
            ][(n // 9) % 9]
        elif kind == 6:
            r["outcome"] = rng.choice(["completed", "failed", "auth_failed", "skill_collision", 0, ["x"]])
    return rows


def record_bytes(first: dict, *more: dict) -> bytes:
    return dump([first, *more])


def start_line(ident: str, provider: str, model, task, policy="standard", command="run") -> dict:
    hydra = {"type": "run_start", "command": command, "policy": policy, "task": task}
    return {
        "run_id": ident,
        "parent_run_id": None,
        "provider": provider,
        "model": model,
        "ts": "2026-09-24T00:00:00Z",
        "hydra": hydra,
    }


def edited(model: dict, **over) -> bytes:
    out = dict(model)
    for key, value in over.items():
        if value is ...:
            del out[key]
        else:
            out[key] = value
    return (json.dumps(out, indent=2) + "\n").encode()


def listing(root: Path, given: dict[str, bytes]) -> list[dict]:
    """Every file the command left that was not put there unchanged."""
    found = []
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        name = str(path.relative_to(root))
        data = path.read_bytes()
        if given.get(name) != data:
            found.append({"path": name, "text": data.decode("utf-8")})
    return found


class Recorder:
    def __init__(self, out: Path, repo: Path, scan: dict[str, bytes]):
        self.out = out
        self.repo = repo
        self.scan = scan
        self.cases = []
        self.datasets: dict[str, bytes] = {}
        self.models: dict[str, str] = {}

    def dataset(self, name: str, rows=None, data=None) -> dict:
        data = dump(rows) if data is None else data
        self.datasets[name] = data
        return {"path": "dataset.jsonl", "dataset": name}

    def case(self, name: str, argv: list[str], files=(), env=None) -> dict:
        env = {"HYDRA_ROOT": "root"} if env is None else env
        assert "HYDRA_ROOT" in env or "HYDRA_GRADER_HOME" in env, name
        assert all(case["name"] != name for case in self.cases), name
        given = {}
        for spec in files:
            if "dataset" in spec:
                data = self.datasets[spec["dataset"]]
                if "line" in spec:
                    # The dataset with one line replaced, so a row of the wrong shape costs
                    # the fixtures one line and not a second copy of the dataset.
                    lines = data.splitlines(keepends=True)
                    lines[spec["line"]] = bytes.fromhex(spec["with"])
                    data = b"".join(lines)
                given[spec["path"]] = data
            elif "scan" in spec:
                given[spec["path"]] = self.scan[spec["scan"]]
            elif "model" in spec:
                given[spec["path"]] = self.models[spec["model"]].encode()
            else:
                given[spec["path"]] = bytes.fromhex(spec["hex"])
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for path, data in given.items():
                (root / path).parent.mkdir(parents=True, exist_ok=True)
                (root / path).write_bytes(data)
            ran = subprocess.run(
                [
                    sys.executable,
                    "-c",
                    CHILD,
                    str(self.out / "data" / "aliases.json"),
                    str(self.repo / "integrations" / "grader" / "baseline.py"),
                    *argv,
                ],
                cwd=root,
                env=dict(env, PYTHONDONTWRITEBYTECODE="1"),
                stdin=subprocess.DEVNULL,
                capture_output=True,
            )
            case = {
                "name": name,
                "argv": argv,
                "env": env,
                "files": list(files),
                "exit": ran.returncode,
                "stdout": ran.stdout.decode("utf-8"),
                # A traceback names the files of this checkout. Its directory is the one thing
                # replaced here, so a recording made elsewhere gives the same bytes (build 220).
                "stderr": ran.stderr.decode("utf-8").replace(str(self.repo), "<repo>"),
                "written": listing(root, given),
            }
        self.cases.append(case)
        return case

    def trained(self, name: str, argv: list[str], files, env=None) -> dict:
        """A `train` that must succeed, and whose model later cases score with."""
        case = self.case(name, argv, files, env)
        assert case["exit"] == 0, (name, case["stderr"])
        (written,) = [f for f in case["written"] if f["path"].endswith(".json")]
        self.models[name] = written["text"]
        return json.loads(written["text"])


def hexed(data: bytes) -> str:
    return data.hex()


def record(out: Path, repo: Path, scan_inputs: list[tuple[str, bytes]]) -> None:
    corpus = {name: data for name, data in scan_inputs if name.endswith(".jsonl")}
    assert len(corpus) == 12, len(corpus)
    rec = Recorder(out, repo, corpus)

    # ------------------------------------------------------------------ train
    main = rec.dataset("main", dataset(213, 200))
    odd = rec.dataset("odd", odd_dataset())
    # Every task the same length, so the scale's deviation is 0 and becomes 1.
    constant = rec.dataset("constant", dataset(2133, 150, chars=300))
    rows = dataset(213, 200)

    models = {}
    for label in ("outcome", "verdict", "task"):
        models[label] = rec.trained(
            f"train {label} on main", ["train", "dataset.jsonl", "model.json", "--label", label], [main]
        )
    rec.trained("train with no label named", ["train", "dataset.jsonl", "out/deep/model.json"], [main])
    rec.trained(
        "train into the grader home",
        ["train", "dataset.jsonl", "--label=task"],
        [main],
        env={"HYDRA_GRADER_HOME": "home"},
    )
    rec.trained(
        "train into the grader under the root",
        ["train", "--label", "verdict", "dataset.jsonl"],
        [main],
        env={"HYDRA_ROOT": "root/", "HYDRA_GRADER_HOME": ""},
    )
    rec.trained(
        "train from an earlier day",
        ["train", "dataset.jsonl", "model.json", "--test-from", "2026-09-20"],
        [main],
    )
    rec.trained(
        "train from a time with a fraction",
        ["train", "dataset.jsonl", "--test-from=2026-09-22T12:30:00.5", "model.json"],
        [main],
    )
    rec.trained(
        "train from a week date, options cut short",
        ["train", "--t", "2026-W39-2", "--l", "task", "dataset.jsonl", "model.json"],
        [main],
    )
    rec.trained(
        "train from a time whose offset is replaced, not applied",
        ["train", "dataset.jsonl", "model.json", "--test-from", "2026-09-23T00:00:00+05:30"],
        [main],
    )
    rec.trained(
        "train over a model that exists",
        ["train", "dataset.jsonl", "model.json"],
        [main, {"path": "model.json", "hex": hexed(b"kept\n")}],
    )
    models["odd"] = rec.trained(
        "train outcome on odd rows", ["train", "dataset.jsonl", "model.json"], [odd]
    )
    rec.trained(
        "train task on odd rows", ["train", "dataset.jsonl", "model.json", "--label", "task"], [odd]
    )
    rec.trained(
        "train on tasks of one length", ["train", "dataset.jsonl", "model.json"], [constant]
    )
    # Two small datasets found by search, the first of each kind: one where the scale's
    # deviation depends on `x ** 2` being `pow`, one where the Brier score does.
    spec = importlib.util.spec_from_file_location("searched_baseline", repo / "integrations/grader/baseline.py")
    baseline = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(baseline)
    wanted = {"scale": 0, "brier": 1}
    for seed in range(21300, 61300):
        found = squares_differ(baseline, dataset(seed, 110, mu=3.5))
        for kind, at in list(wanted.items()):
            if found[at]:
                del wanted[kind]
                rec.trained(
                    f"train where the square is pow, in the {kind}",
                    ["train", "dataset.jsonl", "model.json"],
                    [rec.dataset(f"square-{kind}", dataset(seed, 110, mu=3.5))],
                )
        if not wanted:
            break
    assert not wanted, wanted
    lines = [dump([r]) for r in rows]

    def patched(at: int, data: bytes) -> dict:
        return dict(main, line=at % len(rows), **{"with": hexed(data)})

    rec.trained(
        "train with a torn tail",
        ["train", "dataset.jsonl", "model.json"],
        [patched(-1, lines[-1] + b'{"run_id": "0199')],
    )

    # Under the floor, and nothing to train on.
    few = [r for r in rows if r.get("outcome") == "completed"][:40] + [
        r for r in rows if r.get("outcome") == "failed"
    ][:9]
    rec.case(
        "the fit slice is under the floor",
        ["train", "dataset.jsonl", "model.json"],
        [rec.dataset("few", sorted(few, key=lambda r: r["run_id"]))],
    )
    rec.case(
        "the test slice is under the floor",
        ["train", "dataset.jsonl", "model.json", "--test-from", "2026-09-29T12:00"],
        [main],
    )
    rec.case(
        "every row is in the fit slice",
        ["train", "dataset.jsonl", "model.json", "--test-from", "2027-01-01"],
        [main],
    )
    rec.case(
        "no verdicts in the fit slice",
        ["train", "dataset.jsonl", "--label", "verdict", "--test-from", "2026-09-05T06:00"],
        [main],
    )
    rec.case("an empty dataset", ["train", "dataset.jsonl", "model.json"], [rec.dataset("empty", data=b"")])
    rec.case("no dataset", ["train", "missing.jsonl", "model.json", "--label", "task"])
    rec.case(
        "a dataset that is a directory",
        ["train", "dir", "model.json"],
        [{"path": "dir/inside.jsonl", "hex": hexed(b"{}\n")}],
    )

    # A launch time is the float `millis / 1000`, rounded to microseconds: the last
    # millisecond of year 9999 lands on .998993, so it is in the test slice from there and
    # in the fit slice from one microsecond later.
    edge = dump([{"run_id": f"{253402300799999:012x}", "outcome": "failed"}])
    edge = {"path": "dataset.jsonl", "hex": hexed(edge)}
    for stamp in ("9999-12-31T23:59:59.998993", "9999-12-31T23:59:59.998994", "9999-12-31T23:59:59.999"):
        rec.case(f"the last millisecond, from {stamp}", ["train", "dataset.jsonl", "m.json", "--test-from", stamp], [edge])

    # One labelled row each: the refusal says which slice it fell in, or the id does not read.
    for n, ident in enumerate(
        [
            "a", "_1", "0x_1f", "0x1f", "1__2", "1_", " 1f", "+1f", "１２", "", "1f ",
            "\x1c1", "0X1F", "0x", "1_f", "--1-f--", "zz", "ffffffffffff", "e677d21fdbff",
            "e677d21fdc00", "\ud800", "0199d3f2-a1b0-7000-8000-000000000000", "1f g", "0_1", "+", "0x+1",
            "١٢", "1 ", "g", "0199D3F2A1B0FF", "-0x1f", "0b11", "0o17", "1 ", "\t\n1f\r\x0b\x0c",
        ]
    ):
        rec.case(
            f"a run id of {ident!r}",
            ["train", "dataset.jsonl", "m.json", "--test-from", "2000-01-01"],
            [{"path": "dataset.jsonl", "hex": hexed(dump([{"run_id": ident, "outcome": "failed"}]))}],
        )

    # Rows of the wrong shape, where the Python ends in a traceback.
    labelled = [n for n, r in enumerate(rows) if r.get("outcome") not in (None, "stopped")]

    def spoiled(change, where="fit") -> dict:
        at = labelled[3] if where == "fit" else labelled[-3]
        return patched(at, change(dict(rows[at]), lines[at]))

    def setter(key, value):
        return lambda r, _: dump([dict(r, **{key: value})])

    def dropper(key):
        return lambda r, _: dump([{k: v for k, v in r.items() if k != key}])

    shapes = [
        ("a row that is a list", lambda _, line: b"[1, 2]\n" + line, "fit"),
        ("a row that is a string", lambda _, line: line + b'"text"\n', "test"),
        ("a row with no run id", dropper("run_id"), "fit"),
        ("a run id that is a number", setter("run_id", 12), "test"),
        ("a run id that is null", setter("run_id", None), "fit"),
        ("a run id that does not read", setter("run_id", "not-a-run-id"), "fit"),
        ("a run id past year 9999", setter("run_id", "ffffffff-ffff-7000-8000-000000000000"), "test"),
        ("a task that is a number, fit", setter("task_text", 7), "fit"),
        ("a task that is a number, test", setter("task_text", 7.5), "test"),
        ("a task that is true, test", setter("task_text", True), "test"),
        ("a provider that is a list, fit", setter("provider", ["claude"]), "fit"),
        ("a provider that is a list, test", setter("provider", ["claude"]), "test"),
        ("a model that is an object, test", setter("model", {"name": "opus"}), "test"),
        ("an outcome that is a list", setter("outcome", ["failed"]), "fit"),
    ]
    for name, change, where in shapes:
        rec.case(name, ["train", "dataset.jsonl", "model.json"], [spoiled(change, where)])
    rec.case(
        "a verdict that is a list",
        ["train", "dataset.jsonl", "model.json", "--label", "verdict"],
        [spoiled(setter("human_verdict", ["fixed"]))],
    )
    rec.case(
        "a garbled line",
        ["train", "dataset.jsonl", "model.json"],
        [patched(50, b"not json\n" + lines[50])],
    )
    rec.case(
        "a line that is not utf-8",
        ["train", "dataset.jsonl", "model.json"],
        [patched(50, b'{"run_id": "caf\xe9"}\n')],
    )

    # The command line.
    for n, argv in enumerate(
        [
            [], ["x"], ["trai", "d"], ["--foo", "train", "d"], ["--foo"], ["train"], ["score"],
            ["score", "m"], ["score", "m", "r", "x"], ["train", "d", "--label", "bad"],
            ["train", "d", "--label"], ["train", "d", "--label", "--test-from"],
            ["train", "d", "--test-from", "nope"], ["train", "d", "--t", "2026"],
            ["train", "d", "m", "x", "y"], ["train", "--foo", "d"], ["train", "--label", "task"],
            ["train", "d", "--test-from", "-1"], ["train", "d", "--test-from", "-1_0"],
            ["train", "d", "--test-from", "-.5"], ["train", "d", "--test-from", "-1."],
            ["train", "d", "-x"], ["train", "d", "--label="], ["train", "-l", "task", "d"],
            ["train", "d", "--test-from", "café 'q'"], ["score", "--label", "x", "m", "r"],
            ["train", "d", "m", "--foo=1", "z"], ["train", "d", "--label", "task", "e1", "e2"],
            ["train", "d", "--test-from", "2026-09-01 x"], ["train", "d", "--test-from", ""],
            ["train", "d", "--label", "--"], ["train", "d", "--label", "-"], ["train", "d", "--=x"],
            ["train", "d", "--help=x"], ["train", "d", "-hx"], ["train", "d", "--label", "bad", "-h"],
            ["train", "d", "--labels", "task"], ["train", "d", "--label", "Outcome"],
            ["score", "m", "r", "--x y", "-5", "--zed"], ["-x", "score", "m", "r", "q"],
            ["train", "--label", "task", "--label"], ["score", "--", "-h"],
        ]
    ):
        rec.case(f"arguments {n:02d}: {' '.join(argv)}", argv)
    # Arguments that parse, on a dataset that is not there: the refusal names the label.
    for n, argv in enumerate(
        [
            ["train", "d", "--lab", "verdict", "m"], ["train", "d", "--l=verdict"],
            ["train", "d", "--", "--label"], ["train", "--", "d", "m"], ["train", "d", "-1"],
            ["train", "d", "--label=task", "--label", "outcome"], ["train", "a b", "--foo bar"],
            ["train", "d", "--test-from=2026-09-01", "m"], ["train", "-", "--label", "task"],
            ["train", "d", "--label", "task", "--test-from", "20260901T1200"],
        ]
    ):
        rec.case(f"arguments that parse {n:02d}: {' '.join(argv)}", argv)

    # ------------------------------------------------------------------ score
    trained = {
        "outcome": "train outcome on main",
        "verdict": "train verdict on main",
        "task": "train task on main",
        "odd": "train outcome on odd rows",
    }
    for which, source in trained.items():
        model = {"path": "model.json", "model": source}
        for name in corpus:
            rec.case(
                f"score {name} with the {which} model",
                ["score", "model.json", "record.jsonl"],
                [model, {"path": "record.jsonl", "scan": name}],
            )
    rng = random.Random(2134)
    for n, r in enumerate(rng.sample(rows, 24) + rng.sample(odd_dataset(), 12)):
        task = r.get("task_text")
        line = start_line(
            r["run_id"], r["provider"], r["model"], task, r.get("policy"), ["run", "resume"][n % 2]
        )
        if not r["provider"] or not r["run_id"]:
            continue
        which = ["outcome", "verdict", "task", "odd"][n % 4]
        rec.case(
            f"score a record of row {n} with the {which} model",
            ["score", "model.json", "record.jsonl"],
            [
                {"path": "model.json", "model": trained[which]},
                {"path": "record.jsonl", "hex": hexed(record_bytes(line))},
            ],
        )

    model = {"path": "model.json", "model": trained["outcome"]}
    good = record_bytes(start_line("0199d3f2-a1b0-7000-8000-000000000001", "claude", "Sonnet 5", PREAMBLE + "do it"))
    plain = {"path": "record.jsonl", "hex": hexed(good)}
    by_id = {"path": "root/runs/0199d3f2-a1b0-7000-8000-000000000001.jsonl", "hex": hexed(good)}
    rec.case("score a run id under the root", ["score", "model.json", "0199d3f2-a1b0-7000-8000-000000000001"], [model, by_id])
    rec.case(
        "score a run id under a root named another way",
        ["score", "model.json", "0199d3f2-a1b0-7000-8000-000000000001"],
        [model, {"path": "else/where/runs/0199d3f2-a1b0-7000-8000-000000000001.jsonl", "hex": hexed(good)}],
        env={"HYDRA_ROOT": "else/where"},
    )
    rec.case("score a run id nobody has", ["score", "model.json", "0199d3f2"], [model, by_id])
    rec.case("score a record that is a directory", ["score", "model.json", "root"], [model, by_id])
    for name, data in [
        ("an evolve record", record_bytes(start_line("a", "claude", "x", "t", command="evolve"))),
        ("an empty record", b""),
        ("a torn first line", b'{"run_id": "a", "provider"'),
        ("a garbled record", b"not json\n"),
        ("a record that is not utf-8", b'{"run_id": "caf\xe9"}\n'),
        ("a record whose line 0 is a list", b"[1]\n"),
        ("a record with no provider", record_bytes({"run_id": "a", "hydra": {"command": "run"}})),
        ("a record whose hydra is a string", record_bytes({"run_id": "a", "provider": "claude", "hydra": "run"})),
    ]:
        rec.case(
            f"score {name}",
            ["score", "model.json", "./recs//bad.jsonl"],
            [model, {"path": "recs/bad.jsonl", "hex": hexed(data)}],
        )
    for name, line in [
        ("a record with no task", start_line("a", "claude", "Opus 5.5", None)),
        ("a record whose task is a list", start_line("a", "claude", None, ["Hydra launched this run", "x"])),
        ("a record whose task is a number", start_line("a", "claude", None, 7)),
        ("a record whose model is a list", start_line("a", "claude", ["x"], "t")),
        ("a record whose provider is a number", start_line("a", 7, "x", "t")),
        ("a record with no policy", {"run_id": "a", "provider": "codex", "hydra": {"command": "run", "task": "t" * 5000}}),
        ("a record with non-ascii text", start_line("café \U0001f600", "café", "\ud800", "café " * 99)),
    ]:
        rec.case(f"score {name}", ["score", "model.json", "record.jsonl"], [model, plain | {"hex": hexed(record_bytes(line))}])

    # A model file somebody edited.
    base = models["outcome"]
    at = base["vocab"].index("log_task_chars")
    weights = base["weights"]
    for name, data in [
        ("no model file", None),
        ("a model that is not json", b"not json"),
        ("a model that is a list", b"[1, 2]\n"),
        ("a model that is null", b"null\n"),
        ("a model with no version", edited(base, version=...)),
        ("a model with no label", edited(base, label=...)),
        ("a model with no scale", edited(base, scale=...)),
        ("a model with no weights", edited(base, weights=...)),
        ("a model with no bias", edited(base, bias=...)),
        ("a model with no vocab", edited(base, vocab=...)),
        ("a model with whole-number weights", edited(base, weights=[round(w) for w in weights], bias=1)),
        ("a model with true and false for weights", edited(base, weights=[w > 0 for w in weights], bias=True)),
        ("a model with a huge whole weight", edited(base, weights=[10**40] + weights[1:], bias=-(10**39))),
        ("a model with a weight past a float", edited(base, weights=[10**400] + weights[1:])),
        ("a model with a text weight", edited(base, weights=["1.5"] + weights[1:])),
        ("a model with a text weight past the vocabulary", edited(base, weights=weights + ["x", None])),
        ("a model with fewer weights", edited(base, weights=weights[:3])),
        ("a model with no weights at all", edited(base, weights=[])),
        ("a model with a NaN weight", edited(base, weights=[float("nan")] + weights[1:])),
        ("a model with an infinite bias", edited(base, bias=float("inf"))),
        ("a model with a bias of minus infinity", edited(base, bias=float("-inf"))),
        ("a model with a text bias", edited(base, bias="0.5")),
        ("a model with a null bias", edited(base, bias=None)),
        ("a model with a deviation of zero", edited(base, scale={"log_task_chars": [5.0, 0.0]})),
        ("a model with a deviation of whole zero", edited(base, scale={"log_task_chars": [5.0, 0]})),
        ("a model with a whole-number scale", edited(base, scale={"log_task_chars": [5, 2]})),
        ("a model with a scale of three", edited(base, scale={"log_task_chars": [5.0, 1.0, 2.0]})),
        ("a model with a scale of one", edited(base, scale={"log_task_chars": [5.0]})),
        ("a model with a scale that is a number", edited(base, scale={"log_task_chars": 5.0})),
        ("a model with a scale that is text", edited(base, scale={"log_task_chars": "ab"})),
        ("a model with a scale of text", edited(base, scale={"log_task_chars": ["5.0", 1.0]})),
        ("a model with a scale that is a list", edited(base, scale=[["log_task_chars", [5.0, 1.0]]])),
        ("a model with a scale for a name no row has", edited(base, scale={"elsewhere": 7, "log_task_chars": [5.0, 1.0]})),
        ("a model that scales the preamble", edited(base, scale={"preamble": [0.5, 0.25], "provider=claude": [1, 4]})),
        ("a model with an empty scale", edited(base, scale={})),
        ("a model with odd names in its vocabulary", edited(base, vocab=[7, None, True, 1.5] + base["vocab"][4:])),
        ("a model with a list in its vocabulary", edited(base, vocab=[["x"]] + base["vocab"][1:])),
        ("a model whose vocabulary is text", edited(base, vocab="preamble")),
        ("a model whose vocabulary is an object", edited(base, vocab={"preamble": 1, "log_task_chars": 2})),
        ("a model whose vocabulary is a number", edited(base, vocab=7)),
        ("a model whose weights are a number", edited(base, weights=7)),
        ("a model whose weights are an object", edited(base, weights={"a": 1})),
        ("a model with a label and a version of any shape", edited(base, label=["x"], positive=None, version={"v": 1.5})),
        ("a model with only the scaled name", edited(base, vocab=["log_task_chars"], weights=[weights[at]], bias=0.0)),
        ("a model with a huge weight on the length", edited(base, weights=weights[:at] + [1e6] + weights[at + 1 :])),
        ("a model with a huge negative weight on the length", edited(base, weights=weights[:at] + [-1e6] + weights[at + 1 :])),
    ]:
        files = [plain] if data is None else [plain, {"path": "model.json", "hex": hexed(data)}]
        rec.case(f"score with {name}", ["score", "model.json", "record.jsonl"], files)

    # Build 220, recorded last so no earlier case moves: where `argparse` keeps the first
    # `--` and where it drops it. It drops it while it fills positionals, keeps it after a
    # surplus argument, and a second one is an argument like any other.
    for n, argv in enumerate(
        [
            ["score", "m", "r", "--", "x"], ["score", "m", "--", "r", "x"], ["score", "m", "r", "x", "--", "y"],
            ["score", "x", "y", "z", "--", "w"], ["score", "m", "--", "r", "--", "x"], ["score", "--", "--", "m", "r"],
            ["score", "m", "--", "--", "r"], ["train", "d", "m", "--", "--label"], ["train", "--", "d", "--", "m"],
            ["train", "--label", "task", "--", "d", "m", "x"], ["train", "d", "x", "y", "--", "z"],
            ["score", "m", "r", "--foo", "--", "x"],
        ]
    ):
        rec.case(f"the separator {n:02d}: {' '.join(argv)}", argv)

    directory = out / "baseline"
    directory.mkdir(exist_ok=True)
    for stale in directory.glob("*.jsonl"):
        stale.unlink()
    for name, data in rec.datasets.items():
        (directory / f"{name}.jsonl").write_bytes(data)
    (out / "baseline.json").write_text(json.dumps(rec.cases, indent=1) + "\n", encoding="utf-8")
