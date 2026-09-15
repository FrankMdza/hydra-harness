# 60 - The compiler and the gate case-fold differently

Type: build
Status: resolved
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

## Outcome

`fold_ascii` on both sides, one in `src/policy/mod.rs` and one in `src/policy/gate.py`, each
lowercasing ASCII letters and leaving everything else alone. Rust's `casefold` routes through it,
and so do the four `casefold()` calls in the script: `resolve`, the raw pass of
`protected_path_named_by`, `check_slack` and `floor_violation`. The guard matters more on the
Python side than it looks: `'İ'.lower()` returns two characters and would change the length of
a path, which `isascii()` now prevents.

`tests/gate.rs` builds the same gate twice, under a root holding `Straße` and under an ASCII
control, and asserts a `permissive` write into the corpus is refused on both. Measured after
applying: revert the Python fold alone and the non-ASCII root comes back
`hydra/allow: policy permissive allows file_write` while the control still refuses, which is the
failure the review measured. Gate suite 68 of 69; the one red is build ticket 58's test, which waits
on a human editing protected config.

Built by `cursor-agent` on `composer-2.5` under `standard`, run `01a0a572-e4ab`, 4 files and
71 insertions. The report named the ticket 58 failure already in its worktree and attributed it
correctly rather than trying to fix it.
