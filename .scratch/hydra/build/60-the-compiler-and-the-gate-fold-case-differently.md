# 60 - The compiler and the gate case-fold differently

Type: build
Status: open
Blocked by:

## What

`src/policy/mod.rs` lowercases a path with Rust's `to_lowercase`; `gate.py` compares with
Python's `casefold`. The two disagree on characters like `ß` and `ﬁ`: Rust leaves `ß` alone
and Python folds it to `ss`. A layout at a path holding one of them compiles a deny entry the
gate can never match, so **every denial and every scope silently stops applying**, and the
review measured exactly that with a `permissive` write into the corpus coming back allowed
while the ASCII control was refused.

Nobody's home directory is called that today, which is why this is theoretical and why it is
also total the day it fires: it fails open across the board with no message.

Fold both sides the same way. An ASCII-only lowercase map on both sides is the smallest change
and matches what 15.2 actually asks for, which is the macOS case-insensitivity of a filesystem
rather than Unicode equivalence. A test with a non-ASCII layout root belongs in `tests/gate.rs`.

Found by the adversarial review.
