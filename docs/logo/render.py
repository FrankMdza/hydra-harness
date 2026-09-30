#!/usr/bin/env python3
"""Print a logo candidate from candidates.md by name, or all of them with no argument."""
import re
import sys
from pathlib import Path

CANDIDATES = Path(__file__).with_name("candidates.md")


def load(path=CANDIDATES):
    """Return {name: (art_lines, note)} in file order."""
    pattern = r"^## (\S+)\n\n```\n(.*?)\n```\n\n(.*?)$"
    found = {}
    for m in re.finditer(pattern, path.read_text(), re.M | re.S):
        found[m.group(1)] = (m.group(2).split("\n"), m.group(3).strip())
    return found


def main(argv):
    cands = load()
    for name in argv or list(cands):
        if name not in cands:
            print(f"no candidate {name!r}; have: {', '.join(cands)}", file=sys.stderr)
            return 1
        art, note = cands[name]
        print(name)
        print("\n".join(art))
        print(note + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
