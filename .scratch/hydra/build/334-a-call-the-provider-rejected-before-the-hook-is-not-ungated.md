# 334 - A tool call the provider rejected before any hook ran is not `Ungated`

Type: build
Status: resolved
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

## Outcome

Built `Finding::RejectedBeforeHook { tool, call_id, error }` for Claude error
results without a gate decision, retaining the first error line. Success,
unclassified and absent results remain `Ungated` when the gate is silent.
Rejected calls do not produce gate leaks. The walk counts both classes apart,
show emits `rejected_before_hook`, and Rungs treats it as a limit rather than a
policy disagreement. Updated IMPLEMENTATION.md and contract 7.7.

Added a synthetic Edit call/result fixture with short stand-ins, guarded by
fixture_corpus.rs's existing builder equality and secret/path checks. Added
three tests: doctor CLI counts, show fields/first line, and a normalization
matrix covering error, success, absent result, hook allow and hook deny.
Targeted verification: 116 tests passed, 1 ignored.

Measured the same one-record scratch corpus with the original release binary
and the patched binary: before, 1 `Ungated`, 0 `RejectedBeforeHook` (class
absent), exit 1; after, 0 `Ungated`, 1 `RejectedBeforeHook`, exit 0. Exactly
one rejected call accounts for the one-call reduction.
That scratch measurement used an envelope with no worktree. The full workspace
run caught that replay then made it a second gated fixture; the checked-in
fixture now names its synthetic worktree, preserving the existing outcome
test's single gated Write. No existing test assertion was changed. Show wraps
the provider's first error line in its usual untrusted-text boundary.

Other-provider fixture inspection: cursor-agent 2026.09.02-c22c1a3 has 2 records
and 2 successful tool results (read and web search); agy 1.1.26 has 1 record
and 0 tool calls; codex 0.154.0 has 2 records and 3 command results, all exit 0.
None demonstrates a malformed call rejected before a hook, so those adapters
remain unchanged. No paid provider probes were launched.

What was false or unavailable: eight specific Claude input-error spellings were
already exempted from `Ungated`; the new class also makes those visible.
The machine-wide before/after counts and a literal copy of seq 326/327 cannot
be obtained under this task's explicit worktree-only access rule. No machine
record was read. The fixture reconstructs the ticket's reported shape in the
existing Claude fixture dialect, rather than claiming the source version or
7 ms timing was independently verified. The driving session must perform the
real-corpus comparison. The gate refused `clock.sleep` as `hydra/unknown-tool`
(`clocksleep` absent from its capability table). It was not retried; waiting
continued through the already-running check sessions.

Final checks, without environment scrubbing, all exit 0:

- `cargo fmt --all --check`: no output.
- `cargo clippy --workspace --all-targets -- -D warnings`: `Finished dev profile [unoptimized + debuginfo] target(s) in 11.54s`.
- `cargo nextest run --release --workspace`: `Summary [44.048s] 1430 tests run: 1430 passed, 4 skipped`.
- `cargo test --release --workspace --doc`: `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.

Also ran `cargo build --release --workspace` successfully. No commits made.

Measured by the session on the Mac after applying, 2026-10-15, with this build's binary:
`hydra doctor --walk` over 963 records prints `RejectedBeforeHook: 49; Ungated: 5`. Before this
build the same corpus reported six `Ungated`; the one that moved is build 327's Edit
(`01a13e24-243b`, `toolu_01Xt5rNaN1rRLwgWUxeSZEK6`), and the five left are the Bash and Read calls
of 2026-10-02 to 2026-10-05 the walk has reported since map 109. Forty-nine calls across the corpus
were rejected by `claude` before any hook ran, none of them a write that happened.
