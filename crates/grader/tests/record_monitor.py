"""The part of `record.py` that records `monitor.py`'s three commands (build ticket 215).

`record.py` loads this file and calls `record`. It is not run on its own.

Each case is one command line, run the way `record_baseline.py` runs one: the Python as a
child with nothing in the environment but what the case names, in a scratch directory that
holds the case's files under relative paths. What comes back is the exit code, stdout,
stderr and every file the command wrote. `fixtures/monitor.json` holds the cases, and
`fixtures/monitor/` holds what is too large to repeat in each: the records a `train` joins
its dataset to, as one JSON object per directory of them, and the datasets this file makes.
The dataset build 213 recorded, `fixtures/baseline/main.jsonl`, is read from where it is.

The frames come from three places: the twelve records of `tests/fixtures/records/`, the
frames `integrations/grader/tests/test_monitor.py` builds, and the ones below, which are
shapes a provider could leave and the Python either steps over or ends in a traceback on.

Every case names `HYDRA_ROOT`, so no default path reaches the real `~/hydra`, and the child
reads the frozen `fixtures/data/aliases.json`.
"""

import importlib.util
import json
import random
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

# The child: `monitor.py` loaded as a module, the alias table of the `records` it reaches
# through `baseline` frozen, its `main` run as the script would run it.
CHILD = """
import importlib.util, sys
frozen, script = sys.argv[1], sys.argv[2]
sys.argv = ["monitor.py", *sys.argv[3:]]
spec = importlib.util.spec_from_file_location("recorded_monitor", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
load = module.baseline.records.load_aliases
module.baseline.records.load_aliases = lambda path=None: load(frozen)
raise SystemExit(module.main())
"""

FRACTIONS = ("0.1", "0.25", "0.5")
IDENT = "0199d3f2-a1b0-7000-8000-0000000000e1"
FILLER = {"type": "result"}


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def dump(rows) -> bytes:
    return "".join(json.dumps(r) + "\n" for r in rows).encode()


def hexed(data: bytes) -> str:
    return data.hex()


def blob(data: bytes) -> dict:
    """How a case keeps a file's bytes: as text when they are UTF-8, which is half the size."""
    try:
        return {"text": data.decode("utf-8")}
    except UnicodeDecodeError:
        return {"hex": data.hex()}


def stamp(at: datetime) -> str:
    return at.strftime("%Y-%m-%dT%H:%M:%S.") + f"{at.microsecond // 1000:03d}Z"


def first_line(ident: str, provider="claude", model="Sonnet 5", task="do the thing", **over) -> dict:
    line = {
        "run_id": ident,
        "provider": provider,
        "model": model,
        "ts": "2026-09-24T00:00:00.000Z",
        "hydra": {"type": "run_start", "command": "run", "policy": "standard", "task": task},
    }
    line.update(over)
    return line


def framed(raws: list, provider="claude", pad=True) -> bytes:
    """A record whose every frame is inside the half a monitor at 0.5 reads."""
    lines = [first_line(IDENT, provider)]
    at = datetime(2026, 9, 24, tzinfo=timezone.utc)
    for n, raw in enumerate(raws):
        at += timedelta(seconds=3, milliseconds=250)
        lines.append({"seq": n + 1, "ts": stamp(at), "raw": raw})
    if pad:
        lines += [{"seq": len(lines) + n, "ts": stamp(at), "raw": FILLER} for n in range(len(lines))]
    return dump(lines)


# ------------------------------------------------------------------ frames, by provider


def use(n, name, **inp) -> dict:
    block = {"type": "tool_use", "id": f"u{n}", "name": name}
    if inp:
        block["input"] = inp
    return {"type": "assistant", "message": {"content": [block]}}


def result(ident, text, error=True) -> dict:
    block = {"type": "tool_result", "content": text, "is_error": error}
    if ident is not None:
        block["tool_use_id"] = ident
    return {"type": "user", "message": {"content": [block]}}


def hook(name="PreToolUse:Bash", output=None, **over) -> dict:
    if output is None:
        output = json.dumps({"hookSpecificOutput": {"permissionDecision": "deny"}})
    raw = {"type": "system", "subtype": "hook_response", "hook_name": name, "output": output}
    raw.update(over)
    return raw


def cursor(subtype, call_id, tool_call) -> dict:
    raw = {"type": "tool_call", "subtype": subtype, "tool_call": tool_call}
    if call_id is not None:
        raw["call_id"] = call_id
    return raw


def codex(kind, ident="x1", **item) -> dict:
    body = {"type": kind}
    if ident is not None:
        body["id"] = ident
    body.update(item)
    return {"type": "item.completed", "item": body}


def agy(index, state, **step) -> dict:
    body = {"step_type": "tool", "step_index": index, "state": state}
    body.update(step)
    return {"event": "step_update", "step_update": body}


DENY = "hydra/capability: policy read-only does not allow file_write (Write)."
DENIED = {"hookSpecificOutput": {"permissionDecision": "deny"}}


def odd_frames() -> list[tuple[str, str, list]]:
    """(name, provider, raws): one record each, read at 0.5 with every frame in the prefix."""
    nan = float("nan")
    return [
        # Names and ids of a tool call.
        ("names that are numbers and compare equal", "claude", [
            use(1, 7), use(2, 1.5), use(3, True), use(4, 1), use(5, 1.0), use(6, "1"), use(7, "True"),
        ]),
        ("a name that is a list", "claude", [use(1, "Read"), use(2, ["Read"])]),
        ("a name that is an object", "claude", [use(1, {"tool": "Read"})]),
        ("names that are empty", "claude", [use(1, ""), use(2, None), use(3, 0), use(4, [])]),
        ("a name that is not ascii", "claude", [use(1, "café \U0001f600", path="a"), use(2, "\ud800", path="a")]),
        ("ids that are empty, numbers or missing", "claude", [
            {"type": "assistant", "message": {"content": [
                {"type": "tool_use", "id": "", "name": "Read"},
                {"type": "tool_use", "id": 7, "name": "Read"},
                {"type": "tool_use", "name": "Bash"},
                {"type": "tool_use", "id": "u1", "name": "Write"},
                {"type": "tool_use", "id": "u1", "name": "Edit"},
                "text", 7, None, {"type": "text", "name": "Read"},
            ]}},
            hook("PreToolUse:Read"), hook("PreToolUse:Read"), hook("PreToolUse:Read"),
            hook("PreToolUse:Edit"), hook("PreToolUse:Write"),
        ]),
        ("a message that is no object and content that is no list", "claude", [
            {"type": "assistant", "message": "text"}, {"type": "assistant", "message": {"content": "text"}},
            {"type": "assistant", "message": {"content": {"type": "tool_use", "name": "Read"}}},
            {"type": "assistant"},
        ]),
        # The input a call is keyed on.
        ("inputs of every shape", "claude", [
            {"type": "assistant", "message": {"content": [
                {"type": "tool_use", "id": f"u{n}", "name": "Read", "input": inp}
                for n, inp in enumerate([
                    None, {}, [], "", 0, False, {"b": 1, "a": 2}, {"a": 2, "b": 1}, {"a": 2.0, "b": 1},
                    {"a": {"z": [1, {"y": None, "x": True}]}}, {"a": {"z": [1, {"x": True, "y": None}]}},
                    "café", "café", "\ud800", {"\U0001f600": "é", "a": "\x7f\x00"}, 1e16, 1e-7, -0.0,
                    10**40, [1, 2], [2, 1], {"": None}, {}, None,
                ])
            ]}},
        ]),
        ("an input that holds a NaN and an infinity", "claude", [
            use(1, "Read", x=nan), use(2, "Read", x=nan), use(3, "Read", x=float("inf")),
            use(4, "Read", x=float("-inf")),
        ]),
        ("a streak, a break and a streak again", "claude", [
            use(1, "Read", path="a"), use(2, "Read", path="a"), use(3, "Bash"), use(4, "Read", path="a"),
            use(5, "Read", path="a"), use(6, "Read", path="a"), use(7, "Bash"), use(8, "Bash"),
        ]),
        # Usage.
        ("usage in every place a provider parks it", "claude", [
            {"type": "result", "usage": {"output_tokens": 1}},
            {"type": "x", "result": {"usage": {"output_tokens": 2}}},
        ]),
        ("usage under step_update, in camel case", "agy", [
            {"event": "other", "step_update": {"usage": {"outputTokens": 31}}},
        ]),
        ("usage under message wins over the frame's", "claude", [
            {"usage": {"output_tokens": 5}, "result": {"usage": {"outputTokens": 6}},
             "step_update": {"usage": None}, "message": {"usage": {"output_tokens": 7, "outputTokens": 8}}},
        ]),
        ("a null count hides the camel case one", "claude", [
            {"usage": {"output_tokens": 9}}, {"usage": {"output_tokens": None, "outputTokens": 10}},
        ]),
        ("counts that are not whole numbers", "claude", [
            {"usage": {"output_tokens": 12.9}}, {"usage": {"output_tokens": True}},
            {"usage": {"output_tokens": "40"}}, {"usage": {"output_tokens": [1]}}, {"usage": [1]},
            {"usage": "text"}, {"result": "text", "message": 7},
        ]),
        ("a count that is a negative fraction", "claude", [{"usage": {"output_tokens": -0.5}}]),
        ("a count that is a float past any integer type", "claude", [{"usage": {"output_tokens": 1e40}}]),
        ("a count that is a huge float", "claude", [{"usage": {"output_tokens": 1.7976931348623157e308}}]),
        ("a count that is a huge whole number", "claude", [{"usage": {"output_tokens": 10**50}}]),
        ("a count that is zero", "claude", [{"usage": {"output_tokens": 5}}, {"usage": {"output_tokens": 0}}]),
        ("a count that is NaN", "claude", [{"usage": {"output_tokens": nan}}]),
        ("a count that is infinite", "claude", [{"usage": {"output_tokens": float("inf")}}]),
        # Hooks.
        ("a hook whose output is an object", "claude", [use(1, "Bash"), hook(output=DENIED)]),
        ("a hook that allows", "claude", [
            use(1, "Bash"), hook(output={"hookSpecificOutput": {"permissionDecision": "allow"}}),
        ]),
        ("a hook whose output does not parse", "claude", [use(1, "Bash"), hook(output="not json")]),
        ("a hook whose output is a list, a number or missing", "claude", [
            hook(output="[1]"), hook(output="7"), hook(output=7), hook(output=[DENIED]),
            {"subtype": "hook_response"},
        ]),
        ("a hook that denies at the top", "claude", [
            use(1, "Write"), hook("PreToolUse:Write", output={"permissionDecision": "deny"}),
            hook("x:y", output={"hookSpecificOutput": {}, "permissionDecision": "deny"}),
            hook("x:z", output={"hookSpecificOutput": "text", "permissionDecision": "deny"}),
            hook("x:w", output={"hookSpecificOutput": {"permissionDecision": "ask"}, "permissionDecision": "deny"}),
        ]),
        ("a hook that names no tool, then one that does", "claude", [
            use(1, "Bash"), hook("PreToolUse"), hook("PreToolUse:Bash"),
        ]),
        ("a hook with no name at all", "claude", [use(1, "Bash"), hook(None), hook(""), hook(0)]),
        ("a hook named with two colons", "claude", [hook("a:b:c"), hook(":"), hook("a:")]),
        ("a hook for a tool nobody opened", "claude", [hook("PreToolUse:Write"), hook("PreToolUse:Bash")]),
        ("a hook whose name is a number", "claude", [hook(7)]),
        ("a hook whose name is a list with a colon", "claude", [hook([":"])]),
        ("a hook whose name is a list", "claude", [use(1, "Bash"), hook(["x"])]),
        ("a hook whose name is an object with a colon", "claude", [hook({":": 1})]),
        ("a hook whose name is an object", "claude", [hook({"x": 1})]),
        ("a hook whose output is a number of 4301 digits", "claude", [hook(output="9" * 4301)]),
        ("a hook closes the last open call of its tool", "claude", [
            use(1, "Bash", command="a"), use(2, "Bash", command="b"), use(3, "Read"),
            hook("PreToolUse:Bash"), result("u2", DENY), result("u1", "fine", False), result("u3", DENY),
        ]),
        # Results.
        ("a result that denies an open call", "claude", [use(1, "Write"), result("u1", DENY)]),
        ("a result that is not an error", "claude", [use(1, "Write"), result("u1", DENY, False)]),
        ("a result that only allows", "claude", [
            use(1, "Bash"), result("u1", "hydra/allow"), use(2, "Bash"), result("u2", "hydrahydra/allow/x"),
            use(3, "Read"), result("u3", "hydra/allow and hydra/path: refused"),
        ]),
        ("a result for a call nobody opened", "claude", [result("u9", DENY), result(None, DENY), result(None, DENY)]),
        ("a result with an id that is a number", "claude", [use(1, "Write"), result(7, DENY), result("", DENY)]),
        ("a result whose content is parts", "claude", [
            use(1, "Write"),
            result("u1", [{"type": "text", "text": "hydra/"}, "skipped", {"type": "image"}, {"text": "capability"}]),
            use(2, "Read"), result("u2", [{"text": "hydra/allow"}, {"text": ""}]),
            use(3, "Read"), result("u3", [{"text": "hydra"}, {"text": "/path"}]),
            use(4, "Read"), result("u4", {"text": DENY}), use(5, "Read"), result("u5", None),
        ]),
        ("a result whose part is a number", "claude", [use(1, "Write"), result("u1", [{"text": "a"}, {"text": 7}])]),
        # cursor-agent.
        ("cursor: every listed tool and two stems", "cursor", [
            cursor("started", f"c{n}", {key: {"args": {"n": n % 3}}})
            for n, key in enumerate([
                "readToolCall", "shellToolCall", "editToolCall", "deleteToolCall", "grepToolCall", "globToolCall",
                "lsToolCall", "webSearchToolCall", "webFetchToolCall", "mcpToolCall", "ToolCall", "todoToolCalls",
            ])
        ]),
        ("cursor: a listed key wins over the order of the object", "cursor", [
            cursor("started", "c1", {"zToolCall": {"args": {"a": 1}}, "lsToolCall": 7, "readToolCall": {}}),
            cursor("started", "c2", {"zToolCall": 7, "aToolCall": {"args": [1]}, "bToolCall": {"args": {"b": 2}}}),
            cursor("started", "c3", {"zToolCall": 7, "aToolCall": {"args": [1]}, "bToolCall": {"args": {"b": 2}}}),
        ]),
        ("cursor: a start seen twice and a completion seen alone", "cursor", [
            cursor("started", "c1", {"readToolCall": {"args": {"path": "a"}}}),
            cursor("started", "c1", {"readToolCall": {"args": {"path": "a"}}}),
            cursor("completed", "c1", {"readToolCall": {"result": {"success": {}}}}),
            cursor("completed", "c2", {"shellToolCall": {"result": {"failure": {"stderr": "hydra/path: no"}}}}),
            cursor("completed", "c2", {"shellToolCall": {"result": "hydra/path: no"}}),
            cursor("other", "c3", {"readToolCall": {}}),
        ]),
        ("cursor: calls with no id", "cursor", [
            cursor("completed", None, {"readToolCall": {"result": {"error": "hydra/capability: no"}}}),
            cursor("started", None, {"readToolCall": {}}),
            cursor("started", None, {"shellToolCall": {}}),
            cursor("completed", None, {"readToolCall": {"result": {"error": {"m": "hydra/allow"}}}}),
            cursor("completed", None, {"shellToolCall": {"result": {"error": None, "failure": ["hydra/x"]}}}),
            cursor("completed", None, {"shellToolCall": {}}),
            cursor("completed", None, {"editToolCall": {"result": "hydra/capability: no"}}),
        ]),
        ("cursor: the id rides in the tool call", "cursor", [
            cursor("started", None, {"toolCallId": "t1", "readToolCall": {"args": {"toolCallId": "t1", "path": "a"}}}),
            cursor("started", "", {"toolCallId": "t2", "readToolCall": {"args": {"toolCallId": "t2", "path": "a"}}}),
            cursor("started", 7, {"toolCallId": 8, "readToolCall": {"args": None}}),
            cursor("started", "", {"toolCallId": "", "readToolCall": {"args": "text"}}),
            cursor("completed", None, {"toolCallId": "t1", "readToolCall": {"result": "hydra/x"}}),
        ]),
        ("cursor: a tool call that is no object", "cursor", [
            cursor("started", "c1", "text"), cursor("completed", "c1", None), cursor("completed", None, [1]),
            {"type": "tool_call", "subtype": "completed"}, cursor("started", "c2", {}),
            cursor("completed", "c3", {"other": {"result": "hydra/x"}}),
        ]),
        # codex.
        ("codex: every kind", "codex", [
            codex("command_execution", "x1", command="ls", aggregated_output="README\n"),
            codex("command_execution", "x2", command="ls", aggregated_output="hydra/path: refused"),
            codex("command_execution", "x3", command=["ls", "-l"], output="hydra/path: refused"),
            codex("command_execution", "x4", aggregated_output=7, output="hydra/allow"),
            codex("web_search", "x5", query="q"), codex("apply_patch", "x6"), codex("file_change", "x7"),
            codex("mcp_tool_call", "x8", name="search"), codex("mcp_tool_call", "x9", name=7),
            codex("mcp_tool_call", "x10", name=""), codex("mcp_tool_call", "x11"),
            codex("agent_message", "x12", text="hydra/x"), codex(None, "x13"), codex(7, "x14"),
            codex("command_execution", None, command="ls"), codex("command_execution", 7, command="ls"),
            {"type": "item.completed", "item": "text"}, {"type": "item.completed"},
            {"type": "turn.completed", "usage": {"output_tokens": 44}},
        ]),
        ("codex: a kind that is a list", "codex", [codex(["command_execution"])]),
        ("codex: a kind that is an object", "codex", [codex({"a": 1})]),
        # agy.
        ("agy: steps", "agy", [
            agy(1, "ACTIVE", tool_name="run_command", tool_info={"parameters": {"CommandLine": "ls"}}),
            agy(1, "DONE", tool_name="run_command", tool_info={"parameters": {"CommandLine": "ls"}}),
            agy(2, "ERROR", tool_name="Edit", tool_info={"error": {"message": "hydra/capability: no"}}),
            agy(3, "ERROR", tool_info={"name": "view_file", "error": {"message": "hydra/allow"}}),
            agy(4, "ERROR", tool_name=7, tool_info={"name": "x", "error": {"message": 7}}),
            agy(5, "DONE", tool_name="", tool_info={"name": ""}),
            agy(None, "DONE", tool_name="a"), agy("5", "ERROR", tool_info="text"),
            agy(5.0, "DONE", tool_name="b"), agy([1], "DONE", tool_name="c"), agy({"a": 1}, "ERROR", tool_name="d"),
            agy(True, "ACTIVE", tool_name="e", tool_info={"error": "text"}),
            {"event": "step_update", "step_update": {"step_type": "text", "step_index": 9}},
            {"event": "step_update", "step_update": "text"}, {"event": "step_update"},
            {"event": "step_update", "step_update": {"step_type": "tool"}},
        ]),
        ("agy: a refusal with no step index", "agy", [
            {"event": "step_update", "step_update": {
                "step_type": "tool", "state": "ERROR", "tool_info": {"error": {"message": "hydra/path: no"}},
            }},
        ]),
        # A frame can look like two providers'. The first test that matches takes it.
        ("a frame of two shapes", "claude", [
            {"type": "tool_call", "subtype": "started", "call_id": "c1", "tool_call": {"readToolCall": {}},
             "event": "step_update", "message": {"content": [{"type": "tool_use", "id": "u1", "name": "Bash"}]}},
            {"type": "item.completed", "event": "step_update", "item": {"type": "web_search", "id": "x"},
             "step_update": {"step_type": "tool", "step_index": 1}},
            {"type": ["tool_call"], "event": "step_update", "step_update": {"step_type": "tool", "step_index": 1}},
        ]),
    ]


def odd_lines() -> list[tuple[str, bytes]]:
    """Records told apart by their lines, not their frames."""
    start = first_line(IDENT)
    seat = {"hydra": {"type": "seat_request", "request_id": "r"}}

    def at(ts, **more) -> dict:
        return dict({"ts": ts, "raw": FILLER}, **more)

    return [
        ("one line", dump([start])),
        ("two lines", dump([start, seat])),
        ("three lines", dump([start, seat, seat])),
        ("eleven lines", dump([start] + [seat] * 10)),
        ("lines that are not objects", dump([start, [1], 7, "text", None, seat, {"raw": "text"}, {"hydra": "text"}] + [FILLER] * 8)),
        ("a seat request after the prefix", dump([start, FILLER, seat, seat])),
        ("times with a fraction and an offset", dump([
            dict(start, ts="2026-09-24T00:00:00.250+02:00"), at("2026-09-23T22:00:01.7505Z"), at("x"), at("x"),
        ])),
        ("times that go backwards", dump([start, at("2026-09-23T23:59:00Z"), at("x"), at("x")])),
        ("a time that does not read is stepped over", dump([
            start, at("not a time"), at(7), at(None), at("2026-09-24T00:01:00.5Z"), at("2026-13-01T00:00:00Z"),
        ] + [at("x")] * 6)),
        ("a first time with no offset and a last with one", dump([dict(start, ts="2026-09-24T00:00:00"), at("2026-09-24T00:00:01Z")] * 2)),
        ("both times with no offset", dump([dict(start, ts="2026-09-24T00:00:00"), at("2026-09-24 00:00:01.5")] * 2)),
        ("only one time reads", dump([dict(start, ts="2026-09-24T00:00:00Z"), at("x"), at("x"), at("x")])),
        ("a time that is a date", dump([dict(start, ts="2026-09-24"), at("2026-09-25"), at("x"), at("x")])),
        ("a time 400 years on", dump([start, at("2426-09-24T00:00:00.000001Z"), at("x"), at("x")])),
        ("a task with the preamble and a model with an alias", dump([
            first_line(IDENT, "claude", "Opus 5.5", "Hydra launched this run and wrote. café \U0001f600 " * 30), seat,
        ])),
        ("a resume with no model and no policy", dump([
            {"run_id": IDENT, "provider": "codex", "hydra": {"command": "resume"}}, FILLER,
        ])),
        ("a model that is a list", dump([first_line(IDENT, "claude", ["x"])])),
        ("a task that is a number", dump([first_line(IDENT, "claude", None, 7)])),
        ("a task that is a list", dump([first_line(IDENT, "claude", None, ["Hydra launched this run", "x"])])),
        ("a run id that is a number", dump([first_line(12, "agy", None)])),
        ("an evolve record", dump([dict(start, hydra={"command": "evolve"})])),
        ("an empty record", b""),
        ("a torn first line", b'{"run_id": "a", "provider"'),
        ("a garbled record", b"not json\n"),
        ("a garbled later line", dump([start]) + b"not json\n" + dump([seat])),
        ("a record that is not utf-8", dump([start]) + b'{"raw": "caf\xe9"}\n'),
        ("a record whose line 0 is a list", b"[1]\n"),
        ("a record whose hydra is a string", dump([{"run_id": "a", "provider": "claude", "hydra": "run"}])),
        ("a torn tail is not an event", dump([start, seat, seat]) + b'{"hydra": {"type": "seat_request"}}'),
    ]


# ------------------------------------------------------------------ records for a dataset

POOL = {
    "Read": [{"path": "README.md"}, {"path": "src/run.rs"}, {"path": "src/run.rs", "limit": 80}],
    "Bash": [{"command": "cargo test"}, {"command": "ls"}, {"command": "git status"}, None],
    "Write": [{"path": "src/run.rs", "content": "x"}, {"path": "notes.md", "content": "é"}],
    "Grep": [{"pattern": "fn main", "path": "src"}, {"path": "src", "pattern": "fn main"}],
}
CURSOR_KEYS = ["readToolCall", "shellToolCall", "editToolCall", "grepToolCall", "mcpToolCall"]
AGY_TOOLS = ["run_command", "view_file", "write_to_file"]


def traced(rng: random.Random, row: dict) -> list[dict]:
    """The lines of one synthesized record. A row that did not complete is refused more
    often and sooner, so the prefix carries something to fit."""
    provider = row["provider"]
    bad = row.get("outcome") not in (None, "completed", "stopped")
    lean = 0.35 if bad else 0.06
    at = datetime(2026, 9, 24, tzinfo=timezone.utc)
    lines = [{
        "run_id": row["run_id"], "provider": provider, "model": row["model"], "ts": stamp(at),
        "hydra": {"type": "run_start", "command": "run", "policy": row.get("policy"), "task": "t"},
    }]
    tokens = 0
    calls = 0

    def push(raw: dict, **more) -> None:
        nonlocal at
        at += timedelta(milliseconds=rng.randrange(50, 9000))
        lines.append(dict({"seq": len(lines), "ts": stamp(at), "raw": raw}, **more))

    for _ in range(rng.randrange(1, 12)):
        calls += 1
        denied = rng.random() < lean
        tokens += rng.randrange(20, 900)
        if provider == "claude":
            name = rng.choice(list(POOL))
            inp = rng.choice(POOL[name])
            block = {"type": "tool_use", "id": f"toolu_{calls:02d}", "name": name}
            if inp is not None:
                block["input"] = inp
            push({"type": "assistant", "message": {"content": [block], "usage": {"output_tokens": tokens}}})
            if denied:
                push(hook(f"PreToolUse:{name}"))
            text = DENY if denied else "ok"
            if rng.random() < 0.3:
                text = [{"type": "text", "text": text}]
            push(result(f"toolu_{calls:02d}", text, denied))
        elif provider == "cursor":
            key = rng.choice(CURSOR_KEYS)
            ident = f"call_{calls}"
            args = {"path": rng.choice(["a", "b"]), "toolCallId": ident}
            push(cursor("started", ident, {key: {"args": args}}))
            done = {"error": {"message": "hydra/path: refused"}} if denied else {"success": {}}
            push(cursor("completed", ident, {key: {"args": args, "result": done}}))
        elif provider == "codex":
            output = "hydra/capability: no shell" if denied else "ok\n"
            kind = rng.choice(["command_execution", "command_execution", "file_change", "mcp_tool_call"])
            extra = {"command": rng.choice(["ls", "cargo test", "git diff"])} if kind == "command_execution" else {}
            push(codex(kind, f"item_{calls}", aggregated_output=output, **extra))
            if rng.random() < 0.4:
                push({"type": "turn.completed", "usage": {"output_tokens": tokens}})
        else:
            name = rng.choice(AGY_TOOLS)
            info = {"parameters": {"CommandLine": rng.choice(["ls", "pwd"])}}
            push(agy(calls, "ACTIVE", tool_name=name, tool_info=info))
            if denied:
                info = dict(info, error={"message": "hydra/capability: refused"})
            usage = {"usage": {"outputTokens": tokens}} if rng.random() < 0.5 else {}
            push(agy(calls, "ERROR" if denied else "DONE", tool_name=name, tool_info=info, **usage))
        if rng.random() < 0.05:
            push({"type": "system"}, hydra={"type": "seat_request", "request_id": f"r{calls}"})
    push({"type": "result", "usage": {"output_tokens": tokens}})
    push({"type": "system"}, hydra={"type": "run_end", "exit_code": 1 if bad else 0})
    return lines


def traced_records(rows: list[dict], seed: int) -> dict[str, str]:
    """A record for nine rows in ten, by file name."""
    rng = random.Random(seed)
    found = {}
    for n, row in enumerate(rows):
        lines = traced(rng, row)
        if n % 10 == 7 or not isinstance(row["run_id"], str):
            continue
        found[f"{row['run_id']}.jsonl"] = dump(lines).decode()
    return found


def squares_differ(monitor, rows: list[dict]) -> bool:
    """Whether `x ** 2` and `x * x` part ways in some scale of this fit slice. They differ
    on about one float in a thousand, so a dataset has to be checked before it can tell a
    port that multiplies from one that calls `pow`."""
    for name in monitor.SCALE_NAMES:
        values = [monitor.raw_features(r).get(name, 0.0) for r in rows]
        mean = sum(values) / len(values)
        if sum((v - mean) ** 2 for v in values) != sum((v - mean) * (v - mean) for v in values):
            return True
    return False


# ------------------------------------------------------------------ the recorder


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
        self.bundles: dict[str, dict[str, str]] = {}
        self.models: dict[str, str] = {}

    def dataset(self, name: str, rows) -> dict:
        self.datasets[name] = dump(rows)
        return {"path": "dataset.jsonl", "dataset": name}

    def bundle(self, name: str, files: dict[str, str], path="runs") -> dict:
        self.bundles[name] = files
        return {"dir": path, "bundle": name}

    def files(self, specs) -> dict[str, bytes]:
        given = {}
        for spec in specs:
            if "bundle" in spec:
                for name, text in self.bundles[spec["bundle"]].items():
                    given[f"{spec['dir']}/{name}"] = text.encode()
                continue
            if "dataset" in spec or "baseline" in spec:
                if "dataset" in spec:
                    data = self.datasets[spec["dataset"]]
                else:
                    data = (self.out / "baseline" / f"{spec['baseline']}.jsonl").read_bytes()
                if "line" in spec:
                    lines = data.splitlines(keepends=True)
                    lines[spec["line"]] = bytes.fromhex(spec["with"])
                    data = b"".join(lines)
            elif "scan" in spec:
                data = self.scan[spec["scan"]]
            elif "model" in spec:
                data = self.models[spec["model"]].encode()
            elif "text" in spec:
                data = spec["text"].encode("utf-8")
            else:
                data = bytes.fromhex(spec["hex"])
            given[spec["path"]] = data
        return given

    def case(self, name: str, argv: list[str], files=(), env=None) -> dict:
        env = {"HYDRA_ROOT": "root"} if env is None else env
        assert env.get("HYDRA_ROOT"), name
        assert all(case["name"] != name for case in self.cases), name
        given = self.files(files)
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
                    str(self.repo / "integrations" / "grader" / "monitor.py"),
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
                # replaced here, so a recording made elsewhere gives the same bytes.
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


def edited(model: dict, **over) -> bytes:
    out = dict(model)
    for key, value in over.items():
        if value is ...:
            del out[key]
        else:
            out[key] = value
    return (json.dumps(out, indent=2) + "\n").encode()


def record(out: Path, repo: Path, scan_inputs: list[tuple[str, bytes]]) -> None:
    corpus = {name: data for name, data in scan_inputs if name.endswith(".jsonl")}
    assert len(corpus) == 12, len(corpus)
    grader = repo / "integrations" / "grader"
    monitor = load("searched_monitor", grader / "monitor.py")
    tests = load("monitor_tests_for_recording", grader / "tests" / "test_monitor.py")
    made = load("record_baseline_for_monitor", HERE / "record_baseline.py")
    rec = Recorder(out, repo, corpus)
    record_file = {"path": "record.jsonl"}

    # ------------------------------------------------------------------ features
    for name in corpus:
        for at in FRACTIONS:
            rec.case(
                f"features of {name} at {at}",
                ["features", "record.jsonl", "--at", at],
                [record_file | {"scan": name}],
            )
    for name, provider, raws in odd_frames():
        rec.case(
            f"features of {name}",
            ["features", "record.jsonl", "--at", "0.5"],
            [record_file | blob(framed(raws, provider))],
        )
    # Each listed tool refused first, so the name it maps to is printed.
    for key, _ in monitor.CURSOR_TOOLS.items():
        refused = cursor("completed", "c1", {key: {"result": {"error": "hydra/capability: no"}}})
        rec.case(
            f"features of cursor: a refused {key}",
            ["features", "record.jsonl", "--at", "0.5"],
            [record_file | blob(framed([refused], "cursor"))],
        )
    for kind in list(monitor.CODEX_TOOLS) + ["mcp_tool_call"]:
        rec.case(
            f"features of codex: a refused {kind}",
            ["features", "record.jsonl", "--at", "0.5"],
            [record_file | blob(framed([codex(kind, aggregated_output="hydra/capability: no")], "codex"))],
        )
    for name, data in odd_lines():
        for at in FRACTIONS:
            rec.case(
                f"features of {name} at {at}",
                ["features", "--at", at, "./recs//record.jsonl"],
                [{"path": "recs/record.jsonl"} | blob(data)],
            )

    # The frames `test_monitor.py` builds.
    ident = tests.run_id(tests.START, 1)
    task = "Hydra launched this run\n\nprobe"
    whole = [
        dict(tests.envelope(ident, 0, tests.ts_at(0)), hydra={
            "type": "run_start", "command": "run", "policy": "read-only", "task": task,
        }),
        dict(tests.envelope(ident, 1, tests.ts_at(5)), hydra={
            "type": "seat_request", "request_id": "r", "shape": None, "seats": [],
            "task": "t", "decision": "launched", "reason": None, "runs": [],
        }),
        dict(tests.envelope(ident, 2, tests.ts_at(10)), raw={
            "type": "assistant",
            "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": "toolu_01", "name": "Bash", "input": {}},
                {"type": "tool_use", "id": "toolu_02", "name": "Write", "input": {}},
            ]},
        }),
        dict(tests.envelope(ident, 3, tests.ts_at(15)), raw=tests.hook_deny("Write")),
        dict(tests.envelope(ident, 4, tests.ts_at(20)), raw={
            "type": "user",
            "message": {"role": "user", "content": [{
                "type": "tool_result", "tool_use_id": "toolu_02", "is_error": True,
                "content": "hydra/capability: policy read-only does not allow file_write (Write).",
            }]},
            "usage": {"output_tokens": 12},
        }),
        dict(tests.envelope(ident, 5, tests.ts_at(40)), raw=tests.hook_deny("Bash")),
        dict(tests.envelope(ident, 6, tests.ts_at(50)), raw={"type": "result", "usage": {"output_tokens": 99999}}),
        dict(tests.envelope(ident, 7, tests.ts_at(50)), hydra={"type": "run_end"}),
        dict(tests.envelope(ident, 8, tests.ts_at(50)), raw={"type": "assistant", "message": {"content": []}}),
        dict(tests.envelope(ident, 9, tests.ts_at(50)), raw={"type": "result"}),
    ]
    for at in FRACTIONS:
        rec.case(
            f"features of the test's ten lines at {at}",
            ["features", "record.jsonl", f"--at={at}"],
            [record_file | blob(dump(whole))],
        )
    for fail in (True, False):
        for at in FRACTIONS:
            rec.case(
                f"features of the test's traced run, fail {fail}, at {at}",
                ["features", "record.jsonl", "--a", at],
                [record_file | blob(dump(tests.traced(ident, fail)))],
            )
    repeats = {
        "cursor ids inside the args": ([{
            "type": "tool_call", "subtype": "started", "call_id": f"c{n}",
            "tool_call": {"webSearchToolCall": {"args": {"searchTerm": "hydra", "toolCallId": f"c{n}"}}},
        } for n in (1, 2)], "cursor-agent"),
        "claude, a call seen again": ([
            tests.claude_use(1, "Read", path="a"), tests.claude_use(2, "Read", path="b"),
            tests.claude_use(3, "Read", path="a"), tests.claude_use(4, "Read", path="a"),
        ], "claude"),
        "key order of the input": ([
            tests.claude_use(1, "Grep", pattern="x", path="a"), tests.claude_use(2, "Grep", path="a", pattern="x"),
        ], "claude"),
        "a call without input": ([
            tests.claude_use(1, "Bash"), tests.claude_use(2, "Bash"), tests.claude_use(3, "Read"),
        ], "claude"),
        "cursor args": ([
            tests.cursor_start(1, "a"), tests.cursor_start(2, "a"), tests.cursor_start(3, "b"),
            tests.cursor_start(4, None),
        ], "cursor-agent"),
        "codex commands": ([
            tests.codex_run(1, "ls"), tests.codex_run(2, "ls"), tests.codex_run(3, "pwd"), tests.codex_run(4, "ls"),
        ], "codex"),
        "agy parameters": ([
            tests.agy_step(1, {"CommandLine": "ls"}), tests.agy_step(2, {"CommandLine": "ls"}),
            tests.agy_step(3, {"CommandLine": "pwd"}), tests.agy_step(4, None),
        ], "agy"),
    }
    for name, (raws, provider) in repeats.items():
        rec.case(
            f"features of the test's repeats: {name}",
            ["features", "record.jsonl", "--at", "0.5"],
            [record_file | blob(framed(raws, provider))],
        )

    good = dump(whole)
    by_id = {"path": f"root/runs/{ident}.jsonl", "hex": hexed(good)}
    rec.case("features of a run id under the root", ["features", ident, "--at", "0.25"], [by_id])
    rec.case(
        "features of a run id under a root named another way",
        ["features", "--at", "0.1", ident],
        [{"path": f"else/where/runs/{ident}.jsonl", "hex": hexed(good)}],
        env={"HYDRA_ROOT": "else/where/"},
    )
    rec.case("features of a run id nobody has", ["features", "0199d3f2", "--at", "0.5"], [by_id])
    rec.case("features of a directory", ["features", "root", "--at", "0.5"], [by_id])

    # ------------------------------------------------------------------ train
    rows = made.dataset(213, 200)
    main = {"path": "dataset.jsonl", "baseline": "main"}
    assert (out / "baseline" / "main.jsonl").read_bytes() == dump(rows)
    # The records are the first seed's whose fit slice can tell `pow` from a multiplication
    # in some scale, at some fraction. Most cannot.
    def told(files: dict[str, str]) -> list[float]:
        telling = []
        for at in monitor.FRACTIONS:
            joined = []
            for row in rows:
                text = files.get(f"{row['run_id']}.jsonl")
                if text is not None:
                    lines = [json.loads(line) for line in text.splitlines()]
                    joined.append(dict(row, _prefix=monitor.summarize(lines, at)))
            labelled = [r for r in joined if monitor.baseline.target(r, "outcome") is not None]
            fit, _ = monitor.baseline.split(labelled, monitor.baseline.day("2026-09-23"))
            if squares_differ(monitor, fit):
                telling.append(at)
        return telling

    telling = []
    for seed in range(21500, 22500):
        main_files = traced_records(rows, seed)
        telling = told(main_files)
        if telling:
            break
    assert telling, "no records whose scale tells pow from a multiplication"
    runs = rec.bundle("main", main_files)

    models = {}
    for label in ("outcome", "verdict", "task"):
        for at in FRACTIONS:
            models[label, at] = rec.trained(
                f"train {label} at {at}",
                ["train", "dataset.jsonl", "model.json", "--label", label, "--at", at, "--records", "runs"],
                [main, runs],
            )
    in_root = rec.bundle("main", main_files, "root/runs")
    rec.trained(
        "train into the grader home, from the runs under the root",
        ["train", "dataset.jsonl", "--at", "0.5"],
        [main, in_root],
        env={"HYDRA_ROOT": "root", "HYDRA_GRADER_HOME": "home"},
    )
    rec.trained(
        "train into the grader under the root, options cut short",
        ["train", "--a=0.25", "--l", "task", "dataset.jsonl", "--t", "2026-09-20"],
        [main, in_root],
        env={"HYDRA_ROOT": "root/", "HYDRA_GRADER_HOME": ""},
    )
    rec.trained(
        "train into a model path that is deep, the fraction spelled another way",
        ["train", "--records=./runs/", "--at", " 2.5e-1 ", "dataset.jsonl", "out/deep/m.json", "--label=verdict"],
        [main, runs],
    )
    rec.trained(
        "train at a fraction a billionth off",
        ["train", "dataset.jsonl", "model.json", "--at", "0.1000000005", "--records", "runs"],
        [main, runs],
    )

    # The corpus `test_monitor.py` trains on.
    with tempfile.TemporaryDirectory() as tmp:
        data, records_dir, test_rows = tests.corpus(Path(tmp))
        test_files = {p.name: p.read_text() for p in sorted(records_dir.glob("*.jsonl"))}
    test_data = rec.dataset("test", test_rows)
    test_runs = rec.bundle("test", test_files)
    day = tests.TEST.date().isoformat()
    rec.trained(
        "train on the test's corpus, outcome at 0.5",
        ["train", "dataset.jsonl", "model.json", "--at", "0.5", "--label", "outcome", "--test-from", day, "--records", "runs"],
        [test_data, test_runs],
    )
    rec.trained(
        "train on the test's corpus, verdict at 0.25",
        ["train", "dataset.jsonl", "model.json", "--at", "0.25", "--label", "verdict", "--test-from", day, "--records", "runs"],
        [test_data, test_runs],
    )
    gated = [dict(r, outcome="gated") if n < 6 else r for n, r in enumerate(test_rows)]
    rec.trained(
        "train on the test's corpus, task with gated rows",
        ["train", "dataset.jsonl", "model.json", "--at", "0.5", "--label", "task", "--test-from", day, "--records", "runs"],
        [rec.dataset("test-gated", gated), test_runs],
    )
    completions = {n: t for n, t in test_files.items() if "PreToolUse:Bash" not in t.splitlines()[1]}
    rec.case(
        "the test's corpus with every failure's record gone",
        ["train", "dataset.jsonl", "model.json", "--at", "0.5", "--test-from", day, "--records", "runs"],
        [test_data, rec.bundle("test-completions", completions)],
    )

    # Under the floor, and nothing to train on.
    base = ["train", "dataset.jsonl", "model.json", "--at", "0.5"]
    rec.case("no records directory", base + ["--records", "nowhere"], [main])
    rec.case("no runs under the root", base, [main])
    rec.case("the records directory is a file", base + ["--records", "dataset.jsonl"], [main])
    rec.case("the test slice is under the floor", base + ["--records", "runs", "--test-from", "2026-09-29T12:00"], [main, runs])
    rec.case("every row is in the fit slice", base + ["--records", "runs", "--test-from", "2027-01-01"], [main, runs])
    rec.case("no dataset", ["train", "missing.jsonl", "--at", "0.5", "--records", "runs"], [runs])
    rec.case(
        "an empty dataset",
        base + ["--records", "runs"],
        [{"path": "dataset.jsonl", "hex": ""}, runs],
    )

    # Records of the wrong shape beside good ones, and rows of the wrong shape.
    names = sorted(main_files)
    lines = [dump([r]) for r in rows]
    with_file = [n for n, r in enumerate(rows) if f"{r['run_id']}.jsonl" in main_files]

    def beside(at: int, data: bytes) -> list:
        return [main, runs, {"path": f"runs/{names[at]}", "hex": hexed(data)}]

    def patched(at: int, data: bytes) -> dict:
        return dict(main, line=at, **{"with": hexed(data)})

    start = main_files[names[5]].splitlines(keepends=True)[0].encode()
    rec.trained("train beside a garbled record", base + ["--records", "runs"], beside(5, b"not json\n"))
    rec.trained("train beside a record that is not utf-8", base + ["--records", "runs"], beside(5, start + b'{"raw": "caf\xe9"}\n'))
    rec.trained("train beside an evolve record and an empty one", base + ["--records", "runs"], [
        main, runs,
        {"path": f"runs/{names[5]}", "hex": hexed(dump([{"run_id": "a", "provider": "claude", "hydra": {"command": "evolve"}}]))},
        {"path": f"runs/{names[6]}", "hex": ""},
    ])
    rec.trained("train beside a record with a torn tail", base + ["--records", "runs"], beside(5, main_files[names[5]].encode() + b'{"raw": {"usage"'))
    rec.trained(
        "train beside a record whose times go backwards and whose count is a float",
        base + ["--records", "runs"],
        beside(5, start + dump([{"ts": "2026-09-23T00:00:00Z", "raw": {"usage": {"output_tokens": 1234.9}}}] * 3)),
    )
    rec.case("a record whose count is NaN", base + ["--records", "runs"], beside(5, start + dump([{"raw": {"usage": {"output_tokens": float("nan")}}}] * 3)))
    rec.case("a record whose count is past a float", base + ["--records", "runs"], beside(5, start + dump([{"raw": {"usage": {"output_tokens": 10**400}}}] * 3)))
    rec.case("a record whose tool is a list", base + ["--records", "runs"], beside(5, start + dump([{"raw": use(1, ["x"])}] * 3)))
    rec.case("a record whose hydra is a string", base + ["--records", "runs"], beside(5, dump([{"run_id": "a", "provider": "claude", "hydra": "run"}])))
    shapes = [
        ("a row that is a list", b"[1, 2]\n"),
        ("a row that is a string", b'"text"\n'),
        ("a row that is a number", b"7\n"),
        ("a row with no run id", dump([{k: v for k, v in rows[with_file[3]].items() if k != "run_id"}])),
        ("a garbled line", b"not json\n"),
        ("a line that is not utf-8", b'{"run_id": "caf\xe9"}\n'),
    ]
    for name, data in shapes:
        rec.case(name, base + ["--records", "runs"], [patched(with_file[3], data), runs])
    harmless = [
        ("a run id that is a number", dump([dict(rows[with_file[3]], run_id=12)])),
        ("a run id that is null", dump([dict(rows[with_file[3]], run_id=None)])),
        ("a run id that is a list", dump([dict(rows[with_file[3]], run_id=["a"])])),
        ("a run id that climbs out of the directory", dump([dict(rows[with_file[3]], run_id="../dataset")])),
        ("a run id with a nul in it", dump([dict(rows[with_file[3]], run_id="a\x00b")])),
        ("a row that already has a prefix", dump([dict(rows[with_file[3]], _prefix={"events_so_far": 10**6, "first_refused_tool": "X"})])),
        ("a row told twice", lines[with_file[3]] * 2),
    ]
    for name, data in harmless:
        rec.trained(f"train with {name}", base + ["--records", "runs"], [patched(with_file[3], data), runs])
    rec.case(
        "a run id that names a record of another run id",
        base + ["--records", "runs"],
        [patched(with_file[3], dump([dict(rows[with_file[3]], run_id=7)])), runs,
         {"path": "runs/7.jsonl"} | blob(dump([first_line("zz", "claude")]))],
    )
    rec.case(
        "an outcome that is a list",
        base + ["--records", "runs"],
        [patched(with_file[3], dump([dict(rows[with_file[3]], outcome=["failed"])])), runs],
    )
    rec.case(
        "a task that is a number, in a row with a record",
        base + ["--records", "runs"],
        [patched(with_file[3], dump([dict(rows[with_file[3]], task_text=7, outcome="failed")])), runs],
    )

    # The command line.
    for n, argv in enumerate(
        [
            [], ["x"], ["feature", "r"], ["--foo", "features", "r"], ["features"], ["features", "r"],
            ["features", "--at", "0.5"], ["train"], ["train", "d"], ["train", "--at", "0.5"], ["score"],
            ["score", "m"], ["score", "m", "r", "x"], ["features", "r", "--at"], ["features", "r", "--at", "0.3"],
            ["features", "r", "--at", "half"], ["features", "r", "--at", ""], ["features", "r", "--at", "nan"],
            ["features", "r", "--at", "inf"], ["features", "r", "--at", "-0.5"], ["features", "r", "--at", "1"],
            ["features", "r", "--at", "0"], ["features", "r", "--at", "0.5", "x"], ["features", "r", "--at=0.5", "--foo"],
            ["features", "r", "--at", "0.5_"], ["features", "r", "--at", "0._5"], ["features", "r", "--at", "_0.5"],
            ["features", "r", "--at", "0x1p-1"], ["features", "r", "--at", "0.5 x"], ["features", "r", "--at", "0,5"],
            ["features", "r", "--at", "1/2"], ["features", "r", "--at", "0.2500001"], ["features", "r", "--at", "--"],
            ["features", "--at", "0.5", "--at"], ["features", "-a", "0.5", "r"], ["features", "r", "--att", "0.5"],
            ["train", "d", "--at", "0.5", "--label", "bad"], ["train", "d", "--at", "0.5", "--test-from", "nope"],
            ["train", "d", "--at", "0.5", "--records"], ["train", "d", "--label", "task"],
            ["train", "d", "m", "x", "--at", "0.5"], ["train", "d", "--at", "0.5", "--r"],
            ["train", "d", "--at", "0.5", "--", "--records"], ["train", "--label", "task", "--records", "r"],
            ["train", "d", "--test", "2026"], ["train", "d", "--te", "2026-09-01 x", "--at", "0.5"],
            ["score", "--at", "0.5", "m", "r"], ["-x", "score", "m", "r", "q"], ["features", "-h"], ["train", "--help"],
            ["score", "-h"], ["-h"], ["features", "r", "--at", "0.5", "--help=x"], ["train", "d", "--at=bad", "-h"],
            ["train", "d", "--", "--at", "0.5"], ["features", "--", "r"], ["features", "--at", "0.5", "--"],
        ]
    ):
        rec.case(f"arguments {n:02d}: {' '.join(argv)}", argv)
    # Arguments that parse, on a record or a dataset that is not there.
    for n, argv in enumerate(
        [
            ["features", "r", "--at", "0.5"], ["features", "--at=.5", "r"], ["features", "r", "--a", "5e-1"],
            ["features", "r", "--at", "0_0.2_5"], ["features", "r", "--at", "\t0.1\n"], ["features", "r", "--at", "+0.1"],
            ["features", "r", "--at", "０.５"], ["features", "r", "--at", "0.25", "--at", "0.5"],
            ["features", "r", "--at", "00.50"], ["features", "r", "--at", "0.1", "--at=0.25"],
            ["features", "--at", "0.5", "--", "-r"], ["features", "a b", "--at", "0.5"], ["features", "-1", "--at", "0.5"],
            ["train", "d", "--at", "0.5", "--records", "-1"], ["train", "d", "--at", "0.5", "--r", "a b"],
            ["train", "--at", "0.1", "--", "d", "m"], ["train", "d", "--at", "0.25", "--label=task", "--label", "verdict"],
        ]
    ):
        rec.case(f"arguments that parse {n:02d}: {' '.join(argv)}", argv)

    # ------------------------------------------------------------------ score
    trained = {
        "outcome at 0.5": "train outcome at 0.5",
        "verdict at 0.25": "train verdict at 0.25",
        "task at 0.1": "train task at 0.1",
        "the test's": "train on the test's corpus, outcome at 0.5",
    }
    for which, source in trained.items():
        model = {"path": "model.json", "model": source}
        for name in corpus:
            rec.case(
                f"score {name} with {which} model",
                ["score", "model.json", "record.jsonl"],
                [model, record_file | {"scan": name}],
            )
    rng = random.Random(2152)
    for n, name in enumerate(rng.sample(names, 30)):
        which = list(trained)[n % 3]
        rec.case(
            f"score a traced record {n} with {which} model",
            ["score", "model.json", "record.jsonl"],
            [{"path": "model.json", "model": trained[which]}, record_file | blob(main_files[name].encode())],
        )
    test_model = {"path": "model.json", "model": trained["the test's"]}
    for fail in (True, False):
        text = dump(tests.traced(ident, fail)).replace(b'"provider": "claude"', b'"provider": "same"')
        rec.case(
            f"score the test's traced run, fail {fail}",
            ["score", "model.json", "record.jsonl"],
            [test_model, record_file | blob(text)],
        )
    for name, provider, raws in odd_frames()[::4]:
        rec.case(
            f"score a record of {name}",
            ["score", "model.json", "record.jsonl"],
            [{"path": "model.json", "model": trained["outcome at 0.5"]}, record_file | blob(framed(raws, provider))],
        )

    model = {"path": "model.json", "model": trained["outcome at 0.5"]}
    plain = record_file | blob(good)
    rec.case("score a run id under the root", ["score", "model.json", ident], [model, by_id])
    rec.case("score a run id nobody has", ["score", "model.json", "0199d3f2"], [model, by_id])
    rec.case("score a record that is a directory", ["score", "model.json", "root"], [model, by_id])
    for name, data in odd_lines()[-15:]:
        rec.case(
            f"score {name}",
            ["score", "./models//model.json", "./recs//bad.jsonl"],
            [{"path": "models/model.json", "model": trained["outcome at 0.5"]}, {"path": "recs/bad.jsonl"} | blob(data)],
        )

    # A model file somebody edited.
    fitted = models["outcome", "0.5"]
    weights = fitted["weights"]
    where = fitted["vocab"].index("events_so_far")
    for name, data in [
        ("no model file", None),
        ("a model that is not json", b"not json"),
        ("a model that is a list", b"[1, 2]\n"),
        ("a model that is null", b"null\n"),
        ("a model that is text", b'"at"\n'),
        ("a model with no fraction", edited(fitted, at=...)),
        ("a model whose fraction is null", edited(fitted, at=None)),
        ("a model whose fraction is text", edited(fitted, at="0.5")),
        ("a model whose fraction is true", edited(fitted, at=True)),
        ("a model whose fraction is false", edited(fitted, at=False)),
        ("a model whose fraction is one", edited(fitted, at=1)),
        ("a model whose fraction is zero", edited(fitted, at=0)),
        ("a model whose fraction is 0.3", edited(fitted, at=0.3)),
        ("a model whose fraction is a list", edited(fitted, at=[0.5])),
        ("a model whose fraction is NaN", edited(fitted, at=float("nan"))),
        ("a model whose fraction is infinite", edited(fitted, at=float("inf"))),
        ("a model whose fraction is a whole number past a float", edited(fitted, at=10**400)),
        ("a model whose fraction is a huge whole number", edited(fitted, at=10**40)),
        ("a model whose fraction is a billionth off a quarter", edited(fitted, at=0.2500000004)),
        ("a model whose fraction is a tenth", edited(fitted, at=0.1)),
        ("a model whose fraction is a tenth, spelled long", edited(fitted, at=0.1).replace(b'"at": 0.1', b'"at": 1.00000000001e-1')),
        ("a model with no version", edited(fitted, version=...)),
        ("a model with no label", edited(fitted, label=...)),
        ("a model with no positive", edited(fitted, positive=...)),
        ("a model with no scale", edited(fitted, scale=...)),
        ("a model with no weights", edited(fitted, weights=...)),
        ("a model with no bias", edited(fitted, bias=...)),
        ("a model with no vocab", edited(fitted, vocab=...)),
        ("a model with an empty scale", edited(fitted, scale={})),
        ("a model with a deviation of zero", edited(fitted, scale={"events_so_far": [5.0, 0.0]})),
        ("a model with a scale of three", edited(fitted, scale={"events_so_far": [5.0, 1.0, 2.0]})),
        ("a model that scales a one-hot name and the seat", edited(fitted, scale={"seat_requested": [0.5, 0.25], "first_refused=Write": [1, 4], "provider=claude": [2, 2]})),
        ("a model with whole-number weights", edited(fitted, weights=[round(w) for w in weights], bias=1)),
        ("a model with fewer weights", edited(fitted, weights=weights[:3])),
        ("a model with a text weight", edited(fitted, weights=["1.5"] + weights[1:])),
        ("a model with odd names in its vocabulary", edited(fitted, vocab=[7, None, True, 1.5] + fitted["vocab"][4:])),
        ("a model with a list in its vocabulary", edited(fitted, vocab=[["x"]] + fitted["vocab"][1:])),
        ("a model with the repeat counts in its vocabulary", edited(fitted, vocab=["repeated_calls_so_far", "longest_repeat_run", "first_refused_tool", "_prefix"] + fitted["vocab"][4:])),
        ("a model with a huge weight on the events", edited(fitted, weights=weights[:where] + [1e6] + weights[where + 1:])),
        ("a model with a label and a version of any shape", edited(fitted, label=["x"], positive=None, version={"v": 1.5})),
    ]:
        files = [plain] if data is None else [plain, {"path": "model.json"} | blob(data)]
        rec.case(f"score with {name}", ["score", "model.json", "record.jsonl"], files)

    directory = out / "monitor"
    directory.mkdir(exist_ok=True)
    for stale in directory.iterdir():
        stale.unlink()
    for name, data in rec.datasets.items():
        (directory / f"{name}.jsonl").write_bytes(data)
    for name, files in rec.bundles.items():
        (directory / f"{name}.json").write_text(json.dumps(files, indent=0) + "\n", encoding="utf-8")
    (out / "monitor.json").write_text(json.dumps(rec.cases, indent=1) + "\n", encoding="utf-8")
    print(f"monitor: {len(rec.cases)} cases, seed {seed} tells pow from a multiplication at {telling}", file=sys.stderr)
