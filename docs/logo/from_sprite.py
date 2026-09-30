#!/usr/bin/env python3
"""Print the six round-two logo candidates as candidates.md sections.

Four come from the HydraPet sprite by downscaling (`shrink`): each output pixel takes the
commonest palette letter under it, so no colour is ever blended and the palette stays the
sprite's. The other two are small grids typed by hand, marked below. `~` in a grid is a cell
that carries a letter of the word instead of pixels.

    python3 docs/logo/from_sprite.py            # the sections, for candidates.md
"""
from collections import Counter
from pathlib import Path

import render

SPRITE = Path(__file__).parents[2] / "integrations/herdr/sprites/hydra-hydrapet-micro.txt"


def load_sprite(path=SPRITE):
    """Return (rows, palette). Rows are strings of one letter per pixel, `.` transparent."""
    head, _, tail = path.read_text().partition("\n\n")
    rows = [r for r in head.split("\n") if r and not r.startswith("#")]
    pal = {}
    for line in tail.split("\n"):
        parts = line.split()
        if len(parts) == 4 and len(parts[0]) == 1:
            pal[parts[0]] = tuple(int(v) for v in parts[1:])
    return rows, pal


def span(lo, hi, i, n):
    a = lo + i * (hi - lo) // n
    return range(a, max(lo + (i + 1) * (hi - lo) // n, a + 1))


def shrink(rows, box, w, h):
    """Crop box=(x0, y0, x1, y1) and shrink to w x h. A pixel is transparent when under 40% of
    its source block is painted; the eye survives if any of its source pixels is one."""
    x0, y0, x1, y1 = box
    out = []
    for j in range(h):
        line = ""
        for i in range(w):
            cells = [rows[y][x] for y in span(y0, y1, j, h) for x in span(x0, x1, i, w)]
            paint = [c for c in cells if c != "."]
            if len(paint) < 0.4 * len(cells):
                line += "."
            elif "E" in paint:
                line += "E"
            else:
                line += Counter(paint).most_common(1)[0][0]
        out.append(line)
    return out


def compose(pixels, word):
    """Two pixel rows per art line; the word goes on the last line, two columns after the art."""
    if len(pixels) % 2:
        pixels = pixels + ["." * len(pixels[0])]
    pad = ".." + "~" * len(word)
    last = len(pixels) - 2
    grid = [p + (pad if r >= last else "." * len(pad)) for r, p in enumerate(pixels)]
    return grid, len(pixels[0])


def section(name, pixels, word, fits):
    grid, width = compose(pixels, word)
    art = []
    for line in render.plain(grid):
        for k, ch in enumerate(word):
            line = line.replace("~", ch, 1)
        art.append(line.rstrip())
    body = "\n".join(art)
    return f"## {name}\n\n```\n{body}\n```\n\n```paint\n" + "\n".join(grid) + f"\n```\n\n{fits}\n"


def main():
    rows, _ = load_sprite()
    left = shrink(rows, (0, 0, 10, 5), 5, 2)
    mid = shrink(rows, (12, 0, 22, 5), 5, 2)
    one_a = [a + "." + b + "." + b for a, b in zip(left, mid)]
    one_b = ["GEG.GEG.GEG", "LGL.LGL.LGL"]
    bar_a = shrink(rows, (0, 0, 24, 8), 12, 4)
    bar_b = [
        "..H.H...H.H...H.H..",
        ".gGGGg.gGGGg.gGGGg.",
        ".GEGEG.GEGEG.GEGEG.",
        ".LGGGL.LGGGL.LGGGL.",
        "..LGL...LGL...LGL..",
        "...LGGGGGGGGGGGL...",
    ]
    top_a = shrink(rows, (0, 0, 24, 13), 22, 12)
    top_b = shrink(rows, (0, 0, 24, 13), 14, 8)
    parts = [
        section("splash-tiny-sprite", one_a, "hydra",
                "Fits: a shell prompt or a tab title. One row. Made by `from_sprite.py`: the sprite's head crop shrunk to 5 by 2 pixels, the left head once and the middle head twice."),
        section("splash-tiny-hand", one_b, "hydra",
                "Fits: a shell prompt or a tab title. One row. Typed by hand from the sprite's palette: scale, eye and highlight per head."),
        section("splash-bar-sprite", bar_a, "hydra",
                "Fits: the cockpit header. Two rows. Made by `from_sprite.py`: the sprite's top 8 rows shrunk to 12 by 4 pixels."),
        section("splash-bar-hand", bar_b, "hydra",
                "Fits: the cockpit header. Three rows. Typed by hand: three heads on one shared neck bar, colours from the sprite."),
        section("splash-top-sprite", top_a, "hydra",
                "Fits: the README top. Six rows. Made by `from_sprite.py`: the sprite's heads and necks (rows 0 to 12) shrunk to 22 by 12 pixels. The third head is the sliver the 24 pixel sprite has at its right edge."),
        section("splash-top-small", top_b, "hydra",
                "Fits: the README top when the mark should be quiet. Four rows. Made by `from_sprite.py`: the same crop shrunk to 14 by 8 pixels."),
    ]
    print("\n".join(parts))


if __name__ == "__main__":
    main()
