This goal is small enough for two tasks, and they touch different files, so they can run in parallel.

The first adds `chars` to `src/lib.rs` after `lines` and before `bump`, so the three counters sit together. The body is `text.chars().filter(|c| !c.is_whitespace()).count()`, which counts Unicode scalar values and not bytes. It gets a doc comment in the style of the other two. The new test asserts `chars("") == 0`, `chars("   ") == 0` and `chars("a b") == 2`, and the `use super::{...}` line gains `chars`. The module doc `//! Count words and lines in a string.` should name characters too, or it goes stale the moment the function lands.

The second edits the README. Its first sentence already says what the crate tallies, so it should name non-whitespace characters there instead of adding a separate line. The sentence also misspells "tallies" as "tallys", and fixing that is cheap while the sentence is open anyway. The README check depends on nothing in the code, so the task has no deps. The goal's final `cargo test` run belongs to the first task.

```json
[
  {
    "id": "chars-fn",
    "text": "In src/lib.rs, add `pub fn chars(text: &str) -> usize` after `lines`, returning `text.chars().filter(|c| !c.is_whitespace()).count()`, with a one-line doc comment like the others: `/// Number of non-whitespace characters.`. Update the module doc to `//! Count words, lines and non-whitespace characters in a string.`. Add `chars` to the test module's `use super::{...}` and a test `chars_skips_whitespace` asserting `chars(\"\") == 0`, `chars(\"   \") == 0` and `chars(\"a b\") == 2`. Leave the existing functions and tests untouched.",
    "class": "batch_scripting",
    "grade": "low",
    "done": "`cargo test` exits 0 and its output lists `tests::chars_skips_whitespace ... ok` next to the three existing tests",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "chars-readme",
    "text": "In README.md, rewrite the first sentence so it names the new function, for example: `A tiny crate that tallies lines, words and non-whitespace characters (`chars`) in a string.` This also fixes the misspelling \"tallys\". Leave the rest of the paragraph as it is.",
    "class": "docs",
    "grade": "low",
    "done": "`grep -n 'chars' README.md` prints the rewritten first sentence, and `grep -c tallys README.md` prints 0",
    "deps": [],
    "paths": ["README.md"]
  }
]
```
