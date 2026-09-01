#!/usr/bin/env python3
"""Print the wayfinder frontier: which tickets are takeable now, and what blocks the rest."""

import pathlib
import re
import sys

ISSUES = pathlib.Path(__file__).parent / "issues"


def load():
    tickets = {}
    for f in sorted(ISSUES.glob("[0-9][0-9]-*.md")):
        text = f.read_text()
        num = f.name[:2]
        status = re.search(r"^Status: (\w+)", text, re.M)
        ttype = re.search(r"^Type: (\w+)", text, re.M)
        blocked = re.search(r"^Blocked by: (.+)$", text, re.M)
        tickets[num] = {
            "file": f.name,
            "title": text.splitlines()[0].lstrip("# ").strip(),
            "status": status.group(1) if status else "open",
            "type": ttype.group(1) if ttype else "?",
            "blocked_by": [b.strip() for b in blocked.group(1).split(",")] if blocked else [],
        }
    return tickets


def main():
    t = load()
    if not t:
        sys.exit(f"no tickets found in {ISSUES}")

    def unblocked(num):
        return all(t.get(b, {}).get("status") == "resolved" for b in t[num]["blocked_by"])

    frontier = [n for n in t if t[n]["status"] == "open" and unblocked(n)]
    blocked = [n for n in t if t[n]["status"] == "open" and not unblocked(n)]
    claimed = [n for n in t if t[n]["status"] == "claimed"]
    done = [n for n in t if t[n]["status"] == "resolved"]

    print(f"FRONTIER ({len(frontier)} takeable now)")
    for n in frontier:
        print(f"  {n}  {t[n]['type']:<9} {t[n]['title']}")

    if claimed:
        print(f"\nCLAIMED ({len(claimed)} in flight, do not take)")
        for n in claimed:
            print(f"  {n}  {t[n]['type']:<9} {t[n]['title']}")

    print(f"\nBLOCKED ({len(blocked)})")
    for n in blocked:
        waiting = [b for b in t[n]["blocked_by"] if t.get(b, {}).get("status") != "resolved"]
        print(f"  {n}  {t[n]['title']}\n      waiting on {', '.join(waiting)}")

    print(f"\n{len(done)} of {len(t)} resolved")


if __name__ == "__main__":
    main()
