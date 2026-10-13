This is one task. The goal says "medium", but the change is three lines in one file: swap `wrapping_add` for `saturating_add` in `bump`, fix the doc comment that still says "wrapping at 255", and add a test that pins 0, 254 and 255. Splitting it would mean two agents editing the same function and the same test module, and the second would wait on the first anyway.

The only callers of `bump` in the crate are the tests, so "no call of `bump` wraps" comes down to the function body itself. The reviewer's check is a grep that finds no `wrapping_add` left in `src/lib.rs`, plus `cargo test` green with the new test in the list. The existing `bump_adds_one` test stays, since 0 to 1 and 1 to 2 still hold under saturating add.

```json
[
  {
    "id": "saturating-bump",
    "text": "In src/lib.rs, change `bump` from `n.wrapping_add(1)` to `n.saturating_add(1)` and change its doc comment from \"Add one, wrapping at 255.\" to \"Add one, saturating at 255.\". In the existing `tests` module, keep `bump_adds_one` and add a test `bump_saturates_at_max` asserting `bump(0) == 1`, `bump(254) == 255` and `bump(255) == 255`. Touch no other file.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` exits 0 and lists `tests::bump_saturates_at_max ... ok` and `tests::bump_adds_one ... ok`; `grep -n wrapping src/lib.rs` prints nothing",
    "deps": [],
    "paths": ["src/lib.rs"]
  }
]
```
