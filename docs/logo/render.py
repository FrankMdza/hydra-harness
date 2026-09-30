#!/usr/bin/env python3
"""Print a logo candidate from candidates.md by name, or all of them with no argument.

The colour form (24-bit escapes, the sprite's palette, the half-block rule of hydra-mascot) goes
to a terminal or with --color. The plain form, the same shape in block characters only, goes
everywhere else. Candidates from round one have no paint grid and print as they are.
"""
import re
import sys
from pathlib import Path

CANDIDATES = Path(__file__).with_name("candidates.md")
SPRITE = Path(__file__).parents[2] / "integrations/herdr/sprites/hydra-hydrapet-micro.txt"


def load(path=CANDIDATES):
    """Return {name: (art_lines, note, paint_lines or None)} in file order. Paint is two pixel
    rows per art line, one letter per pixel, `.` transparent, `~` a cell that holds a letter."""
    pattern = r"^## (\S+)\n\n```\n(.*?)\n```\n\n(?:```paint\n(.*?)\n```\n\n)?(.*?)$"
    found = {}
    for m in re.finditer(pattern, path.read_text(), re.M | re.S):
        paint = m.group(3).split("\n") if m.group(3) else None
        found[m.group(1)] = (m.group(2).split("\n"), m.group(4).strip(), paint)
    return found


def palette(path=SPRITE):
    """{letter: (r, g, b)} from the sprite file's `X r g b` lines."""
    pal = {}
    for line in path.read_text().partition("\n\n")[2].split("\n"):
        parts = line.split()
        if len(parts) == 4 and len(parts[0]) == 1:
            pal[parts[0]] = tuple(int(v) for v in parts[1:])
    return pal


def block(top, bot):
    """The plain glyph for two pixels. The eye counts as a hole, so it shows as a notch."""
    top = "." if top == "E" else top
    bot = "." if bot == "E" else bot
    if top == "." and bot == ".":
        return " "
    if bot == ".":
        return "▀"
    if top == ".":
        return "▄"
    return "█"


def plain(paint):
    """Plain lines from a paint grid; `~` stays `~` for the caller to fill with the word."""
    lines = []
    for top, bot in zip(paint[0::2], paint[1::2]):
        lines.append("".join("~" if t == "~" else block(t, b) for t, b in zip(top, bot)))
    return lines


def colour(art, paint, pal):
    """The colour form, with hydra-mascot's rule: top pixel as foreground of a `▀`, bottom pixel
    as its background, or a lone bottom pixel as the foreground of a `▄`."""
    def fg(c):
        return "\x1b[38;2;%d;%d;%dm" % pal[c]

    def bg(c):
        return "\x1b[48;2;%d;%d;%dm" % pal[c]

    lines = []
    for i, text in enumerate(art):
        top, bot = paint[2 * i], paint[2 * i + 1]
        out = []
        for x in range(len(top)):
            ch = text[x] if x < len(text) else " "
            t, b = top[x], bot[x]
            if t == "~":
                out.append("\x1b[0m" + ch)
            elif t == "." and b == ".":
                out.append("\x1b[0m ")
            elif b == ".":
                out.append("\x1b[0m" + fg(t) + "▀")
            elif t == ".":
                out.append("\x1b[0m" + fg(b) + "▄")
            else:
                out.append(fg(t) + bg(b) + "▀")
        lines.append("".join(out) + "\x1b[0m")
    return lines


def show(art, paint, use_colour, pal):
    if paint and use_colour:
        return colour(art, paint, pal)
    return art


def main(argv):
    use_colour = sys.stdout.isatty()
    if "--color" in argv:
        use_colour = True
        argv = [a for a in argv if a != "--color"]
    cands = load()
    pal = palette()
    for name in argv or list(cands):
        if name not in cands:
            print(f"no candidate {name!r}; have: {', '.join(cands)}", file=sys.stderr)
            return 1
        art, note, paint = cands[name]
        print(name)
        print("\n".join(show(art, paint, use_colour, pal)))
        print(note + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
