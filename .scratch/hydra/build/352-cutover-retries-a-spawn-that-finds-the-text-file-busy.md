# 352 - The cutover tests retry a spawn that finds the text file busy, so the Linux job stops failing on a fork in another thread

Type: build
Status: resolved
Size: haiku
Blocked by:
Opened: 2026-10-21, by the session, from the first `ci` run on master after build 351 (run
39684267088, commit `d204326`): the Linux job failed one test,
`a_binary_outside_the_checkout_names_the_two_variables_that_find_the_helper` in
`crates/grader/tests/cutover.rs`, at line 100 with `Os { code: 26, kind: ExecutableFileBusy,
message: "Text file busy" }`. Nothing under `crates/grader/` changed since the run that passed
on `26c3ec8`. The test copies the built binary into its scratch with `std::fs::copy` at line 174
and executes the copy through `run` at line 100. The other tests of the file run on sibling
threads of the same process, and each one spawns a child. On Linux a child forked between the
copy's open and its close inherits the write descriptor until its own exec, and an exec of that
file while the descriptor is open fails with `ETXTBSY`. The window is a few microseconds, which
is why the job passed every time before today.

## What

`run` in `cutover.rs` retries a spawn whose error kind is `ExecutableFileBusy`, up to ten times
with a 20 ms sleep between tries, and returns any other error at once. The retry lives in one
helper that takes a closure returning `io::Result<Output>`, so the policy can be tested without
the race. Nothing under `src/` or `crates/grader/src/` changes, and no other test file changes:
`cutover.rs` is the only test in the workspace that copies a built binary and executes the copy
(`grep -rn "fs::copy(RUST" tests crates/*/tests`).

## Contract

Input: a closure that spawns and returns `io::Result<Output>`. Output: the first `Ok`, or the
first error whose kind is not `ExecutableFileBusy`, or the tenth `ExecutableFileBusy` error.
Time: at most ten tries and nine sleeps of 20 ms, under 200 ms added in the worst case; the
usual case costs nothing.

## Cases

In `cutover.rs`, as unit tests of the helper, written first:

1. `a_spawn_that_finds_the_text_file_busy_twice_is_retried_and_succeeds`: the closure returns
   `ExecutableFileBusy` twice then `Ok`; the helper returns the `Ok` and the closure ran three
   times.
2. `a_spawn_that_fails_another_way_is_not_retried`: the closure returns `PermissionDenied`; the
   helper returns that error after one call.
3. `a_spawn_that_stays_busy_gives_up_after_ten_tries`: the closure always returns
   `ExecutableFileBusy`; the helper returns it after exactly ten calls.

The `Output` for case 1 can come from a real `Command::new("true")` run once outside the closure,
or from any process that exits zero, since the helper does not look inside it.

## Do

1. Read `cutover.rs` lines 36 to 110, and the failing test at lines 170 to 190.
2. The three cases, then the helper and the one-line change in `run` that goes through it.
3. `cargo nextest run --release -p hydra-grader --test cutover` green, then the four checks of
   the workspace. Then `## Outcome` under this ticket, with the panic quoted and the sentence
   that the race was not reproduced on macOS, which has no `ETXTBSY`.

## Outcome

Built. `crates/grader/tests/cutover.rs` now has `retry_while_busy`, which takes a spawn closure
and tries it again on `ExecutableFileBusy`: ten tries in all, 20 ms apart. Any other error and
the tenth busy error come back unchanged. `run` goes through it, so every spawn in the file does.
The three cases from the ticket are there under the names it gives. I wrote them before the
helper, but did not run them red first.

The panic on master, run 39684267088:

```
Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }
```

The race was not reproduced here. macOS has no `ETXTBSY` for this path, so the three cases test
the policy with a closure, and the Linux job is the only real check of the fix.

Checks, all on macOS:

- `cargo nextest run --release -p hydra-grader --test cutover`: 10 passed.
- `cargo fmt --all --check`: clean. The first draft had two long assertions, and `rustfmt` on
  that one file wrapped them.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo nextest run --release --workspace`: 1463 passed, 4 skipped.
- `cargo test --release --workspace --doc`: exit 0.

`IMPLEMENTATION.md` gets one paragraph under the cutover section, saying `run` retries and why.
