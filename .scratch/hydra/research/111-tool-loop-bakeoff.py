#!/usr/bin/env python3
"""Map 111, second pass: the same small models in a real tool loop, scored on the end state.

The first pass (111-tool-call-bakeoff.py) scored only the first call, so a model that reads a
file before editing it failed. Here the model gets each tool's result back and keeps going, up
to MAX_TURNS, in a scratch git repository. A task passes when the files on disk, or the final
answer, are right. Read, Write and Edit act on the scratch directory only. Bash runs only
`ls`, `wc`, `cat`, `head` and `git log`, inside it; anything else is answered "not available".

Usage: python3 111-tool-loop-bakeoff.py MODEL [MODEL...]
"""
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request

URL = "http://127.0.0.1:11434/v1/chat/completions"
MAX_TURNS = 6
ALLOWED = {"ls", "wc", "cat", "head", "git"}

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
    {"type": "function", "function": {"name": "Bash", "description": "Run a shell command in the repository.",
     "parameters": {"type": "object", "properties": {"command": {"type": "string"}},
                    "required": ["command"]}}},
]

SYSTEM = ("You are a coding agent in a git repository. Use the tools to do the task, one call at "
          "a time. When the task is done, answer in plain text and call no tool.")

FILES = {
    "README.md": "# Demo\n\nRuns on macOS.\n",
    "Cargo.toml": "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    "src/lib.rs": "pub mod run;\n\npub fn hello() -> &'static str {\n    \"hi\"\n}\n",
    "src/run.rs": "pub fn go() {}\n",
    "tests/a.txt": "a\n",
    "tests/b.txt": "b\n",
}


def read(root, rel):
    try:
        return open(os.path.join(root, rel)).read()
    except OSError:
        return None


# (task, check(root, final_text) -> bool)
TASKS = [
    ("In README.md, change the word 'macOS' to 'macOS and Linux'.",
     lambda r, t: read(r, "README.md") == "# Demo\n\nRuns on macOS and Linux.\n"),
    ("In src/lib.rs replace 'pub mod run;' with 'pub mod run;\npub mod bench;'.",
     lambda r, t: (read(r, "src/lib.rs") or "").startswith("pub mod run;\npub mod bench;\n")),
    ("Create notes.md containing the line 'hello'.",
     lambda r, t: "hello" in (read(r, "notes.md") or "")),
    ("How many lines does Cargo.toml have? Answer with the number.",
     lambda r, t: "6" in t),
    ("List the files in the tests directory and tell me their names.",
     lambda r, t: "a.txt" in t and "b.txt" in t),
    ("What does the function hello in src/lib.rs return? Answer with the string.",
     lambda r, t: "hi" in t),
    ("Add a line 'Licensed MIT.' at the end of README.md, keeping what is there.",
     lambda r, t: (read(r, "README.md") or "").startswith("# Demo\n\nRuns on macOS.") and "Licensed MIT." in (read(r, "README.md") or "")),
    ("Show the last commit message.",
     lambda r, t: "initial" in t.lower()),
    ("Write a file hi.txt that says 'hi there'.",
     lambda r, t: "hi there" in (read(r, "hi.txt") or "")),
    ("Rename the function go in src/run.rs to start.",
     lambda r, t: "pub fn start()" in (read(r, "src/run.rs") or "") and "go()" not in (read(r, "src/run.rs") or "")),
]


def scratch():
    root = os.path.realpath(tempfile.mkdtemp(prefix="bakeoff-"))  # /var is /private/var on macOS
    for rel, text in FILES.items():
        os.makedirs(os.path.dirname(os.path.join(root, rel)) or root, exist_ok=True)
        with open(os.path.join(root, rel), "w") as f:
            f.write(text)
    env = {"PATH": "/usr/bin:/bin", "HOME": root, "GIT_CONFIG_GLOBAL": "/dev/null"}
    for cmd in (["git", "init", "-q"], ["git", "add", "."],
                ["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "initial commit"]):
        subprocess.run(cmd, cwd=root, env=env, check=True)
    return root


def inside(root, rel):
    path = os.path.realpath(os.path.join(root, rel))
    return path if path == root or path.startswith(root + os.sep) else None


def run_tool(root, name, args):
    try:
        if name == "Read":
            path = inside(root, args["file_path"])
            if not path:
                return "error: outside the repository"
            if os.path.isdir(path):
                return "error: that is a directory"
            return open(path).read()
        if name == "Write":
            path = inside(root, args["file_path"])
            if not path:
                return "error: outside the repository"
            os.makedirs(os.path.dirname(path), exist_ok=True)
            open(path, "w").write(args["content"])
            return "written"
        if name == "Edit":
            path = inside(root, args["file_path"])
            if not path or not os.path.isfile(path):
                return "error: no such file"
            text = open(path).read()
            if text.count(args["old_string"]) != 1:
                return f"error: old_string found {text.count(args['old_string'])} times"
            open(path, "w").write(text.replace(args["old_string"], args["new_string"]))
            return "edited"
        if name == "Bash":
            words = shlex.split(args["command"])
            if not words or words[0] not in ALLOWED or (words[0] == "git" and words[1:2] != ["log"]):
                return "error: only ls, wc, cat, head and git log are available"
            if any(w.startswith("/") or ".." in w for w in words[1:]):
                return "error: paths must be inside the repository"
            out = subprocess.run(words, cwd=root, capture_output=True, text=True, timeout=10,
                                 env={"PATH": "/usr/bin:/bin", "HOME": root, "GIT_CONFIG_GLOBAL": "/dev/null"})
            return (out.stdout + out.stderr)[:2000]
    except OSError as err:
        return f"error: {err.strerror}"
    except (KeyError, TypeError, ValueError) as err:
        return f"error: bad arguments: {err}"
    return f"error: no tool named {name}"


def chat(model, messages):
    body = json.dumps({"model": model, "stream": False, "temperature": 0,
                       "messages": messages, "tools": TOOLS}).encode()
    req = urllib.request.Request(URL, data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=300) as resp:
        return json.load(resp)["choices"][0]["message"]


def attempt(model, task, check):
    root = scratch()
    messages = [{"role": "system", "content": SYSTEM}, {"role": "user", "content": task}]
    calls, final, why = [], "", ""
    try:
        for _ in range(MAX_TURNS):
            msg = chat(model, messages)
            tool_calls = msg.get("tool_calls") or []
            messages.append({"role": "assistant", "content": msg.get("content") or "",
                             "tool_calls": tool_calls} if tool_calls else
                            {"role": "assistant", "content": msg.get("content") or ""})
            if not tool_calls:
                final = msg.get("content") or ""
                break
            for call in tool_calls:
                fn = call["function"]
                try:
                    args = fn["arguments"] if isinstance(fn["arguments"], dict) else json.loads(fn["arguments"])
                except (json.JSONDecodeError, TypeError):
                    args, result = None, "error: arguments are not JSON"
                else:
                    result = run_tool(root, fn["name"], args)
                calls.append(fn["name"])
                messages.append({"role": "tool", "tool_call_id": call.get("id", ""), "content": result})
        else:
            why = "ran out of turns"
        ok = bool(check(root, final))
    except Exception as err:  # an HTTP error from a model without tools lands here
        ok, why = False, f"error: {err}"[:160]
    finally:
        shutil.rmtree(root, ignore_errors=True)
    return {"task": task[:60], "ok": ok, "calls": calls, "final": final[:120], "why": why}


if __name__ == "__main__":
    for model in sys.argv[1:]:
        started = time.time()
        rows = [attempt(model, task, check) for task, check in TASKS]
        print(json.dumps({"model": model, "passed": sum(r["ok"] for r in rows), "of": len(rows),
                          "secs": round(time.time() - started, 1), "rows": rows}))
        sys.stdout.flush()
