This is one edit to one line. The README's first paragraph says "tallys" once, and the fix is to write "tallies" there. Nothing in `src/lib.rs` or `Cargo.toml` mentions the word, so the Rust stays as it is. Splitting this into more than one task would cost more in handoff than the work itself. The single task makes the edit and runs the two checks the goal names: the misspelling is gone, and `cargo test` still passes its three tests. A reviewer can confirm both from the diff and one command, and `git diff --stat` should show `README.md` as the only changed file.

```json
[
  {
    "id": "fix-readme-verb",
    "text": "In README.md, replace the word \"tallys\" with \"tallies\" in the sentence \"A tiny crate that tallys lines and words in a string.\" Change nothing else in the file and touch no other file. Then run cargo test.",
    "class": "docs",
    "grade": "low",
    "done": "`grep -c tallys README.md` prints 0, `grep -c 'tallies lines and words' README.md` prints 1, `git diff --stat` lists only README.md, and `cargo test` reports 3 passed, 0 failed.",
    "deps": [],
    "paths": ["README.md"]
  }
]
```
