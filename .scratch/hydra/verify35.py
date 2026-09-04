#!/usr/bin/env python3
"""Ticket 35: check whether a human's tool rejection produced the pair the contract needs.

Scans ~/.claude/projects for recent transcripts, finds denials where a human answered the
permission prompt, and checks the four things ticket 34's harvest rule depends on.

    python3 .scratch/hydra/verify35.py [minutes]     default: 120
"""
import json, pathlib, sys, time

ROOT = pathlib.Path.home() / ".claude" / "projects"
WINDOW = int(sys.argv[1]) if len(sys.argv) > 1 else 120


def blocks(rec, kind):
    msg = rec.get("message") or {}
    content = msg.get("content")
    if not isinstance(content, list):
        return []
    return [b for b in content if isinstance(b, dict) and b.get("type") == kind]


def load(path):
    out = []
    with open(path, errors="replace") as fh:
        for i, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            try:
                out.append((i, json.loads(line)))
            except Exception:
                pass
    return out


def walk_forward(children, rec, maxhops=25):
    cur = rec
    for _ in range(maxhops):
        kids = children.get(cur.get("uuid"), [])
        if not kids:
            return None
        _, nxt = kids[0]
        if nxt.get("type") == "assistant":
            if blocks(nxt, "tool_use"):
                return ("tool_use", blocks(nxt, "tool_use")[0])
            if blocks(nxt, "text"):
                return ("text", blocks(nxt, "text")[0])
        cur = nxt
    return None


def main():
    cutoff = time.time() - WINDOW * 60
    files = [p for p in ROOT.rglob("*.jsonl") if p.stat().st_mtime >= cutoff]
    if not files:
        sys.exit(f"no transcript touched in the last {WINDOW} minutes under {ROOT}")

    found = 0
    for path in sorted(files, key=lambda p: p.stat().st_mtime):
        recs = load(path)
        by_uuid = {r.get("uuid"): (i, r) for i, r in recs if r.get("uuid")}
        children = {}
        for i, r in recs:
            children.setdefault(r.get("parentUuid"), []).append((i, r))

        for i, r in recs:
            if r.get("toolDenialKind") != "user-rejected":
                continue
            if r.get("entrypoint") != "cli":
                continue
            found += 1
            results = blocks(r, "tool_result")
            tuid = results[0].get("tool_use_id") if results else None
            src = r.get("sourceToolAssistantUUID")
            rejected = None
            if src in by_uuid:
                for b in blocks(by_uuid[src][1], "tool_use"):
                    if b.get("id") == tuid:
                        rejected = b
            feedback = r.get("userFeedback")
            nxt = walk_forward(children, r)

            def ok(cond):
                return "PASS" if cond else "FAIL"

            print("=" * 78)
            print(f"{path.parent.name}/{path.name}  line {i}")
            print("  [PASS] toolDenialKind is user-rejected")
            print("  [PASS] entrypoint is cli")
            print(f"  [{ok(rejected)}] rejected call recovered"
                  + (f": {rejected['name']} {json.dumps(rejected['input'])[:140]}" if rejected else ""))
            print(f"  [{ok(feedback)}] userFeedback present"
                  + (f": {json.dumps(feedback)[:200]}" if feedback else " (empty)"))
            if nxt and nxt[0] == "tool_use":
                same = bool(rejected) and nxt[1].get("name") == rejected.get("name")
                print(f"  [PASS] chain reaches a replacement tool_use: "
                      f"{nxt[1]['name']} {json.dumps(nxt[1]['input'])[:140]}")
                print(f"  [{ok(same)}] replacement uses the same tool"
                      + ("" if same else f" (rejected {rejected['name'] if rejected else '?'}, "
                                         f"chose {nxt[1]['name']})"))
            elif nxt:
                print("  [FAIL] chain reached text, not a replacement call: "
                      f"{nxt[1].get('text','')[:160]!r}")
            else:
                print("  [FAIL] chain reached nothing")

    print("=" * 78)
    if not found:
        print(f"No human rejection found in {len(files)} transcript(s) from the last {WINDOW} min.")
        print("A denial needs toolDenialKind == 'user-rejected' AND entrypoint == 'cli'.")
        print("A rejection from a script writes entrypoint 'sdk-cli' and does not count.")
    else:
        print(f"{found} human rejection(s) across {len(files)} recent transcript(s).")


if __name__ == "__main__":
    main()
