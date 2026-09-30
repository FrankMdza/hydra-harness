#!/usr/bin/env python3
"""Render a text pixel grid followed by RGB legend lines, e.g. G 32 153 103.

Dots are transparent; blank lines and # comments are ignored.
"""

import argparse
from pathlib import Path


def load_grid(path):
    rows = []
    palette = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) == 4 and len(parts[0]) == 1:
            key = parts[0]
            rgb = tuple(int(value) for value in parts[1:])
            if key == "." or key in palette or any(not 0 <= v <= 255 for v in rgb):
                raise ValueError(f"line {number}: invalid or duplicate palette entry")
            palette[key] = rgb
        elif len(parts) == 1 and not palette:
            rows.append(line)
        else:
            raise ValueError(f"line {number}: expected grid row or RGB legend")
    if not rows or any(len(row) != len(rows[0]) for row in rows):
        raise ValueError("grid must be nonempty and rectangular")
    unknown = set("".join(rows)) - set(palette) - {"."}
    if unknown:
        raise ValueError(f"undefined pixels: {sorted(unknown)}")
    return rows, palette


def render(rows, palette, plain=False):
    lines = []
    for y in range(0, len(rows), 2):
        top = rows[y]
        bottom = rows[y + 1] if y + 1 < len(rows) else "." * len(top)
        cells = []
        for a, b in zip(top, bottom):
            if plain:
                cells.append(" " if a == b == "." else
                             "▄" if a == "." else "▀" if b == "." else "█")
                continue
            if a == b == ".":
                cells.append("\x1b[0m ")
                continue
            foreground = palette[b if a == "." else a]
            cell = "\x1b[0m\x1b[38;2;" + ";".join(map(str, foreground)) + "m"
            if a != "." and b != ".":
                cell += "\x1b[48;2;" + ";".join(map(str, palette[b])) + "m"
            cells.append(cell + ("▄" if a == "." else "▀"))
        line = "".join(cells)
        lines.append(line.rstrip() if plain else line + "\x1b[0m")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("grid", type=Path)
    parser.add_argument("--plain", action="store_true", help="silhouette without ANSI escapes")
    args = parser.parse_args()
    try:
        rows, palette = load_grid(args.grid)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(render(rows, palette, args.plain))


if __name__ == "__main__":
    main()
