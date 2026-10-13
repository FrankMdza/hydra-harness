# 302 - The suite is green inside a run: three tests stop reading the launcher's git pointers

Type: build
Status: claimed
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
