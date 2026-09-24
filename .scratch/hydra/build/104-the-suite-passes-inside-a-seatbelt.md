# 104 - The suite passes inside a seatbelt, so a rung-two pass can clear admission

Type: build
Status: resolved
Blocked by: 98

## What

Found by build ticket 98. Admission runs the whole suite under a seatbelt, and two tests fail there on
any profile: `procinfo::tests::our_own_start_time_matches_ps`, because the seatbelt refuses to exec
setuid `/bin/ps`, and `tests/delegation.rs::a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded`,
because the binary refuses `HYDRA_ROOT` inside a seatbelt. So every rung-two pass on macOS stops at
`cargo test`, and has since build ticket 75. Make both skip visibly inside a seatbelt, the way
`tests/corpus/mod.rs::seatbelt_exec_or_skip` does, and look for any other test with the same shape.

## Done when

The whole suite passes under a real admission profile on a copy of this repository, measured the way
build ticket 98's Outcome measured it, and the two tests still run and pass outside a seatbelt.

## Outcome

Built by a Claude Code session working on Hydra's source, not through `hydra run`.

**What was built.** Two tests skip inside a seatbelt and print why, on stderr, the way
`tests/corpus/mod.rs::seatbelt_exec_or_skip` does.

- `procinfo::tests::our_own_start_time_matches_ps` checks `hydra::sandbox::inside_seatbelt()`
  first and returns with "skipped ...: inside a seatbelt, which refuses to exec setuid /bin/ps".
- `tests/delegation.rs` has a new `binary_takes_hydra_root_or_skip`, and both tests that drive
  `launch_with_marker` call it first. `launch_with_marker` now asserts it is not inside a
  seatbelt, so a later test that forgets the skip fails loudly.

**Other tests with the same shape.** I searched for every test that launches the binary with
`HYDRA_ROOT`, calls `ps`, or calls `sandbox-exec`. One more had the shape and the ticket did not
name it: `a_launch_with_no_marker_is_not_refused_by_the_containment_check`. Inside a seatbelt it
passed without testing anything. Its only assertion is that the output lacks "named no parent",
and the `HYDRA_ROOT` refusal comes first, so that sentence never gets printed. The honest-launch
half of the failing test had the same hole. Both now skip visibly. The rest were already guarded:
`tests/spawn.rs` and `tests/evolve.rs` through `seatbelt_exec_or_skip` or an `inside_seatbelt`
check, and `tests/face.rs` and `tests/launch_phases.rs` by their own checks. The `hydra evolve`
`--may-edit` test in `tests/evolve.rs` sets `HYDRA_ROOT`, but `evolve` refuses the path before
it reads the root, so it tests the same thing inside a seatbelt as outside.

**Measured the way build ticket 98 measured it.** I committed the code first so the tree was an
object, `093f95db`, the tree of this commit before this Outcome was written into it. A throwaway ignored test, not
committed, then used a scratch `Layout::at` under the temp directory and a `standard` proposer
profile from `write_spawn_config`. It ran `prepare_admission` on that tree and `admission_gate`
with the toolchain from `admission_toolchain()`. `PATH` had `~/.local/bin` removed, so
`tests/drift.rs` skipped `claude`. The copy took 0.11 s. All four steps passed in 515 s with a
fresh cargo home, against 416 s in build ticket 98. Two other agents were building on this 8 GB
machine at the same time. The admission profile's sha was `714b7554...b835`. On success
`admission_gate` returns no output, so this run cannot show the skip lines. The next check
shows them.

**Each fix checked by reverting it.** Under `sandbox-exec -p '(version 1)(allow default)'`, with
the fix, `cargo test --lib our_own_start_time_matches_ps` printed its skip line and passed. So
did `cargo test --test delegation`, 14 passed, with two skip lines. With `src/procinfo.rs` and
`tests/delegation.rs` reverted to the previous commit, the same two commands failed.
`procinfo.rs:115` panicked on the `ps` spawn, and
`a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded` panicked at its first
assertion, 13 passed and 1 failed. Outside a seatbelt both tests run in full and pass.

**What the ticket assumed that was false.** Nothing it stated. It named two tests. There was a
third, which passed inside a seatbelt without testing anything, and admission would never have
flagged it.

**Suite, outside any seatbelt**, over both build ticket 100 and this ticket, with `~/.local/bin`
off `PATH`: 746 passed, 0 failed, 0 ignored, across 29 test targets with doc tests included.
`tests/drift.rs` skipped `claude`. `codex` is in `/opt/homebrew/bin`, so its version and
`login status` probes ran; neither starts a session. `cargo fmt --check` and `cargo clippy
--all-targets -- -D warnings` are clean.

**Proposed sentences for build ticket 102.**

For `IMPLEMENTATION.md`, in the rung-two section, replacing build ticket 98's proposed sentence
about the two failing tests:

> Admission runs the whole suite inside a seatbelt. Since build ticket 104 it passes there: the
> suite passed all four steps under a real admission profile on a copy of this repository in
> 515 s. Three tests cannot mean anything inside a seatbelt, so they skip and say so on stderr.
> `procinfo::tests::our_own_start_time_matches_ps` skips because the seatbelt will not exec
> setuid `/bin/ps`. Two tests in `tests/delegation.rs` skip because the binary refuses
> `HYDRA_ROOT` inside a seatbelt. Outside a seatbelt all three run.

For `IMPLEMENTATION.md`'s rough edges, as one line:

> A test that launches the built binary with `HYDRA_ROOT`, or execs a setuid binary, has to skip
> inside a seatbelt or it fails rung two's admission. `tests/delegation.rs::launch_with_marker`
> asserts it is outside one, so a forgotten skip fails loudly.

