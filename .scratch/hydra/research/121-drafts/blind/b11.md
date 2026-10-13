One task is enough here. The goal is a single assertion in a test module that already exists, and splitting it would only add handoffs. The existing `words_counts_split_tokens` test covers empty, blank and single-word input, so the new test goes beside it as its own `#[test]` function, `words_counts_three_tokens`, asserting `words("a b c") == 3`. `words` is already imported in `mod tests`, so the `use` line stays as it is. Nothing above `#[cfg(test)]` changes, and a reviewer can confirm that with a diff that touches only lines inside the test module.

Done is mechanical: `cargo test words_counts_three_tokens` reports one test run and passing, and a plain `cargo test` still passes all four tests.

```json
[
  {
    "id": "add-words-test",
    "text": "In src/lib.rs, inside the existing `#[cfg(test)] mod tests`, add a new `#[test] fn words_counts_three_tokens()` that asserts `assert_eq!(words(\"a b c\"), 3);`. Place it next to `words_counts_split_tokens`. `words` is already imported, so do not change the `use` line. Do not change any code outside the test module, and do not edit Cargo.toml or README.md.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test words_counts_three_tokens` prints `test tests::words_counts_three_tokens ... ok` and `1 passed`; `cargo test` reports 4 passed, 0 failed; `git diff` touches only src/lib.rs and only lines inside `mod tests`.",
    "deps": [],
    "paths": ["src/lib.rs"]
  }
]
```
