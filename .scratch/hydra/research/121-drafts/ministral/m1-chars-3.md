The goal involves adding a new function `tally::chars` to count non-whitespace characters in a string, alongside existing `words` and `lines` functions. The function should be tested for edge cases like empty strings, strings with only spaces, and `"a b"`. The README should include a brief mention of the new function.

Here’s how we can break this down into tasks:

1. **Implement `chars` function**: Add the function to `src/lib.rs` to count non-whitespace characters.
2. **Add tests for `chars`**: Write tests for the empty string, a string of only spaces, and `"a b"`.
3. **Update README**: Add a sentence referencing `chars` in the README.md.

---

```json
[
  {
    "id": "implement_chars",
    "text": "Add the `chars` function to `src/lib.rs` that counts non-whitespace characters in a string. The function should match the style of `words` and `lines`.",
    "class": "batch_scripting",
    "grade": "low",
    "done": "`cargo test` passes and `chars` is defined in `src/lib.rs`",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "test_chars",
    "text": "Write tests for `chars` in `src/lib.rs` for the following cases:
    - Empty string (`""`)
    - String of only spaces (`\"  \"`)
    - String with spaces (`\"a b\"`).
    Ensure the tests are placed in the existing test module.",
    "class": "batch_scripting",
    "grade": "low",
    "done": "All tests for `chars` pass (`cargo test` green) and are in the test module.",
    "deps": ["implement_chars"],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "update_readme",
    "text": "Add a sentence to `README.md` referencing `chars` in the context of tallying string metrics. Example: `Count non-whitespace characters in a string.`",
    "class": "docs",
    "grade": "low",
    "done": "The updated `README.md` exists and includes the sentence about `chars`.",
    "deps": ["implement_chars", "test_chars"],
    "paths": ["README.md"]
  }
]
```