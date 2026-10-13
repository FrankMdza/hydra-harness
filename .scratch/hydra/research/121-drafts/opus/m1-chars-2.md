The goal is small enough that splitting it further than two tasks costs more than it saves. The code
and the README touch different files and neither reads the other, so they can run side by side.

The function goes into `src/lib.rs` between `lines` and `bump`, as
`text.chars().filter(|c| !c.is_whitespace()).count()`, with a doc comment in the same style as its
neighbours. The goal fixes the three test cases: `""` is 0, `"   "` is 0, and `"a b"` is 2. The
test belongs in the existing `tests` module, and the `use super::{...}` line has to gain `chars` or
the module won't compile. I'd also update the crate doc line, `//! Count words and lines`, since it
would be wrong once a third count exists. That edit costs one line.

The README edit is a single sentence naming `chars`. The opening line currently says the crate
"tallys lines and words", so the natural place is right after it. Fixing "tallys" while in there is
fine, but the goal does not ask for it.

Neither task is on any launch path or security sensitive. Both are low grade.

```json
[
  {
    "id": "chars-fn",
    "text": "In src/lib.rs, add `pub fn chars(text: &str) -> usize` after `lines`, returning the number of characters for which `char::is_whitespace` is false (`text.chars().filter(|c| !c.is_whitespace()).count()`), with a `///` doc comment matching the neighbours. Update the `//!` crate doc to mention characters. In the `tests` module, add `chars` to the `use super::{...}` import and add a test `chars_skips_whitespace` asserting `chars(\"\") == 0`, `chars(\"   \") == 0` and `chars(\"a b\") == 2`. Touch nothing else.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` exits 0 and its output lists `tests::chars_skips_whitespace ... ok` alongside the three existing tests; `grep -n 'pub fn chars' src/lib.rs` prints one line.",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "readme-chars",
    "text": "In README.md, add one sentence after the opening line saying the crate also has `chars`, which counts the non-whitespace characters in a string. Leave the rest of the README unchanged.",
    "class": "docs",
    "grade": "low",
    "done": "`grep -c 'chars' README.md` prints at least 1, and `git diff --stat README.md` shows a change of one or two lines.",
    "deps": [],
    "paths": ["README.md"]
  }
]
```
