"""Record what `puppets.py` gave, so `differential.rs` can compare `hydra-puppets` with it.

Build ticket 196 deleted `puppets.py`. This script ran it once, just before, on every input
of the differential tests, and wrote the result under `fixtures/`. It needs a checkout that
still has `integrations/puppets/puppets.py` and its tests, so today it runs only against one
taken from history. Three commands, in the order they were run:

    python3 record.py harvest <integrations/puppets/tests> <harvest-dir>
    HYDRA_PUPPETS_JOBS=<jobs-dir> HYDRA_PUPPETS_HARVEST=<harvest-dir> \\
        cargo test -p hydra-puppets --test differential -- --ignored write_the_jobs
    python3 record.py run <jobs-dir> <integrations/puppets> <fixtures-dir>

`harvest` runs the Python tests and keeps every goal log they built: just before each test's
tearDown removes its temp home, the goal logs in `HYDRA_PUPPETS_HOME`, the scores file in
`HYDRA_GRADER_HOME` and the vendors file in `HYDRA_PUPPETS_VENDORS` are copied to
`<harvest-dir>/<n>/`. Whether a test passes does not matter, only what it left on disk.

The ignored Rust test writes one directory per case under `<jobs-dir>/<group>/`: the home
the command starts from and a `job.json` with its arguments, environment and working
directory. `run` copies the Python beside the frozen data files of each group, runs every
job with nothing in the environment but what `job.json` names, and writes
`<fixtures-dir>/<group>/<name>.json`: each step's exit code, stdout and stderr, and the files
left behind. Nothing is normalized here. The comparator does that when it reads the file.

For the jobs that read a goal log it also records the labels the grader derived from the
Python fold (`integrations/grader/labels.py` as it was before map 93), which
`integrations/grader/tests/test_labels_recorded.py` checks the shell-out against.

A fourth command compares the two ways of labelling over any puppets home, read only:

    python3 record.py labels <integrations/puppets/puppets.py> <puppets-home>...
"""

import importlib.util
import io
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import time
import unittest
from pathlib import Path

HARVESTED = [
    "test_puppets",
    "test_approve_revision",
    "test_panel_degraded",
    "test_queue",
    "test_repair",
    "test_goal_log_tail",
    "test_damaged_log",
]

REPO = Path(__file__).resolve().parents[3]


def harvest(tests_dir: Path, out: Path) -> int:
    sys.path.insert(0, str(tests_dir))
    sys.path.insert(0, str(tests_dir.parent))
    counter = [0]

    def snapshot() -> None:
        home = os.environ.get("HYDRA_PUPPETS_HOME")
        if not home or not Path(home).is_dir():
            return
        logs = sorted(p for p in Path(home).glob("*.jsonl") if p.is_file())
        if not logs:
            return
        counter[0] += 1
        case = out / f"{counter[0]:04d}"
        (case / "puppets").mkdir(parents=True)
        for log in logs:
            shutil.copyfile(log, case / "puppets" / log.name)
        grader = os.environ.get("HYDRA_GRADER_HOME")
        if grader and (Path(grader) / "scores.jsonl").is_file():
            (case / "grader").mkdir()
            shutil.copyfile(Path(grader) / "scores.jsonl", case / "grader" / "scores.jsonl")
        vendors = os.environ.get("HYDRA_PUPPETS_VENDORS")
        if vendors and Path(vendors).is_file():
            shutil.copyfile(vendors, case / "vendors.json")

    original = unittest.TestCase._callTearDown

    def call_tear_down(self):
        snapshot()
        original(self)

    unittest.TestCase._callTearDown = call_tear_down
    suite = unittest.defaultTestLoader.loadTestsFromNames(HARVESTED)
    unittest.TextTestRunner(stream=io.StringIO(), verbosity=0).run(suite)
    print(counter[0])
    return 0


def blob(raw: bytes) -> dict:
    """Bytes as JSON: the text when they are UTF-8, their hex when they are not."""
    try:
        return {"text": raw.decode("utf-8")}
    except UnicodeDecodeError:
        return {"hex": raw.hex()}


def files_under(home: Path, keep: list[str]) -> list[dict]:
    found = []
    for top in keep:
        base = home / top
        if base.is_file():
            found.append(base)
        elif base.is_dir():
            found.extend(p for p in base.rglob("*") if p.is_file())
    return [
        {"path": str(p.relative_to(home)), "bytes": blob(p.read_bytes())}
        for p in sorted(found, key=lambda p: str(p.relative_to(home)))
    ]


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def fold_events(path: Path) -> list:
    """`committed_events` of `labels.py` before map 93: every terminated line, a bad last
    one passed over."""
    if not path.is_file():
        return []
    events = []
    lines = path.read_bytes().splitlines(keepends=True)
    for index, line in enumerate(lines):
        if not line.endswith(b"\n"):
            break
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            if index == len(lines) - 1:
                break
            raise
    return events


def fold_labels(puppets, goal_id: str, events: list) -> dict:
    """`labels_for_goal` of `labels.py` before map 93: the labels read from `puppets.fold`."""
    state = puppets.fold(goal_id, events)
    goal_text = (state.goal or {}).get("text", "")
    labels = {}
    for task in state.tasks.values():
        if not task.run_id:
            continue
        labels[task.run_id] = {
            "task_id": task.id,
            "class": task.task_class,
            "grade": task.grade,
            "human_verdict": task.human_result,
            "text": task.text,
        }
    for plan in state.plans:
        labels[plan["run_id"]] = {
            "task_id": None,
            "class": "plan_proposer",
            "grade": None,
            "human_verdict": None,
            "text": goal_text,
        }
    for event in events:
        name = event.get("event")
        if name == "panel_verdict":
            for run_id in event.get("judge_run_ids", []):
                labels[run_id] = {
                    "task_id": None,
                    "class": "plan_judge",
                    "grade": None,
                    "human_verdict": None,
                    "text": goal_text,
                }
        elif name == "review":
            task = state.tasks.get(event.get("task"))
            for run_id in event.get("judge_run_ids", []):
                labels[run_id] = {
                    "task_id": event.get("task"),
                    "class": task.task_class if task else None,
                    "grade": task.grade if task else None,
                    "human_verdict": None,
                    "text": task.text if task else "",
                }
    return labels


def pairs(labels: dict) -> str:
    """Labels as text two runs can be compared by: every pair in order, and NaN as NaN."""
    return json.dumps([[run_id, row] for run_id, row in labels.items()])


def labels_by_fold(puppets, log: Path) -> dict:
    try:
        return {"pairs": pairs(fold_labels(puppets, log.stem, fold_events(log)))}
    except Exception as err:  # noqa: BLE001
        return {"raises": type(err).__name__}


def run_jobs(jobs: Path, python_dir: Path, fixtures: Path) -> int:
    count = 0
    jobs = jobs.resolve()
    # What a child names as its interpreter, which `pending --json` prints in an `argv`. It
    # is not always this process's `sys.executable`: Homebrew's `python3` is a link.
    python = subprocess.run(
        [sys.executable, "-c", "import sys; print(sys.executable)"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    for root in sorted(p for p in jobs.iterdir() if p.is_dir()):
        if root.name == "python":
            record_python(root, python_dir, fixtures / "python")
            continue
        for name in ("puppets.py", "sheet.py", "models.py"):
            shutil.copyfile(python_dir / name, root / "data" / name)
        script = root / "data" / "puppets.py"
        puppets = load_module(f"recorded_puppets_{root.name}", script)
        out = fixtures / root.name
        if out.exists():
            shutil.rmtree(out)
        out.mkdir(parents=True)
        for job_path in sorted(root.glob("*/job.json")):
            job = json.loads(job_path.read_text(encoding="utf-8"))
            home = Path(job["home"])
            recorded = {
                "label": job["label"],
                "goal": job["goal"],
                "root": job["root"],
                "home": job["home"],
                "python": python,
                "script": str(script),
            }
            if job["seed"]:
                recorded["seed"] = files_under(home, ["puppets", "grader", "vendors.json"])
                recorded["labels"] = labels_by_fold(
                    puppets, home / "puppets" / f"{job['goal']}.jsonl"
                )
            steps = []
            for args in job["steps"]:
                done = subprocess.run(
                    [sys.executable, str(script), *args],
                    env=job["env"],
                    cwd=job["cwd"],
                    stdin=subprocess.DEVNULL,
                    capture_output=True,
                )
                steps.append(
                    {
                        "args": args,
                        "code": done.returncode,
                        "stdout": blob(done.stdout),
                        "stderr": blob(done.stderr),
                    }
                )
            recorded["steps"] = steps
            recorded["files"] = files_under(home, job["keep"])
            (out / f"{job_path.parent.name}.json").write_text(
                json.dumps(recorded, indent=1) + "\n", encoding="utf-8"
            )
            count += 1
    print(count)
    return 0


def record_python(root: Path, python_dir: Path, out: Path) -> None:
    """What the interpreter itself answers: `sorted`, `str.isprintable`, `int`,
    `math.lgamma`, and `sheet.beta_lower`, which stays in Python."""
    out.mkdir(parents=True, exist_ok=True)
    (out / "version.txt").write_text(sys.version + "\n")

    lines = []
    for line in (root / "sort-cases.jsonl").read_text().splitlines():
        xs = json.loads(line)
        lines.append(json.dumps(sorted(range(len(xs)), key=xs.__getitem__)))
    (out / "sorted.txt").write_text("\n".join(lines) + "\n")

    ranges, start = [], None
    for c in range(0x110000):
        hidden = not chr(c).isprintable()
        if hidden and start is None:
            start = c
        if not hidden and start is not None:
            ranges.append((start, c - 1))
            start = None
    ranges.append((start, 0x10FFFF))
    (out / "isprintable.txt").write_text("".join(f"{low} {high}\n" for low, high in ranges))

    def read(text):
        try:
            return str(int(text))
        except ValueError:
            return "-"

    rows = []
    for c in range(0x110000):
        if 0xD800 <= c < 0xE000:
            continue
        ch = chr(c)
        row = [read(ch + "7"), read("7" + ch), read(ch), read(ch + "_7"), read("7_" + ch)]
        if any(cell != "-" for cell in row):
            rows.append("%x %s\n" % (c, " ".join(row)))
    (out / "int.txt").write_text("".join(rows))

    rows = []
    for k in range(1, 6001):
        for x in (float(k), k / 2, k / 7):
            rows.append(struct.pack(">d", math.lgamma(x)).hex() + "\n")
    (out / "lgamma.txt").write_text("".join(rows))

    sheet = load_module("recorded_sheet", python_dir / "sheet.py")
    grid = [(a, b) for a in range(25) for b in range(25)] + [(120, 80), (300, 10), (2, 400)]
    (out / "beta_lower.txt").write_text(
        "".join(
            "%d %d %s\n" % (a, b, struct.pack(">d", sheet.beta_lower(a, b)).hex())
            for a, b in grid
        )
    )


def compare_labels(script: Path, homes: list[Path]) -> int:
    """Label every goal log under each home both ways and print what differs. Read only."""
    puppets = load_module("compared_puppets", script)
    labels = load_module("compared_labels", REPO / "integrations" / "grader" / "labels.py")
    goals = same = count = refused = refused_with_labels = 0
    slowest = 0.0
    differing = []
    for home in homes:
        for log in sorted(home.glob("*.jsonl")):
            goals += 1
            old = labels_by_fold(puppets, log)
            started = time.monotonic()
            try:
                state = labels.show_goal(log.stem, home)
            except labels.LabelsError:
                refused += 1
                if "pairs" in old and old["pairs"] != "[]":
                    refused_with_labels += 1
                    differing.append(f"{log}: show refuses it, the fold labelled it")
                continue
            finally:
                slowest = max(slowest, time.monotonic() - started)
            new = labels.labels_for_goal(state, labels.committed_events(log))
            if old.get("pairs") == pairs(new):
                same += 1
                count += len(new)
            else:
                differing.append(f"{log}: fold {old}, show {pairs(new)}")
    print(f"{goals} goal logs: {same} equal with {count} labels, {refused} refused by show")
    print(f"of the refused, {refused_with_labels} had labels under the Python fold")
    print(f"slowest `show`: {slowest * 1000:.0f} ms")
    for line in differing:
        print(line)
    return 1 if goals != same + refused else 0


def main() -> int:
    command, args = sys.argv[1], [Path(a) for a in sys.argv[2:]]
    if command == "harvest":
        return harvest(*args)
    if command == "run":
        return run_jobs(*args)
    if command == "labels":
        return compare_labels(args[0], args[1:])
    raise SystemExit(__doc__)


if __name__ == "__main__":
    raise SystemExit(main())
