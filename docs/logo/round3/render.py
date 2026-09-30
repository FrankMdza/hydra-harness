#!/usr/bin/env python3
"""Print round-three logo grids as half blocks in 24-bit colour, or as plain shape with --plain.

    python3 docs/logo/round3/render.py [--plain] [grid.txt ...]

With no file it prints every `design-*.txt` under this directory. A grid file is pixel rows, one
character per pixel, `.` transparent, then a blank line, then a legend of `X r g b` lines. Lines
starting with `#` are comments. Two pixel rows make one terminal row, the way
pokemon-colorscripts does it: both pixels opaque is a `▀` with the top as foreground and the
bottom as background, a lone top pixel is a `▀`, a lone bottom pixel is a `▄`.
"""
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(path):
    """Return (rows, legend). Rows are padded to one width and an even count with `.`."""
    text = Path(path).read_text()
    lines = [ln for ln in text.split("\n") if not ln.startswith("#")]
    rows, legend = [], {}
    in_grid = True
    for line in lines:
        if in_grid:
            if line.strip() == "":
                if rows:
                    in_grid = False
                continue
            rows.append(line.rstrip())
        else:
            parts = line.split()
            if len(parts) == 4 and len(parts[0]) == 1:
                legend[parts[0]] = tuple(int(v) for v in parts[1:])
    width = max(len(r) for r in rows)
    rows = [r.ljust(width, ".") for r in rows]
    if len(rows) % 2:
        rows.append("." * width)
    missing = {c for r in rows for c in r} - set(legend) - {"."}
    if missing:
        raise ValueError(f"{path}: no legend entry for {''.join(sorted(missing))}")
    return rows, legend


def plain(rows):
    out = []
    for top, bot in zip(rows[0::2], rows[1::2]):
        line = ""
        for t, b in zip(top, bot):
            line += {(False, False): " ", (True, False): "▀", (False, True): "▄"}.get(
                (t != ".", b != "."), "█"
            )
        out.append(line.rstrip())
    return out


def colour(rows, legend):
    def fg(c):
        return "\x1b[38;2;%d;%d;%dm" % legend[c]

    def bg(c):
        return "\x1b[48;2;%d;%d;%dm" % legend[c]

    out = []
    for top, bot in zip(rows[0::2], rows[1::2]):
        line = ""
        for t, b in zip(top, bot):
            if t == "." and b == ".":
                line += " "
            elif b == ".":
                line += fg(t) + "▀\x1b[0m"
            elif t == ".":
                line += fg(b) + "▄\x1b[0m"
            else:
                line += fg(t) + bg(b) + "▀\x1b[0m"
        out.append(line.rstrip())
    return out


def main(argv):
    use_plain = "--plain" in argv
    paths = [a for a in argv if a != "--plain"] or sorted(HERE.glob("*/design-*.txt"))
    for path in paths:
        rows, legend = load(path)
        full = Path(path).resolve()
        print(full.relative_to(HERE) if full.is_relative_to(HERE) else path)
        print("\n".join(plain(rows) if use_plain else colour(rows, legend)))
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
