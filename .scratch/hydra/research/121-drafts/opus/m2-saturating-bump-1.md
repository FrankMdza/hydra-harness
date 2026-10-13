This is one task, not several. The change is a single line in `bump`, its doc comment, and three assertions in the test module of the same file. Splitting the fix from the tests would put two agents on `src/lib.rs` at once and force a merge for nothing. Tests written alone would also go red at 255 until the fix lands, so neither half can be checked by itself.

The one agent swaps `wrapping_add(1)` for `saturating_add(1)` and changes the doc comment from "wrapping at 255" to "stopping at 255". It also adds a test that asserts `bump(0) == 1`, `bump(254) == 255` and `bump(255) == 255`. The existing `bump_adds_one` test stays as it is. `bump` has no other callers in the crate, so "no call wraps" reduces to the function body. A grep confirms it.

The class is `rust_off_launch_path` because `tally` is a scratch crate and launches nothing. The grade is low.

```json
[
  {
    "id": "saturate-bump",
    "text": "In src/lib.rs, change bump to use n.saturating_add(1) instead of n.wrapping_add(1), and change its doc comment to say it adds one and stops at 255. In the tests module of the same file, add a test named bump_saturates_at_255 that asserts bump(0) == 1, bump(254) == 255 and bump(255) == 255. Leave the existing tests unchanged. Touch no other file.",
    "class": "rust_off_launch_path",
    "grade": "low",
    "done": "`cargo test` passes with 4 tests, bump_saturates_at_255 among them; `grep -n wrapping src/lib.rs` prints nothing; `git diff --stat` shows only src/lib.rs changed.",
    "deps": [],
    "paths": ["src/lib.rs"]
  }
]
```
