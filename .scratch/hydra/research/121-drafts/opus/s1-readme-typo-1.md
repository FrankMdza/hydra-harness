This is one edit, so it gets one task. Splitting a single word change into an edit task and a
separate verify task would cost two agent launches and give the reviewer nothing extra to check.
The agent changes "tallys" to "tallies" on line 3 of `README.md`, leaves `src/lib.rs` and
`Cargo.toml` alone, and runs `cargo test` to confirm the three existing tests still pass. A
README edit can't break the build, but the goal names a green `cargo test` as part of done, so
the agent runs it anyway.

```json
[
  {
    "id": "fix-readme-verb",
    "text": "In README.md, replace the misspelled verb \"tallys\" with \"tallies\" in the sentence \"A tiny crate that tallys lines and words in a string.\" Change nothing else in the file and do not touch any Rust source or Cargo.toml. Then run `cargo test`.",
    "class": "docs",
    "grade": "low",
    "done": "`grep -c tallys README.md` prints 0, `grep -c 'tallies lines and words' README.md` prints 1, `git diff --stat` lists only README.md with one line changed, and `cargo test` reports 3 passed, 0 failed.",
    "deps": [],
    "paths": ["README.md"]
  }
]
```
