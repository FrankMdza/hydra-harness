import re
import unittest
import unicodedata

import render

# prefix: (min lines, max lines, max columns, ASCII only), longest matching prefix wins
LIMITS = {
    "line-": (1, 1, 24, True),
    "head-": (3, 3, 32, True),
    "splash-": (3, 5, 40, False),
    "splash-tiny-": (1, 1, 24, False),
    "splash-bar-": (2, 3, 32, False),
    "splash-top-": (4, 6, 48, False),
}
DRAWING = {"BOX", "FULL", "LOWER", "UPPER", "LEFT", "RIGHT", "BLACK", "MIDDLE"}
BLOCKS = set(" ▀▄█")
ROUND_TWO = ("splash-tiny-", "splash-bar-", "splash-top-")
ESCAPE = re.compile(r"\x1b\[([0-9;]*)m")


def limits(name):
    return LIMITS[max((p for p in LIMITS if name.startswith(p)), key=len)]


class Candidates(unittest.TestCase):
    def test_twelve_and_each_fits(self):
        cands = render.load()
        self.assertEqual(len(cands), 12)
        for name, (art, note, _) in cands.items():
            lo, hi, cols, ascii_only = limits(name)
            self.assertTrue(lo <= len(art) <= hi, name)
            self.assertIn("hydra", "\n".join(art), name)
            self.assertTrue(note.startswith("Fits:"), name)
            for line in art:
                self.assertLessEqual(len(line), cols, name)
                for ch in line:
                    if ascii_only:
                        self.assertTrue(32 <= ord(ch) < 127, f"{name}: {ch!r}")
                    elif ord(ch) > 127:
                        word = unicodedata.name(ch).split()[0]
                        self.assertIn(word, DRAWING, f"{name}: {ch!r}")
        prefixes = ("line-", "head-", "splash-tiny-", "splash-bar-", "splash-top-")
        counts = {p: sum(n.startswith(p) for n in cands) for p in prefixes}
        self.assertEqual(counts, dict.fromkeys(prefixes, 2))
        self.assertEqual(sum(n.startswith("splash-") for n in cands), 8)

    def test_round_two_has_both_forms(self):
        pal = render.palette()
        allowed = {"38;2;%d;%d;%d" % c for c in pal.values()}
        allowed |= {"48;2;%d;%d;%d" % c for c in pal.values()} | {"0"}
        for name, (art, _, paint) in render.load().items():
            if not name.startswith(ROUND_TWO):
                continue
            self.assertIsNotNone(paint, name)
            self.assertEqual(len(paint), 2 * len(art), name)
            # the plain form has no escape codes and only block characters and the word
            for line in art:
                self.assertIsNone(ESCAPE.search(line), name)
                self.assertTrue(set(line) <= BLOCKS | set("hydra"), name)
            # the art is the shape of the paint, so the two forms cannot drift apart
            for line, shape in zip(art, render.plain(paint)):
                for x, ch in enumerate(shape):
                    if ch != "~":
                        self.assertEqual(line.ljust(len(shape))[x], ch, f"{name} col {x}")
            # the colour form uses only the sprite's palette
            coloured = render.colour(art, paint, pal)
            self.assertEqual(len(coloured), len(art), name)
            used = set()
            for line in coloured:
                used |= {m.group(1) for m in ESCAPE.finditer(line)}
                self.assertTrue(line.endswith("\x1b[0m"), name)
                plain = ESCAPE.sub("", line)
                self.assertTrue(set(plain) <= BLOCKS | set("hydra"), name)
            self.assertLessEqual(used, allowed, name)
            self.assertTrue(any(u.startswith("38;2;") for u in used), name)
            for letter in set("".join(paint)) - set(".~"):
                self.assertIn(letter, pal, f"{name}: {letter!r} is not in the sprite palette")


if __name__ == "__main__":
    unittest.main()
