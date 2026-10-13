# 296 - The suite runs in parallel again: no test mutates the process environment

Type: build
Status: claimed
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

