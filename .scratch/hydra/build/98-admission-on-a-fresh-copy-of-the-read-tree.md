# 98 - Rung two admits a fresh copy of the tree it read, with a toolchain the proposer cannot change

Type: build
Status: resolved
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 2 and 12, findings H2, H38 and H16.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/rungs/evolve.rs and src/env.rs.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

Admission runs on a copy of the read tree made after the proposer's processes are stopped, with a scratch cargo home and absolute tool paths; a test shows that a change the proposer makes to its checkout or to the cargo configuration it can reach after the read does not change what admission runs. The canary test no longer mutates the process environment under parallel tests. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome

Built by a Claude Code session working on Hydra's source, not through `hydra run`.

**What was built.** In `src/rungs/evolve.rs`:

- `admission_toolchain()` runs `rustc --print sysroot` once, in `evolve()` before the proposer
  launches, and keeps the canonical `<sysroot>/bin`. Admission calls `<sysroot>/bin/cargo` by that
  path. On this machine it is `/opt/homebrew/Cellar/rust/1.94.0/bin`, and `cargo-clippy`,
  `cargo-fmt`, `rustfmt` and `clippy-driver` are in the same directory.
- `prepare_admission()` writes a fresh copy of the read tree to
  `spawn/<pass-id>/admission/tree`, from `patch.tree` in the object store, after the proposer's
  run has ended. `materialize_tree()` lists the tree with `git ls-tree -r -z` and writes each blob
  from `git cat-file --batch` byte for byte, with modes 644 and 755. It does not use
  `checkout-index`, which applies filters and line-ending attributes, or `git archive`, which drops
  `export-ignore` paths. A submodule, or a symbolic link that is absolute or contains `..`, blocks
  the patch with the reason. The same call creates an empty `admission/cargo-home` and, on macOS,
  `admission/admission.sb`.
- `admission.sb` is the proposer's profile with four kinds of line appended, in order. Reads and
  writes are allowed under `admission/`. Metadata is allowed on each ancestor of that directory and
  on the ancestor's direct children. Reads of `.cargo` are denied in every ancestor of the copy.
  SBPL takes the last matching rule, so the `.cargo` denial wins over the metadata allow.
- `admission_gate()` takes an `Admission` and runs the four steps in the copy, under
  `admission.sb`. `evolve()` discards the copy and the cargo home once the steps finish.
  `admission_sandbox` on `run_end` is now the sha of `admission.sb`, not the proposer's profile.

In `src/env.rs`, `CARGO_HOME` left `ADMISSION_ENV_ALLOWLIST`. `admission_env(toolchain,
cargo_home)` sets `CARGO_HOME` to the scratch directory and puts the toolchain first on `PATH`.

H16: the canary no longer calls `set_var`. `run_admission_step_clears_the_parent_environment`
runs this test binary again as a child, sets the canary on the child's `Command` only, and reads
what `admission_env_probe` prints. The probe returns at once unless the canary is set, so in a
normal run it does nothing.

**Why a copy under `spawn/` and not a stopped proposer.** The ticket assumed Hydra can stop the
proposer's processes. That assumption was false. Nothing in `src/` kills or even lists the processes
a run leaves behind. A process started with `setsid` or `nohup` has left the pty's session, so
there is no handle to kill it by. The copy sits where the proposer's profile denies reads and
writes, and a leftover process keeps that profile, because a seatbelt is inherited and cannot be
dropped. Stopping those processes turned out not to be needed. If the human wants them stopped
anyway, that needs a mechanism no ticket has specified. On Linux there is no seatbelt, so neither
the copy nor the scratch cargo home is out of a leftover process's reach. Admission there already
runs bare.

**What the ticket assumed that was false, second part.** Cargo does not only read
`$CARGO_HOME/config.toml`. It also reads `.cargo/config.toml` in every ancestor of its working
directory, and `~/.cargo` is an ancestor of `~/hydra/spawn/...`. A scratch `CARGO_HOME` alone would
have left H2's `runner` forge working. Measured in a scratchpad with a failing test and `runner =
"/usr/bin/true"` in an ancestor's `.cargo/config.toml`. Bare, `cargo test` exited 0 with "test
result: ok". Under a profile that denies reads of that `.cargo`, it exited 101. Cargo skips a
config file it cannot stat.

**A second measured surprise.** The first version of the profile opened only `admission/`.
`cargo fmt --check` then failed with "Operation not permitted", and so did rustc's dep-info step.
Both stat a name in every ancestor, and `spawn/` and `spawn/<pass-id>/` are denied to reads by the
proposer's profile. The metadata allows fixed it. Under the admission profile a sibling run's
`spawn/<id>/launch.json` still refuses `cat` and `stat`, and `ls spawn/` is still refused. What is
now visible is the existence and timestamps of a direct child of `spawn/`, by name.

**Tests, each checked by reverting the fix it guards:**

- `tests/evolve.rs::admission_runs_on_a_copy_of_the_read_tree_that_nothing_after_the_read_reaches`.
  The read tree holds a failing test. After the read, the test makes the checkout's test pass and
  plants `[target.'cfg(all())'] runner = "/usr/bin/true"` in `.cargo/config.toml` at the fixture's
  layout root, the place `~/.cargo` takes on this machine. It first runs the old shape, the steps
  in the checkout under the proposer's profile, and asserts that admission passes. Then it
  prepares the copy and asserts four things: the copy holds the read tree's bytes, the proposer's
  profile cannot write it, the admission profile cannot read or stat a sibling's spawn file, and
  admission fails with the read tree's own panic. Reverting the copy to point at the checkout made
  it fail (the copy held the checkout's bytes). Reverting the `.cargo` denials made it fail
  (admission passed).
- `tests/evolve.rs::admission_refuses_to_copy_a_link_that_leaves_the_tree`. Links to `/etc/hosts`
  and `../README` are refused, and the refused copy is discarded. Reverting the link check made it
  fail.
- `rungs::evolve::admission_step_tests::run_admission_step_clears_the_parent_environment`, now
  with the canary on a child. It also asserts the scratch `CARGO_HOME` and the toolchain first on
  `PATH`. Removing `env_clear` failed it on the canary, forwarding the human's `CARGO_HOME` failed
  it on the cargo home, and dropping the `PATH` prefix failed it on `PATH`.
- `tests/evolve.rs::admission_runs_under_the_proposers_seatbelt_and_refuses_a_hostile_test` now
  builds an `Admission` directly and still passes.

**A real admission of this repository, measured and not committed.** I used a throwaway ignored
test with a scratch `Layout`, the working tree written with `git write-tree`, and a `standard`
proposer profile from `write_spawn_config`. It ran `prepare_admission` and `admission_gate`. The
copy took 0.1 to 0.3 s. The four steps passed in 416 s with a fresh cargo home, which downloads the
123 crates in `Cargo.lock` on every pass. Admission now needs the network for that; the profile
leaves it open. **Two tests of this suite fail under any seatbelt, so every rung-two pass on macOS
is blocked at `cargo test` today, before and after this ticket.** The measurement skipped both
inside the copy only:

- `procinfo::tests::our_own_start_time_matches_ps` panics with EPERM. `sandbox-exec -p
  '(version 1)(allow default)' /bin/ps` is refused on this Mac because `/bin/ps` is setuid.
- `tests/delegation.rs::a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded`
  gets the inside-a-seatbelt `HYDRA_ROOT` refusal instead of the refusal it expects.

Both come from build ticket 75 putting admission under the seatbelt, not from this ticket. The same
measurement in the old shape, the checkout under the proposer's profile, passed in 418 s with the
same two skips. They need a build ticket of their own: skip both inside a seatbelt, the way
`tests/evolve.rs` already does for nested `sandbox-exec`. One run of the new shape also failed
`tests/export.rs::an_unknown_opaque_token_exports_and_is_named_in_the_summary_without_its_value`.
The entropy check flagged a 99-character file path and a 63-character output instead of the one
64-character token. The next run passed, and so did every local run, so it looks like a flake that
depends on the path. I did not chase it.

**What is still open, and why each one is a decision and not this ticket's code.**

- The toolchain is resolved before the proposer runs and called by absolute path. But
  `/opt/homebrew/Cellar/rust/1.94.0/` is owned by `frank`, and a `standard` run can overwrite
  `cargo` or `rustc` there. The panel's H2 row already says binding `PATH` directories for runs
  needs a decision, and the toolchain directory falls under the same one. The ticket's title
  promises "a toolchain the proposer cannot change", and that is only half true: an earlier `PATH`
  directory can no longer shadow it, but its own files can still be rewritten.
- Everything else admission runs by name, such as `cc`, `git` and `python3`, still comes from the
  orchestrator's `PATH` after the toolchain. That falls under the same `PATH` decision.
- The copy is written from the shared object store. Build ticket 103 and map ticket 74 cover that
  store.
- `HOME` still reaches admission, so `~/.cargo/credentials.toml` stays readable. That is the
  home-readable caveat from ticket 90.

Suite, outside any seatbelt, with `~/.local/bin` off `PATH` so `tests/drift.rs` skips rather than
launching `claude`: 735 passed, 0 failed, across 27 test targets with doc tests included. On a `PATH` without `~/.pixi/bin`,
`tests/gate.rs::a_layout_root_rust_and_python_used_to_fold_differently_still_denies_the_corpus`
fails because a different `python3` reads the gate as non-UTF-8. That comes from the environment
and is not new. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean.

**Proposed sentences for build ticket 102.**

For `IMPLEMENTATION.md`, rung-two section, after the paragraph that ends "Since build ticket 90":

> **Since build ticket 98 the four steps run on a copy of the read tree, not in the proposer's
> checkout.** `prepare_admission` writes `patch.tree` byte for byte from the object store to
> `spawn/<pass-id>/admission/tree` after the proposer's run ends, with `git ls-tree` and `git
> cat-file --batch`. No filter, attribute or export rule applies. A submodule, or a symbolic link
> that is absolute or climbs with `..`, blocks the patch. The proposer's profile denies that
> directory to reads and writes, and a process the proposer left running keeps its profile, so
> nothing done after the read reaches what admission compiles. The steps run under
> `admission/admission.sb`, which is the proposer's profile plus three things: `admission/` opened,
> metadata of each ancestor and its direct children opened, and every `.cargo` above the copy
> denied to reads. Cargo reads a `config.toml` from each ancestor of its working directory as well
> as from `CARGO_HOME`, and `~/.cargo` is one of those ancestors. `CARGO_HOME` is an empty
> `admission/cargo-home`, so every pass downloads the crates in `Cargo.lock`. `cargo` is called as
> `<sysroot>/bin/cargo`, with the sysroot read from `rustc --print sysroot` before the proposer
> launches, and that directory goes first on `PATH`. `admission_sandbox` is the sha of
> `admission.sb`. The copy and the cargo home are removed after the steps finish. What stays open:
> the toolchain's own files are writable by the user when Homebrew installed them, and `cc`, `git`
> and `python3` still resolve through the orchestrator's `PATH`. Both are the `PATH` decision the
> third panel's H2 names. On Linux no seatbelt stands between a leftover proposer process and the
> copy.

For `SECURITY.md`, where the admission gate is described (lines 40 to 50):

> Admission runs on a copy of the tree the judges read. The proposer's seatbelt cannot write that
> copy, and neither can any process the proposer left running. Cargo gets an empty cargo home, and
> the admission profile denies reads of every `.cargo` directory above the copy, so a `runner` or
> `rustc-wrapper` the proposer wrote into `~/.cargo/config.toml` is not read. `cargo` is the
> toolchain's own binary, found before the proposer ran, and is called by absolute path. What is not
> closed: on this machine the toolchain lives in a Homebrew directory the user owns, and a
> `standard` run can rewrite it.

For `IMPLEMENTATION.md`'s rough edges, or the ticket that fixes the two tests:

> Admission runs the whole suite inside a seatbelt, and two tests fail there on any profile:
> `procinfo::tests::our_own_start_time_matches_ps`, because the seatbelt refuses to exec setuid
> `/bin/ps`, and `a_run_relaunching_the_binary_without_a_parent_is_refused_and_recorded`, because
> the binary refuses `HYDRA_ROOT` inside a seatbelt. Until both skip inside one, every rung-two pass
> on macOS is blocked at `cargo test`.
