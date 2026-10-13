This is one task. The goal is a single assertion in a test module that already exists, and splitting it would cost more in handoff than the work itself. The agent adds a new `#[test]` function, `words_counts_three_tokens`, inside `mod tests` in `src/lib.rs`, asserting `words("a b c") == 3`. `words` is already imported there, so the `use` line stays as it is. I'd keep it as its own function rather than folding a fourth assert into `words_counts_split_tokens`, because the goal says "add one unit test" and a reviewer should be able to see that test by name in the `cargo test` output.

The check is twofold. `cargo test words_counts_three_tokens` must report `1 passed`, and `git diff` must touch nothing outside the `mod tests` block, so no production code changed.

```json
[
  {
    "id": "add-words-test",
    "text": "In src/lib.rs, inside the existing `#[cfg(test)] mod tests`, add a new test function `words_counts_three_tokens` that asserts `assert_eq!(words(\"a b c\"), 3);`. Do not edit `words`, `lines`, `bump`, the existing tests, Cargo.toml or README.md. The `use super::{bump, lines, words};` line already imports `words`, so leave it alone.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test words_counts_three_tokens` prints `test tests::words_counts_three_tokens ... ok` and `1 passed`; `cargo test` passes all four tests; `git diff --stat` shows only src/lib.rs, and every added line sits inside `mod tests`.",
    "deps": [],
    "paths": ["src/lib.rs"]
  }
]
```
