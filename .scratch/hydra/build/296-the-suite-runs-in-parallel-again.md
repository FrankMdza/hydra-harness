# 296 - The suite runs in parallel again: no test mutates the process environment

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from `~/cerebro/research/security-and-compile.md` section 2,
change 3, which the human put on Hydra's list on 2026-10-12.

## What

`ci.yml` and the PR template run `cargo test --release --workspace -- --test-threads=1`, and the
whole suite pays for it. The only reason is that a few tests call `std::env::set_var` in-process,
which races every other test in the same binary that reads the environment: `tests/git_config.rs`,
`tests/launch_phases.rs` and a unit test in `src/rungs/evolve.rs` (grep `set_var` to find the exact
sites; there may be more). The research note's answer is nextest, one process per test. The
cheaper answer, and the one Cerebro's ticket 06 already used, is that a test which needs a
different environment spawns a child with it and asserts on the child, so no test ever mutates
its own process. Then `--test-threads=1` goes and the suite runs in parallel with plain
`cargo test`.

## Do

1. For every `set_var` in a test: either pass the variable to the process the test already
   spawns (most of these tests launch `hydra` or `git`, and `Command::env` is the right place), or
   re-execute the test binary itself with `std::env::current_exe()`, a marker variable and
   `--exact <test> --nocapture`, the way a fork-style test does, and assert on the child's output.
   No `set_var` and no `remove_var` remain outside a child. `unsafe` is not an answer.
2. Drop `-- --test-threads=1` from `.github/workflows/ci.yml` and from
   `.github/PULL_REQUEST_TEMPLATE.md`. Update `CLAUDE.md` wherever it names the flag.
3. Measure: `cargo test --release --workspace` wall time with `--test-threads=1` before the change
   and without it after, both from a warm build. Run the parallel suite three times in a row.

## Done when

- `grep -rn set_var src tests crates` finds nothing outside a spawned child; the suite is green
  three times in parallel; the two wall times are in the Outcome and in one paragraph of
  `IMPLEMENTATION.md`; fmt and clippy clean over the workspace.

## Outcome

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a1340b-146a-7091-a15e-3b7ab34a5281` under
`standard`, 3.13M tokens, applied to master on 2026-10-13 after the diff was read whole. The two
test files re-execute the named test in a child of the test binary with `Command::env`, and the
child asserts the test ran by name so a misspelled filter cannot pass silently; `evolve.rs`
already used a child. `--test-threads=1` is gone from `ci.yml` and the PR template. The builder
measured, warm, in its worktree: 318 s serial before, 198 s parallel after, and the parallel
suite green three times (198, 208, 198 s). On master the session ran the parallel release suite
twice with a panel seat on the machine: 284 s then 230 s, green both times, no `set_var` left
outside a child. Both the builder and the session ran it with `env -u GIT_OBJECT_DIRECTORY -u
GIT_ALTERNATE_OBJECT_DIRECTORIES` inside the run, because the allowlist hands every run those
two and three tests in `src/run.rs` read them; on a plain shell on master they are not set and
the tests pass as they are. That inherited pointer is now a known trap for any ticket whose
suite must be green inside a run, and 295's Outcome says the same.
