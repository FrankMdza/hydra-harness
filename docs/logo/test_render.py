import unittest
import unicodedata

import render

# prefix: (min lines, max lines, max columns, ASCII only)
LIMITS = {"line-": (1, 1, 24, True), "head-": (3, 3, 32, True), "splash-": (3, 5, 40, False)}
DRAWING = {"BOX", "FULL", "LOWER", "UPPER", "LEFT", "RIGHT", "BLACK", "MIDDLE"}


class Candidates(unittest.TestCase):
    def test_six_and_each_fits(self):
        cands = render.load()
        self.assertEqual(len(cands), 6)
        for name, (art, note) in cands.items():
            prefix = next(p for p in LIMITS if name.startswith(p))
            lo, hi, cols, ascii_only = LIMITS[prefix]
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
        counts = {p: sum(n.startswith(p) for n in cands) for p in LIMITS}
        self.assertEqual(counts, {"line-": 2, "head-": 2, "splash-": 2})


if __name__ == "__main__":
    unittest.main()
