# 334 - A tool call the provider rejected before any hook ran is not `Ungated`

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 130's answer, question 9. `hydra doctor --walk`
reported one `Ungated { tool: "Edit" }` on build 327's run (`01a13e24-243b-7ab2-858b-bd0f215ee858`,
call `toolu_01Xt5rNaN1rRLwgWUxeSZEK6`, 1 of 28 Edits). The session read the record: the Edit's
`old_string` was not in the file, and Claude Code answered `String to replace not found` as an
`is_error` tool result 7 ms after the call, with no `hook_started` line between the two. Claude Code
validates an Edit's input before it runs PreToolUse hooks, so a rejected Edit has no hook decision
and nothing was written. The walk's `Ungated` means "a call ran with no gate decision", and this
call did not run. Today the finding and a real hook bypass read the same.

## What

In `src/normalize/claude.rs` where the `Ungated` finding is derived (near line 615), a tool call
whose result is an error and which has no hook decision is classed `RejectedBeforeHook`, carrying
the tool, the call id and the first line of the error. `src/walk.rs` counts it apart from `Ungated`
and `src/show.rs` prints it as such. A call with no hook decision whose result is not an error stays
`Ungated`. Measure the other three providers' streams for the same shape before deciding whether the
class applies to them: does `cursor-agent`, `agy` or `codex` emit an error result for a malformed
call without a hook line? Read the fixtures under `tests/fixtures/records/` and say what each one
shows, and leave the three untouched where nothing shows it.

The fixture: the two lines of the 327 record (seq 326 and 327) copied into a synthetic record under
`tests/fixtures/records/claude@<version>/`, with the Edit's `old_string` and `file_path` replaced by
short stand-ins, so the fixture carries the shape and none of the build's text. `fixture_corpus.rs`
guards it as it does the others.

## Do

1. The class, the walk's count, show's line.
2. The fixture and a `tests/doctor.rs` case: the walk over it reports one `RejectedBeforeHook` and
   zero `Ungated`.
3. `hydra doctor --walk` over every record on this machine, before and after, counts in the
   Outcome. `IMPLEMENTATION.md` where the walk's findings are listed; `docs/run-record-contract.md`
   7.7 where `Ungated` is defined, one sentence.

## Done means

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
nextest run --release --workspace`, `cargo test --release --workspace --doc` green, and the real
walk's `Ungated` count down by exactly the rejected calls.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
