#!/usr/bin/env python3
"""Map 111: which small local model emits tool calls well enough to drive Hydra's own harness.

Map 88 reopens the harness when a local model emits well-formed tool calls on 7 of 10 tasks.
Ten tasks, the four tools under `claude`'s names, sent one at a time to Ollama's
OpenAI-compatible `/v1/chat/completions` with a `tools` array. A task passes only when the first
answer is a tool call whose arguments parse as JSON, name the expected tool, and carry the
argument the task needs. Usage: python3 111-tool-call-bakeoff.py MODEL [MODEL...]
"""
import json
import sys
import time
import urllib.request

URL = "http://127.0.0.1:11434/v1/chat/completions"

TOOLS = [
    {"type": "function", "function": {"name": "Read", "description": "Read a file.",
     "parameters": {"type": "object", "properties": {"file_path": {"type": "string"}},
                    "required": ["file_path"]}}},
    {"type": "function", "function": {"name": "Write", "description": "Write a whole file.",
     "parameters": {"type": "object", "properties": {"file_path": {"type": "string"},
                                                     "content": {"type": "string"}},
                    "required": ["file_path", "content"]}}},
    {"type": "function", "function": {"name": "Edit", "description": "Replace one exact string in a file.",
     "parameters": {"type": "object", "properties": {"file_path": {"type": "string"},
                                                     "old_string": {"type": "string"},
                                                     "new_string": {"type": "string"}},
                    "required": ["file_path", "old_string", "new_string"]}}},
    {"type": "function", "function": {"name": "Bash", "description": "Run a shell command.",
     "parameters": {"type": "object", "properties": {"command": {"type": "string"}},
                    "required": ["command"]}}},
]

SYSTEM = ("You are a coding agent in a git repository. Act only through the tools. "
          "Call exactly one tool for the user's request.")

# (task, expected tool, a check on the parsed arguments)
TASKS = [
    ("Show me what is in src/run.rs.", "Read", lambda a: a.get("file_path", "").endswith("src/run.rs")),
    ("Run the test suite with cargo.", "Bash", lambda a: "cargo test" in a.get("command", "")),
    ("Create notes.md containing the line 'hello'.", "Write",
     lambda a: a.get("file_path", "").endswith("notes.md") and "hello" in a.get("content", "")),
    ("In README.md, change the word 'macOS' to 'macOS and Linux'.", "Edit",
     lambda a: a.get("file_path", "").endswith("README.md") and a.get("old_string") == "macOS"
     and "Linux" in a.get("new_string", "")),
    ("How many lines does Cargo.toml have? Use a shell command.", "Bash",
     lambda a: "wc" in a.get("command", "") and "Cargo.toml" in a.get("command", "")),
    ("Open the file docs/harness-spec.md.", "Read",
     lambda a: a.get("file_path", "").endswith("docs/harness-spec.md")),
    ("Show the last three commits.", "Bash",
     lambda a: "git log" in a.get("command", "")),
    ("Write a file hi.txt that says 'hi there'.", "Write",
     lambda a: a.get("file_path", "").endswith("hi.txt") and "hi there" in a.get("content", "")),
    ("In src/lib.rs replace 'pub mod run;' with 'pub mod run; pub mod bench;'.", "Edit",
     lambda a: a.get("file_path", "").endswith("src/lib.rs") and a.get("old_string") == "pub mod run;"),
    ("List the files in the tests directory.", "Bash",
     lambda a: "ls" in a.get("command", "") and "tests" in a.get("command", "")),
]


def ask(model, task):
    body = json.dumps({"model": model, "stream": False, "temperature": 0,
                       "messages": [{"role": "system", "content": SYSTEM},
                                    {"role": "user", "content": task}],
                       "tools": TOOLS}).encode()
    req = urllib.request.Request(URL, data=body, headers={"Content-Type": "application/json"})
    started = time.time()
    with urllib.request.urlopen(req, timeout=300) as resp:
        answer = json.load(resp)
    return answer, time.time() - started


def grade(model):
    passed, rows = 0, []
    for task, want, check in TASKS:
        try:
            answer, secs = ask(model, task)
        except Exception as err:  # an HTTP 400 "does not support tools" lands here
            rows.append({"task": task, "ok": False, "why": f"error: {err}"[:160]})
            continue
        msg = answer["choices"][0]["message"]
        calls = msg.get("tool_calls") or []
        if not calls:
            rows.append({"task": task, "ok": False, "why": "no tool call", "secs": round(secs, 1)})
            continue
        fn = calls[0]["function"]
        try:
            args = fn["arguments"] if isinstance(fn["arguments"], dict) else json.loads(fn["arguments"])
        except Exception:
            rows.append({"task": task, "ok": False, "why": "arguments are not JSON", "secs": round(secs, 1)})
            continue
        ok = fn["name"] == want and bool(check(args))
        passed += ok
        rows.append({"task": task, "ok": ok, "tool": fn["name"], "args": args, "secs": round(secs, 1)})
    return passed, rows


if __name__ == "__main__":
    for model in sys.argv[1:]:
        passed, rows = grade(model)
        secs = [r["secs"] for r in rows if "secs" in r]
        print(json.dumps({"model": model, "passed": passed, "of": len(TASKS),
                          "median_secs": sorted(secs)[len(secs) // 2] if secs else None,
                          "rows": rows}))
        sys.stdout.flush()
