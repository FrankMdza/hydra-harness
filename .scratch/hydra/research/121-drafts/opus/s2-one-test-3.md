This is one task. Splitting it would cost more than doing it. The test goes in the existing `mod tests` in `src/lib.rs`, beside `words_counts_split_tokens`, under its own name so `cargo test` reports it separately. `words` is already imported there, so the module needs no `use` change. No production code changes, and a reviewer can check that with `git diff`: the only hunk should sit inside `mod tests`.

`words("a b c")` splits on whitespace into three tokens, so the test passes against today's code. It runs green on its first try, which is what the goal asks for.

```json
[
  {
    "id": "words-three-test",
    "text": "In src/lib.rs, inside the existing #[cfg(test)] mod tests, add a new #[test] fn words_counts_three_tokens() that asserts assert_eq!(words(\"a b c\"), 3). Put it next to words_counts_split_tokens. Do not change any code outside mod tests and do not touch the use line, since words is already imported.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test words_counts_three_tokens` prints `test tests::words_counts_three_tokens ... ok` and `1 passed`; `cargo test` passes all four tests; `git diff src/lib.rs` shows added lines only, all inside mod tests.",
    "deps": [],
    "paths": ["src/lib.rs"]
  }
]
```
