# 302 - The suite is green inside a run: three tests stop reading the launcher's git pointers

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from the Outcomes of builds 295 and 296, which both had to
prefix `cargo test` with `env -u GIT_OBJECT_DIRECTORY -u GIT_ALTERNATE_OBJECT_DIRECTORIES` to
see the suite green inside a Hydra run.

## What

The env allowlist (`src/env.rs`, map 74) hands every run two git pointers so the run's git
writes into its own object store: `GIT_OBJECT_DIRECTORY` and `GIT_ALTERNATE_OBJECT_DIRECTORIES`.
A builder working on Hydra's own source inherits them, and three tests in `src/run/mod.rs`
(the ones near `a_run_with_a_repo_is_handed_its_own_object_directory`, around line 1360) then
fail: they compose a child env from the process env and compare the pointers they expect
against what `compose_child_env` produced, and the inherited values leak into the comparison.
On a plain shell the variables are unset and the tests pass as written. Every builder since 295
has been told the `env -u` incantation in its task text, which is a trap every ticket has to
carry and a lie about the suite: "green" should not depend on who launched the shell.

## Do

1. Reproduce: with both variables exported to any path, `cargo test --lib run::` on master shows
   the failing tests. Name them in the Outcome.
2. Make those tests independent of the process env. The right shape is the one `git_config.rs`
   and `launch_phases.rs` already use after build 296: a test that needs a controlled env
   re-executes itself in a child with `Command::env_remove`, or builds the env it composes from
   an explicit map rather than `std::env::vars`. Prefer the second where `compose_child_env`
   takes or can take an env source, and say in the Outcome which one you chose and why.
3. Grep the whole workspace for other tests that read the two pointers from the process env
   (`tests/spawn.rs:1634` reads an export list, which is fine; look for `std::env::var` of the
   two names in test code) and treat them the same way.
4. `cargo test --release --workspace` green twice inside your run **without** the `env -u`
   prefix, and once with the two variables exported to a nonsense path on top of what the run
   already gave you.
5. Amend `IMPLEMENTATION.md`'s sentence about the trap if it has one; if it does not, add one
   line under the section that describes the allowlist saying the suite no longer reads the two
   pointers from the process env.

## Done means

The suite is green inside a Hydra run with no `env -u`, and the Outcome names the tests that
used to fail and the mechanism that fixed them. No test loses an assertion: the three tests
still check that a run with a repo gets its own store and that a run without one gets no
pointer.

## Outcome

2026-10-13: partial implementation. Status remains claimed because the suite fails.

Reproduced with cargo test --lib run:: and the inherited object pointers. Three failures:
- a_run_with_a_repo_is_handed_its_own_object_directory
- inject_skills_does_not_treat_a_codex_copy_as_a_collision_with_itself
- a_failed_injection_leaves_the_names_it_wrote_where_the_failure_record_reads_them

The three tests now re-execute themselves in a child test binary with Command::env_remove
for GIT_OBJECT_DIRECTORY and GIT_ALTERNATE_OBJECT_DIRECTORIES. The parent verifies exit
status and output showing that the exact named test ran. All original assertions remain.
The no-repo composition test also checks both maps omit both pointers for all providers.
No production code or dependency changed. IMPLEMENTATION.md records the remaining failure.

Chose re-execution because compose_child_env and ChildEnv::build take no env source, and
the actual failures occur in repository setup subprocesses. An explicit composition map
would not isolate those commands. Contrary to the ticket description, inherited pointers
did not leak into composition comparisons: setup metadata and commits were missing when
Hydra read the fixtures after removing those pointers.

The whole-workspace search found no direct std::env::var or var_os reads of the two names
in test code. Other occurrences inspect export lists or command removals, or remove the
pointers already. Subprocess inheritance extends beyond the direct reads named by the ticket.

Verification:
- cargo test --lib run:: passes: all six pass after the fix.
- cargo build, cargo fmt --all --check, and
  cargo clippy --workspace --all-targets -- -D warnings: pass.
- cargo test --release --workspace, twice with the run environment intact: 256 library
  tests and 11 apprentice tests pass, then cleanup has 12 passing and 34 failing tests
  each time. Both commands exit 101. Cargo stops at that integration binary.
- The release workspace command with both pointers set to nonexistent relative paths:
  256 library tests and 11 apprentice tests pass; all 46 cleanup tests fail, exit 101.
  No env -u prefix was used in any verification command.

The done condition remains unmet: cleanup repository fixtures also inherit the pointers
through setup subprocesses. Further integration-fixture isolation and verification beyond
that first failing binary are needed to make the full workspace green. The requested
three-test fix is insufficient; this run does not claim two green suites or resolve the ticket.

2026-10-13: second run, resolved. Added tests/common/git.rs, a small command
constructor that removes GIT_OBJECT_DIRECTORY and GIT_ALTERNATE_OBJECT_DIRECTORIES
from fixture subprocesses without mutating the parallel test runner environment.
Twelve integration files use it: tests/cleanup.rs, tests/copilot_repo_and_tools.rs,
tests/curate.rs, tests/drift.rs, tests/evolve.rs, tests/git_config.rs,
tests/launch_phases.rs, tests/outcome.rs, tests/record_contract.rs, tests/skills.rs,
tests/spawn.rs and tests/worktree.rs. The shared helper is a standalone module
under tests/common/, included directly so fixtures need not import the larger
record/corpus helpers in tests/common/mod.rs. Explicit fixture object stores are
still applied after construction in cleanup::run_git. The single constructors
in crates/grader/tests/originals.rs and crates/puppets/tests/differential.rs remove
the pointers locally; both already used env_clear. tests/export.rs and
tests/sandbox_copilot.rs already remove both pointers, and the grader differential
queue fixture already clears its environment. No production code, assertions or
dependencies changed. Also updated this ticket and replaced the build ticket 302
paragraph in IMPLEMENTATION.md with the final subprocess-inheritance mechanism.
The first nonsense-path pass additionally exposed six copilot failures: its
fixture script launches git internally. The shared helper now also isolates that
script command. After this final adjustment, repeated the requested sequence:
cargo test --release --workspace with the run environment intact, twice, then
once with GIT_OBJECT_DIRECTORY=.scratch/ticket-302-nonexistent-objects and
GIT_ALTERNATE_OBJECT_DIRECTORIES=.scratch/ticket-302-nonexistent-alternates
exported. All three exited 0; each reported 1381 passed, zero failed and four
ignored across 84 test summaries, including doctests. cleanup passed all 46 in
each. No env -u prefix was used. cargo build, cargo fmt --all --check and
cargo clippy --workspace --all-targets -- -D warnings passed. Existing seatbelt
skips remain. The two inherited git pointers redirected fixture objects into
the launching run store; Hydra's isolated readers then could not find those
fixture commits. Cleaning the fixture subprocess environment fixes that mismatch.
