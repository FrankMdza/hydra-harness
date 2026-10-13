The goal of adding `tally::chars` involves breaking it down into manageable tasks to ensure clarity and correctness. Here’s how we can approach it:

1. **Implement `chars` function**: Write the function to count non-whitespace characters in a string.
2. **Update tests for `chars`**: Add tests for the empty string, a string of only spaces, and `"a b"`.
3. **Document `chars` in README**: Add a sentence referencing `chars` in the README.md.

### Tasks

- **Implement `chars` logic**: Add the `chars` function to `src/lib.rs` and ensure it correctly counts non-whitespace characters.
- **Add tests for `chars`**: Update the test module in `src/lib.rs` to include the specified test cases.
- **Update README**: Add a sentence mentioning `chars` in the README.md.

```json
[
  {
    "id": "implement_chars_logic",
    "text": "Add the `chars` function to `src/lib.rs` that counts non-whitespace characters in a string. Ensure it behaves similarly to `words` and `lines` but focuses on non-whitespace characters.",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "cargo test -- --show-output | grep -q 'chars counts non-whitespace characters correctly'",
    "deps": [],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "add_chars_tests",
    "text": "Add tests for `chars` in `src/lib.rs` for the following cases: empty string, a string of only spaces, and the string \"a b\".",
    "class": "batch_scripting",
    "grade": "medium",
    "done": "cargo test -- --show-output | grep -q 'all chars tests pass'",
    "deps": ["implement_chars_logic"],
    "paths": ["src/lib.rs"]
  },
  {
    "id": "update_readme_chars",
    "text": "Add a sentence in `README.md` mentioning `chars` in the context of tallying string metrics.",
    "class": "docs",
    "grade": "low",
    "done": "grep -q 'chars' README.md && grep -q 'counts non-whitespace characters' README.md",
    "deps": ["implement_chars_logic", "add_chars_tests"],
    "paths": ["README.md"]
  }
]
```