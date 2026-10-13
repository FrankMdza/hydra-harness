This goal is two edits in two files, and they don't touch each other. I'd split it that way and
no further. Splitting the function from its tests would buy nothing, because the tests are three
asserts and the agent writing `chars` is the one who knows what it returns.

The function goes in `src/lib.rs` after `lines` and before `bump`, so the three counters sit
together. `text.chars().filter(|c| !c.is_whitespace()).count()` is the whole body. It counts
Unicode scalar values, not bytes, which matches what "characters" means to a reader. The doc
comment should say so in one line, the way `words` and `lines` each have one. The new test goes
in the existing `tests` module, and the `use super::{...}` line has to gain `chars` or the
build fails. Expected values are `chars("") == 0`, `chars("   ") == 0` and `chars("a b") == 2`.
The crate-level `//!` line says "Count words and lines", and it would be wrong once `chars`
exists, so the same task should update it.

The README task has no dependency on the code. It adds one sentence naming `chars`. Its first
line already lists what the crate counts, so the cleanest edit extends that sentence or adds one
right after it. The agent shouldn't rewrite the rest of the README, and shouldn't fix the
"tallys" typo unless it's rewriting that same sentence anyway.

Both tasks are low grade. Neither is on Hydra's launch path, since `tally` is a scratch crate.

```json
[
  {
    "id": "chars-fn",
    "text": "In src/lib.rs, add `pub fn chars(text: &str) -> usize` after `lines` and before `bump`, returning `text.chars().filter(|c| !c.is_whitespace()).count()`, with a one-line doc comment saying it counts non-whitespace characters. Update the crate-level `//!` comment so it names characters as well as words and lines. In the existing `tests` module, add `chars` to the `use super::{...}` import and add a test `chars_skips_whitespace` asserting `chars(\"\") == 0`, `chars(\"   \") == 0` and `chars(\"a b\") == 2`. Leave `words`, `lines`, `bump` and their tests unchanged.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` exits 0 and its output lists `tests::chars_skips_whitespace ... ok` alongside the three existing tests; `grep -n 'pub fn chars' src/lib.rs` prints one line.",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "readme-chars",
    "text": "In README.md, add one sentence naming `tally::chars` and saying it counts the non-whitespace characters in a string. Either extend the opening sentence about what the crate tallies or put the new sentence right after it. Do not change the rest of the README.",
    "class": "docs",
    "grade": "low",
    "done": "`grep -c 'chars' README.md` prints at least 1, and `git diff --stat README.md` shows only a few changed lines.",
    "deps": [],
    "paths": ["README.md"]
  }
]
```
