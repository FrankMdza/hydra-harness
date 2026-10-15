# 331 - The Linux job says something true: four test groups fixed on a real Linux, two decided, two skipped by name

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-15, by the session, from map 130's answer, question 1. Build 315's `linux` job ran
the whole suite minus `sandbox` on `ubuntu-latest` and came back with 51 red tests in ten test
binaries (run 38695261071). The panel sorted them into eight groups (`research/130-panel-readings.md`,
item 1, corrected in `130-synthesis.md`). The human asked for the fixes to be tested on a real
Linux, not reasoned about from the Mac, so this ticket is the session's own and is verified in a
Docker container on this machine (`rust:1-bookworm`, linux/aarch64, Python 3.11, as user `ci`, not
root: as root the permission tests of groups a and others pass for the wrong reason, measured on the
first container run). The `ubuntu-latest` job stays the x86-64 check.

## What

Fix, in this order:

- **a**, `tests/seats.rs`: the scratch root is spelled `/private/tmp` for `sun_path`'s 104-byte
  limit. On Linux `/private` does not exist and a non-root user cannot create it. Spell it
  `/private/tmp` on macOS and `/tmp` elsewhere, in one helper the file's tests share.
- **b**, `tests/corpus/mod.rs` `seatbelt_exec_or_skip`: panics when `CI` is set and `sandbox-exec`
  is missing, on any OS. The panic is for macOS runners only; on another OS the guard skips and
  says so.
- **f**, `tests/detached.rs` `a_grandchild_holding_the_slave_closes_the_record_within_the_deadline`:
  writes its `run_start` with the key `kind` where the validator reads `type`. On macOS the test
  returns before `check`; on Linux it reaches `check` and fails on its own JSON. Write the line
  with `type`, as `minimal_run_start` in `tests/pty_tee.rs` does, and let `check` run on every
  platform that reaches it.
- **h**, the differential fixtures: `crates/puppets/tests/fixtures/launch/*` carry
  `__CF_USER_TEXT_ENCODING`, a macOS-only variable, and a grader stub's error line prints the
  program as the shell names it (`/bin/cat` under dash, `cat` under zsh). Drop that key on both
  sides of the compare, the way `crates/grader/tests/record_collect.py` already does, and
  normalize the program name in the one stub line before comparing.
- **c**, `tests/git_config.rs` `an_attributes_file_under_xdg_config_home_changes_nothing_reap_commits`:
  the reap commit runs with `GIT_CONFIG_GLOBAL=/dev/null`, so its author can only come from
  `identity()`'s earlier read, and the fixture pins none. Pin `user.name` and `user.email` in the
  fixture as `committed_repo` already does. No fallback author in `commit_everything`: the human
  decided against inventing one, because `identity()` also returns nothing when the `~/.gitconfig`
  read times out and a fallback would then replace the human's name. Measured on this Mac with a
  clean `HOME`: git's own guess is `Francisco Mendoza <frank@laptop.local>` and
  the commit succeeds, so the failure is a container's and not a laptop's.
- **d**, `tests/gate.rs` `a_single_call_whose_cwd_holds_a_nul_is_answered_as_before_the_batch_entry`:
  the test pins an allow measured on Python 3.9; Python 3.11 and later raise `ValueError` from
  `os.path.realpath` on an embedded NUL and the gate answers `hydra/gate-error`. Decided: refuse on
  purpose. `evaluate` in `src/policy/gate.py` answers `hydra/bad-payload` for a `cwd` containing
  `\0` before any `os.path` call, the batch entry already does the same, and the test asserts that
  deny on both Pythons. `src/policy` is on `PROTECTED_CODE_PATHS`, which protects it from rung two
  and not from this ticket.
- **e** and **g**, skipped by test name until a Linux machine measures them: `tests/pty_tee.rs`
  `a_long_frame_survives_being_split_across_reads` (a 200,000-byte line arrives cut; unmeasured
  whether the tee or the pty line discipline), and the bit-exact CPython ports:
  `sheet::tests::lgamma_is_cpythons_and_not_the_c_librarys`, `the_ranking_arithmetic_matches_python`,
  `crates/pyjson/tests/math_matches_python.rs`, and the grader and puppets differentials whose
  numbers come from fixtures recorded on this arm64 Mac. In the container (arm64, glibc) the unit
  `lgamma` passes and the fixture-hash tests fail, so the difference is libm and not only the fused
  multiply-add. No tolerance: `near_tie_ranks_as_python_lgamma` exists because one last bit swapped
  two agents in build 207. The skip is a named list in `.github/workflows/ci.yml`, with a comment
  per name saying why, and `README.md`'s Linux sentence names the same list, so the two cannot
  drift apart.

## Do

1. The container as user `ci`, `CI=true`, `HYDRA_ROOT` set as the job sets it: the baseline list of
   red tests, kept in the Outcome.
2. a, b, f, h, c, d; after each, the affected test binary in the container.
3. The skip list in `ci.yml` and the README sentence.
4. The full job command in the container: green. The four checks on the Mac: green.

## Done means

The job command green in the container as a non-root user, the four checks green on macOS, the
README's Linux sentence naming the skipped tests and the same command the job runs, and the
Outcome holding the before and after lists. Amend `IMPLEMENTATION.md` where the Linux paragraph of
build 315 is, saying what runs on Linux and what is skipped and why.
