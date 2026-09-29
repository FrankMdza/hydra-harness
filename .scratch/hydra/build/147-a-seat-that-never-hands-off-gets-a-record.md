# 147 - A seat that never hands off gets a record

Type: build
Status: resolved
Blocked by:

## What

Map ticket 84, resolved 2026-09-29. Read its `## Answer` first.

When a gated interactive session ends, `Served::stop` in `src/seats.rs` waits up to
`HANDOFF_BUDGET`, 60 seconds, for a seat still inside `run::run` to hand off to its detached
watcher. A seat still launching after that dies with the process and is named on stderr. Its run id
is in the `runs` of the session's `seat_request` line, and no record holds what happened to it.

The answer: that seat gets its own record with outcome `never_started`, the outcome a launch that
fails after its worktree exists already writes (`record_never_started`, `src/run.rs` near 936).
The session's record gets no new line type.

## Do

**First, the check the answer asks for, before any code.** Read `docs/run-record-contract.md` and
`src/record/` for what a `never_started` record must carry. If the contract does not admit one for
a run whose worktree may not exist, or whose line 0 was never written, stop and say so in your
report. That is a decision for the map and not yours.

If it does:

1. Tests first in `tests/seats.rs`, on the stub provider the file already uses, each shown failing
   before the change. A seat that does not hand off inside the budget leaves a record under its
   run id, with outcome `never_started`, `parent_run_id` the session's, and a reason that says the
   session ended before the handoff. The record passes the contract's validator.
2. A seat that hands off inside the budget is untouched: one record, written by its own watcher,
   and no second one.
3. A seat that had already written its line 0 when the session ended does not get a second line 0.
   Say in the report which of the two the code does for that seat, and why.
4. The line on stderr stays, since a human at the terminal still wants it.
5. `IMPLEMENTATION.md`: amend the passage on the launcher in the same change. `grep -n` for
   `HANDOFF_BUDGET` or `seat_request`, and read at most 80 lines around it.

## Done when

- The tests above exist, each failed before the change, and `cargo test --test seats` passes with
  `cargo test --lib`, `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`.
- `hydra doctor --walk` accepts the record the test wrote, or the report says why the walk could
  not be run inside the worktree.
- The report says what stays untested on stubs and needs a live launch.

## Outcome

Resolved 2026-09-29. Built by `claude` opus on run 01a0ebc8-c4bf under `standard`, about 4.02M
tokens. The session applied the diff as written.

**The check came first and passed.** Contract 5.1 allows `worktree: null` and 5.2 a null `pid`
for a run that never had a process. The validator checks that the `run_start` fields are present
and not that they are filled. And the `containment` and `policy_narrowing` refusals in
`src/run.rs` already write a line 0 with no worktree, no provider version and a null pid. So the
answer of map ticket 84 amends nothing.

What was built. `run::record_unlaunched` builds line 0 from the request alone and writes it
through the existing `never_started`. The launcher keeps each seat's request until the seat lands,
and `Served::stop_within` writes a record for each seat that did not. `Launcher::new` takes a
`Layout` where it took the approvals path. The line on stderr stays.

`Writer::create` decides between the launcher and the seat. A seat that wrote its own file first
keeps it, and the launcher appends nothing, because the contract allows one writer per file and
the seat's watcher may still be alive.

**The ticket and map ticket 84 were wrong about one word.** `never_started` is a name in the code
and not an outcome. The record derives `failed` under contract 8, and the test asserts `failed`.
Map ticket 84 now says so.

Tests, three added to `tests/seats.rs`. The first failed before the change with "seat has no
record". The other two check that nothing extra is written, so they could not fail before a change
that wrote anything; the run showed each can fail by breaking the code on purpose. On master:
fmt and clippy clean, seats 26, lib 217, delegation 14, launch_phases 1, record_contract 20. The
release binary was rebuilt.

Left open, and written into `IMPLEMENTATION.md`:

- A seat that hands off between the end of the wait and the launcher's write loses the file to
  the launcher. Its watcher then refuses to start, so the record stays true: nothing ran.
- A seat whose worktree already existed is recorded with `worktree: null`, and `hydra reap`'s
  orphan pass is what finds the checkout.
- The tests read the real `config/preamble.md` through `preamble::load()`.

For the grader: a seat that died this way is a `failed` row under its provider and model, and it
is not a failure of the agent. It joins `rate_limited` and `gated` as something a label must
leave out. No ticket yet.

Nothing here ran against a real provider. The first live panel is the measure.
