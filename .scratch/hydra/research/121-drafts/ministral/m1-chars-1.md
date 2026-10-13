The goal involves adding a new function `tally::chars` to count non-whitespace characters in a string, alongside existing `words` and `lines` functions. The implementation should include tests for edge cases like an empty string, a string of only spaces, and `"a b"`. The function should be documented in the README.

Here’s how to break this down into tasks:

1. **Implement `chars` function**: Write the function to count non-whitespace characters in a string.
2. **Update tests for `chars`**: Add tests for the empty string, a string of only spaces, and `"a b"`.
3. **Document `chars` in README**: Add a sentence mentioning `chars` in the README.

---

```json
[
  {
    "id": "implement_chars",
    "text": "Add the `chars` function to `src/lib.rs` that counts non-whitespace characters in a string. Use `chars()` from `std::string::Chars` or `filter`/`map` over whitespace characters if needed.",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "cargo test -- --test-chars",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "add_chars_tests",
    "text": "Add tests for `chars` in `src/lib.rs` for the following cases:
    - Empty string (`""`),
    - String of only spaces (`\"  \"`),
    - String with spaces (`\"a b\"`). Use `assert_eq!` to verify the results.",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "cargo test -- --test-chars",
    "deps": ["implement_chars"],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "update_readme",
    "text": "Add a sentence in `README.md` mentioning `chars` in the context of tallying string metrics, e.g., 'Count non-whitespace characters in a string.'",
    "class": "docs",
    "grade": "low",
    "done": "grep -q \"chars\" README.md",
    "deps": ["implement_chars", "add_chars_tests"],
    "paths": ["README.md"]
  }
]
```