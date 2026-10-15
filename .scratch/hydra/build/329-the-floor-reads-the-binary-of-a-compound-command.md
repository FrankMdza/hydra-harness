# 329 - The floor reads `then`, `do` and `else` as the binary, so `if true; then git push --force; fi` passes

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from build 319's Outcome ("a hole I found and didn't fix").
The pattern floor of harness spec 7.4 reads each segment's first word as the program it runs
and checks it against the floor targets. A segment that starts with a shell keyword, `then`,
`do`, `else`, `elif`, `{`, `!`, `time`, hands the real program as its second word, so `if true;
then git push --force; fi` passed both the old gate and build 319's on a compiled `standard` gate.
The same shape works with `while :; do`, `else`, and `{ git push --force; }`.

## What

In `src/policy/gate.py`, where `floor_violation` reads a segment's binary (and
`protected_path_named_by` where it reads tokens), skip the shell's reserved words and the
constructs that precede a simple command before taking the binary: `if`, `then`, `elif`, `else`,
`fi`, `for`, `while`, `until`, `do`, `done`, `case`, `esac`, `{`, `}`, `!`, `time`, `function`,
and a `name=value` assignment prefix (which `first_word` from build 319 already skips). Also
`command`, `builtin`, `exec`, `nohup`, `env` with its assignments, `nice`, `timeout <n>`,
`xargs`: wrappers whose argument is the program. Read what `shell_hazard` already covers before
adding a list, and share one.

The corpus under `tests/corpus/` must keep refusing what it refuses; `tests/gate.rs` gets one
case per construct above, each red before the fix, the `if true; then git push --force; fi` one
first. Measure the result on a compiled gate as 319 did and paste the table in the Outcome.

## Do

1. Read build 319's Outcome and `shell_layout`'s docstring in `gate.py`.
2. The keyword and wrapper skip, as one function the two passes call.
3. The cases; the four checks.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green. Amend
`IMPLEMENTATION.md` where the pattern floor is described, beside 319's sentences.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
