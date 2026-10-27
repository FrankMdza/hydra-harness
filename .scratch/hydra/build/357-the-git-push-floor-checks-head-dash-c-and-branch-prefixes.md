# Build 357 - The `git push` floor checks `HEAD`, `-C`, `-c` refspecs, and `refs/heads/` prefixes

Status: resolved
Size: haiku

## Goal

Build 349 taught `check_git` in `src/policy/gate.py` to check a bare `git push` against the branch `HEAD` names and its upstream `merge` target. Four bypasses remain around `git push` in `src/policy/gate.py`:

1. **`refs/heads/<prefix>` on protected branch prefixes:**
   In `check_git` around lines 1075 to 1081, `branch = token.split(":")[-1]` turns `refs/heads/release/2026-09` into `2026-09` via `branch.rsplit("/", 1)[-1]`, and line 1080 checks `token.split(":")[-1].startswith(prefix)` against the unstripped `refs/heads/release/2026-09`. As a result, `git push origin topic:refs/heads/release/2026-09` slips past `protected_branch_prefixes`.
   Strip `refs/heads/` first when `branch.startswith("refs/heads/")`, or fall back to `branch.rsplit("/", 1)[-1]` for other `refs/` spellings, and check `protected_branch(branch)` so both `protected_branches` and `protected_branch_prefixes` see `release/2026-09` and `main`.

2. **`git push origin HEAD` and `git push origin @`:**
   When a refspec target is `HEAD` or `@`, such as `git push origin HEAD`, `git push origin @`, or `git push origin topic:HEAD`, `positional` has length 2 so `len(positional) < 2` does not fire, and `branch` is `HEAD` or `@` rather than the branch `HEAD` points to. When `branch` is `HEAD` or `@`, check `bare_push_violation` for the checkout so pushing `HEAD` or `@` from a checkout of `master` or from a branch whose upstream is `main` is refused just like a bare `git push`.

3. **`-C`, `--git-dir`, and `--work-tree` before `push`:**
   `git_subcommand` consumes `-C`, `--git-dir`, and `--work-tree` before `push`, but `check_git` still calls `bare_push_violation(cwd)` with the unmodified `cwd`. Track any `-C <dir>`, `-C=<dir>`, `--work-tree <dir>`, `--work-tree=<dir>`, `--git-dir <dir>`, and `--git-dir=<dir>` options consumed before `push` so `bare_push_violation` inspects the target checkout or git directory that `git` will actually use.

4. **`-c` and `--config-env` push overrides before `push`:**
   `git_subcommand` consumes `-c` and `--config-env` before `push`.
   - If `--config-env` or `--config-env=...` precedes `push`, refuse it under the floor because the gate does not read the environment variable it names.
   - If `-c <key>=<value>` precedes `push`:
     - When `key` lowercased is `push.default` and `value` lowercased is `matching`, refuse it under the floor. Keep `push.default=current`, `push.default=simple`, and `push.default=upstream` working as today: `bare_push_violation` already checks both the current branch and its upstream, and `tests/gate.rs` asserts that `git -c push.default=current push` is refused on `master` and allowed on `hydra/own`.
     - When `key` lowercased starts with `remote.` and ends with `.push`, treat `value` as a refspec and run the same refspec checks on it: forced `+`, `HEAD` or `@` via `bare_push_violation`, and `protected_branch`.

## What to change

1. **`src/policy/gate.py`:**
   Update the git option parsing and `check_git` for `sub == "push"` to close all four residuals above without changing non-push git checks or existing allowed pushes.

2. **`tests/gate.rs`:**
   Add unit tests covering all four cases:
   - `git push origin topic:refs/heads/release/2026-09` and `git push origin refs/heads/main` are refused with `hydra/floor`.
   - `git push origin HEAD` and `git push origin @` from a checkout of `master` or a branch whose upstream is `main` are refused with `hydra/floor`, and allowed from `hydra/own`.
   - `git -C <other_checkout> push` and `git --git-dir=<other_gitdir> push` where the target checkout is on `master` are refused with `hydra/floor`, even when the run's own worktree is on `hydra/own`.
   - `git -c remote.origin.push=HEAD:refs/heads/master push`, `git -c remote.origin.push=+topic:topic push`, `git -c push.default=matching push`, and `git --config-env=remote.origin.push=V push` are refused with `hydra/floor`, while `git -c push.default=current push` from `hydra/own` remains allowed.

## Done when

- `cargo fmt --all --check` passes.
- `cargo clippy --release --workspace --all-targets -- -D warnings` passes.
- `cargo nextest run --release -E 'binary(gate)'` passes, including the new tests and all existing bare-push tests.

## Outcome

Built in `src/policy/gate.py` and `tests/gate.rs`. All four residuals close.

- `refs/heads/` is stripped before the protected lists are read. `topic:refs/heads/release/2026-09`
  and `refs/heads/main` are refused. Other `refs/` spellings keep their last component, as before.
- A target of `HEAD` or `@` gets the bare-push check, from the checkout the push reads.
- `-C`, `-C=`, `--git-dir` and `--git-dir=` decide which checkout is read. `--work-tree` adds its
  checkout to the ones read. The gate parses options into `(flag, value)` pairs in `git_options`,
  which replaces `git_subcommand`.
- `--config-env` before `push` is refused. `-c push.default=matching` is refused. A
  `-c remote.<name>.push=` value gets the refspec checks, and so does any refspec it names.

Measured: the eight push tests in `binary(gate)` pass, the full `binary(gate)` run passes 149 of
149, and `binary(spawn)`, `binary(evolve)`, `binary(normalize)`, `binary(record_contract)` and
`binary(doctor)` pass 217 of 217. `cargo fmt --all --check` and
`cargo clippy --release --workspace --all-targets -- -D warnings` are clean.

Assumptions that turned out false or changed:

- The ticket says `-C=<dir>` is a form git reads. Git does not take `-C=`. The gate still
  reads it, as the old parser did, so the check is conservative.
- `refs/heads/feature/main` was refused before, because the old code kept only the last
  component. It is now judged as `feature/main`, which is allowed, like the unprefixed spelling.
- The ticket asks for `--work-tree` to be tracked. Git finds `.git` from the cwd, not from the
  work tree, so the work-tree checkout is an extra check and can refuse a push git would allow.
