This is one task, and splitting it would make it worse. The fix is a single line in `bump`,
`n.wrapping_add(1)` to `n.saturating_add(1)`, plus its doc comment, which still says "wrapping
at 255" and would lie after the change. The tests go in the same `mod tests` block in the same
file. A separate test task would edit `src/lib.rs` in parallel with the fix and conflict. Run
after the fix, it would only repeat what the fix's own check already proves.

Outside the tests nothing calls `bump`. The crate defines it and that is all. So "no call of bump wraps" comes down to the body using `saturating_add` and to no
`wrapping_add` left anywhere in `src/`. A grep checks that.

The existing `bump_adds_one` test already covers 0. I'd add a new test named for the boundary,
asserting 0 to 1, 254 to 255 and 255 to 255, so the goal's three cases sit together and a
reviewer reads them in one place. Edition 2024 needs nothing special here. `saturating_add` on
`u8` has been stable for years.

```json
[
  {
    "id": "saturating-bump",
    "text": "In src/lib.rs, change the body of `bump` from `n.wrapping_add(1)` to `n.saturating_add(1)` and change its doc comment from \"Add one, wrapping at 255.\" to \"Add one, saturating at 255.\". In the existing `mod tests`, add a test `bump_saturates_at_255` asserting `bump(0) == 1`, `bump(254) == 255` and `bump(255) == 255`. Keep the existing tests as they are. Touch no other file.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` exits 0 and lists `tests::bump_saturates_at_255` as ok; `grep -rn wrapping_add src/` prints nothing; `git diff --stat` shows only src/lib.rs changed.",
    "deps": [],
    "paths": ["src/lib.rs"]
  }
]
```
