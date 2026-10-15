# 331 - The Linux job says something true: four test groups fixed on a real Linux, two decided, two skipped by name

Type: build
Status: resolved
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

## Outcome

Resolved on 2026-10-15 by the session, verified in a Docker container on this machine:
`rust:1-bookworm` (rustc 1.99.0, glibc, linux/aarch64, Python 3.11.2, `sqlite3` and `procps`
installed as the runner has them), with an init process, the repository at
`/home/ci/hydra-harness` as user `ci`, `LANG=C.UTF-8`, `CI=true`, `HYDRA_ROOT` set as the job
sets it. The `ubuntu-latest` job on the next push is the x86-64 check.

### Measured

- Baseline, the job's command as it stood: 53 red tests. 50 of the CI's 51 (the one missing is
  `lgamma_is_cpythons_and_not_the_c_librarys`, which passes on arm64 because `c_mul_add` takes
  the fused branch there), plus three harvest tests and one normalize test that spawn `sqlite3`,
  absent from the image until installed. Two wrong setups came first and are worth knowing: as
  root the permission tests pass for the wrong reason (37 red, a different set), and a checkout at
  `/work` makes the gate's raw pass refuse every path under `worktrees/` because `/work` is a
  substring of it (674 red).
- After this ticket, the job's command with the nine skips: `passed 1401 failed 0 ignored 4
  filtered 14`, exit 0. On the Mac, `cargo nextest run --release --workspace` 1427 passed, 4
  skipped; fmt, clippy and the doctests green.

### What changed

- a: `tests/seats.rs` spells the scratch root `/private/tmp` on macOS and `/tmp` elsewhere.
- b: `tests/corpus/mod.rs` panics on a missing `sandbox-exec` under `CI` only on macOS.
- c: `tests/git_config.rs` pins `user.name` and `user.email` in the fixture repository, where
  `identity()` reads them. No fallback author in `commit_everything`. Measured on this Mac with a
  clean `HOME`, `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`: git's own guess is
  `Francisco Mendoza <frank@laptop.local>` and the commit succeeds, so a laptop
  without a git identity reaps; a container does not.
- d: `src/policy/gate.py` refuses a `cwd` holding a NUL as `hydra/bad-payload` before any
  `os.path` call; the test is renamed `a_single_call_whose_cwd_holds_a_nul_is_refused_as_a_bad_payload`
  and asserts the deny and that the reason names the NUL.
- f: `tests/detached.rs` writes a whole `RunStart` (`held_slave_run_start`) instead of a
  hand-written object with `kind` for `type` and no version fields.
- h: `crates/puppets/tests/differential.rs` drops `__CF_USER_TEXT_ENCODING=`, `LC_CTYPE=C.UTF-8`
  and `_=` lines from `.env` captures on both sides before comparing; the last because `env |
  sort` places `_` first under macOS's `en_US.UTF-8` collation and last under glibc's C fallback.
  `crates/grader/tests/differential/collect.rs` folds GNU `cat`'s `/bin/cat:` into `cat:`.
- The job's command in `.github/workflows/ci.yml` skips nine tests by name with a comment each;
  `README.md`'s Linux sentence and the Linux paragraph of `IMPLEMENTATION.md` name the same list.

### What the ticket assumed that turned out false

- Group f was not one bug but two. With `type` fixed, the test's Linux path ran for the first time
  and failed at its last assertion: `pgrep -f "sleep 300"` finds nothing, so the orphan holding the
  slave died when the master closed. On Linux closing a pty master sends SIGHUP to the slave's
  session, and `sleep` dies of it; macOS never reached this path because it closes the master when
  the leader exits. The test asserts the orphan must survive ("Hydra killed the grandchild still
  holding the slave"). Whether Hydra wants an orphan to outlive a run on Linux is a decision, not a
  fix, so the test joins the skip list with that sentence and the question goes to map 119's
  follow-up (build 318) rather than being answered here.
- Group g on arm64 glibc: the unit `lgamma` passes, the fixture-hash tests fail. The difference the
  CI saw on x86-64 is fused multiply-add plus libm; on arm64 it is libm alone.
- `crates/grader/tests/cutover.rs` `a_marked_show_is_read_bare` went red once in the full
  container run (`result_text: Null`) and passed three of three alone and in the final run. Flaky
  under load; not touched, noted here.
- The job's own comment said `--skip sandbox` drops the `tests/spawn.rs` profile tests. It did
  not: those names carry no `sandbox`, and only the guard's panic made them red. The guard change
  is what skips them now.
