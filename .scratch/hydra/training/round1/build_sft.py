#!/usr/bin/env python3
"""Round one: turn `hydra export` into chat trajectories a small model can be fine-tuned on.

Keeps the runs whose events carry the task as a user message. Every provider's tools map onto
five names under `claude`'s spelling (Read, Write, Edit, Bash, Grep); a trajectory stops at the
first call that maps to none of them, since what follows it depends on a tool the model will not
have. Worktree paths become relative, tool output is cut to OUT_CHARS, thinking is dropped.
Splits by run, 15 percent held out, seed fixed.

Usage: python3 build_sft.py EXPORT.jsonl OUTDIR
"""
import json
import random
import re
import sys

OUT_CHARS = 600
PROMPT_CHARS = 2000  # both cut so a trajectory's calls fit a T4's 2048-token training window
WORKTREE = re.compile(r"/Users/USER/hydra/(?:skill-)?worktrees/[0-9a-f-]{36}/?")

SYSTEM = ("You are a coding agent in a git repository. Act only through the tools, one call at "
          "a time, and answer in plain text when the task is done.")

TOOLS = [
    {"type": "function", "function": {"name": "Read", "description": "Read a file.",
     "parameters": {"type": "object", "properties": {"file_path": {"type": "string"},
                                                     "offset": {"type": "integer"},
                                                     "limit": {"type": "integer"}},
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
    {"type": "function", "function": {"name": "Grep", "description": "Search file contents with a regex.",
     "parameters": {"type": "object", "properties": {"pattern": {"type": "string"},
                                                     "path": {"type": "string"},
                                                     "glob": {"type": "string"}},
                    "required": ["pattern"]}}},
]


def rel(value):
    if isinstance(value, str):
        return WORKTREE.sub("", value).replace("/Users/USER", "~") or "."
    return value


def keep(args, *names):
    return {k: rel(args[k]) for k in names if args.get(k) not in (None, "")}


def mapped(name, a):
    """The call under one of the five names, or None when it has no equivalent."""
    if not isinstance(a, dict):
        return None
    if name in ("Read", "Write"):
        out = keep(a, "file_path", "offset", "limit", "content")
    elif name == "Edit":
        out = keep(a, "file_path", "old_string", "new_string")
    elif name in ("Bash", "Shell", "bash"):
        name, out = "Bash", keep(a, "command")
    elif name == "run_command":
        name, out = "Bash", {"command": rel(a.get("CommandLine", ""))}
    elif name == "view_file":
        name, out = "Read", {"file_path": rel(a.get("AbsolutePath", ""))}
    elif name == "list_dir":
        name, out = "Bash", {"command": "ls " + rel(a.get("DirectoryPath", "."))}
    elif name == "find_by_name":
        name, out = "Bash", {"command": f"find {rel(a.get('SearchDirectory', '.'))} -name '{a.get('Pattern', '')}'"}
    elif name == "Grep":
        out = {"pattern": a.get("pattern", "")}
        out.update({"path": rel(p) for p in [a.get("path") or a.get("file_path")] if p})
        out.update(keep(a, "glob"))
    elif name == "grep_search":
        name, out = "Grep", {"pattern": a.get("Query", ""), "path": rel(a.get("SearchPath", "."))}
    else:
        return None
    required = {"Read": ["file_path"], "Write": ["file_path", "content"],
                "Edit": ["file_path", "old_string", "new_string"], "Bash": ["command"],
                "Grep": ["pattern"]}[name]
    if any(not out.get(k) for k in required):
        return None
    return name, out


def text_of(output):
    if isinstance(output, str):
        s = output
    else:
        s = json.dumps(output, ensure_ascii=False)
    s = rel(s)
    return s if len(s) <= OUT_CHARS else s[:OUT_CHARS] + "\n[... cut]"


def trajectory(events):
    users = [e for e in events if e["type"] == "message" and e.get("role") == "user"]
    if not users:
        return None
    start = events.index(users[0])
    messages = [{"role": "system", "content": SYSTEM},
                {"role": "user", "content": rel(users[0]["text"])[:PROMPT_CHARS]}]
    results = {e.get("call_id"): e for e in events if e["type"] == "tool_result"}
    n = 0
    for e in events[start + 1:]:
        if e["type"] == "message" and e.get("role") == "assistant" and e.get("text", "").strip():
            messages.append({"role": "assistant", "content": rel(e["text"])})
        elif e["type"] == "message" and e.get("role") == "user":
            break
        elif e["type"] == "tool_call":
            m = mapped(e["name"], e.get("arguments"))
            if m is None:
                break
            n += 1
            cid = f"c{n:08d}"  # Mistral wants nine alphanumerics
            call = {"id": cid, "type": "function", "function": {"name": m[0], "arguments": m[1]}}
            if messages[-1]["role"] == "assistant" and "tool_calls" not in messages[-1]:
                messages[-1]["tool_calls"] = [call]  # text then a call is one turn, roles alternate
            else:
                messages.append({"role": "assistant", "tool_calls": [call]})
            r = results.get(e.get("call_id"))
            messages.append({"role": "tool", "tool_call_id": cid,
                             "content": text_of(r.get("output", "")) if r else "(no result recorded)"})
    if n == 0:
        return None
    while messages[-1]["role"] == "tool":  # end on the model's own turn
        messages.pop(); messages.pop()
    return messages if any("tool_calls" in m for m in messages) else None


def main(export, outdir):
    lines = open(export).read().splitlines()[1:]
    rows = [t for t in (trajectory(json.loads(l)["events"]) for l in lines) if t]
    random.Random(111).shuffle(rows)
    held = max(1, len(rows) * 15 // 100)
    for name, part in (("eval", rows[:held]), ("train", rows[held:])):
        with open(f"{outdir}/{name}.jsonl", "w") as f:
            for m in part:
                f.write(json.dumps({"messages": m}, ensure_ascii=False) + "\n")
    json.dump(TOOLS, open(f"{outdir}/tools.json", "w"))
    calls = sum(sum("tool_calls" in m for m in r) for r in rows)
    print(json.dumps({"runs": len(rows), "train": len(rows) - held, "eval": held, "calls": calls}))


if __name__ == "__main__":
    main(*sys.argv[1:3])
