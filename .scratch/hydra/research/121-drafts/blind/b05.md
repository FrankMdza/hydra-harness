This is one word in one file, so it gets one task. Splitting it into an edit task and a separate test task would only add a handoff. The agent that fixes the README can run `cargo test` in the same session.

The typo appears once, on the README's first prose line: "tallys" becomes "tallies". `src/lib.rs` already says "Count words and lines" and has nothing to fix, and the goal forbids touching Rust anyway. The done check makes three claims a reviewer can run without redoing the work. The misspelling is gone, the correct word is present, and the diff touches only `README.md`. `cargo test` passing is mostly a guard here, since a Markdown edit can't break the three tests, but the goal names it, so it's in the check.

```json
[
  {
    "id": "fix-readme-verb",
    "text": "In README.md, replace the word \"tallys\" with \"tallies\" in the sentence \"A tiny crate that tallys lines and words in a string.\" Change nothing else in the file and touch no other file. Then run `cargo test` from the crate root.",
    "class": "docs",
    "grade": "low",
    "done": "`grep -c tallys README.md` prints 0, `grep -c 'tallies lines and words' README.md` prints 1, `git diff --name-only` lists only README.md, and `cargo test` reports 3 passed, 0 failed.",
    "deps": [],
    "paths": ["README.md"]
  }
]
```
