# 362 - The `git push` floor reads `remote.<name>.push` and `push.default` from repository config

Status: claimed

## Objective

Close the `git push` floor gap found by the adversarial review of `2e99a25` (`01a17dfa-7a1a-7c01-a9a4-f251f124913b`). Build 357 checked `-c remote.<name>.push=<refspec>` and `-c push.default=matching` passed on the command line, and `bare_push_violation` read `HEAD` and `[branch "<name>"] merge` from `.git/config`. However, a run can write `[remote "origin"] push = HEAD:refs/heads/master` or `[push] default = matching` into the repository config and then run `git push` or `git push origin` with no explicit refspec on the command line.

## Problem

In `src/policy/gate.py`:
- `bare_push_violation(git_dir)` (lines 1038 to 1060) reads `HEAD` and `upstream_merge(git_dir, name)` from the shared git config (`commondir/config`), plus the worktree config if present, only for `[branch "<name>"] merge`.
- It never inspects `[push] default = matching` or `[remote "<name>"] push = <refspec>` in the repository config files (`os.path.join(shared, "config")` and `os.path.join(git_dir, "config")`).
- Consequently, `git push` or `git push origin` with no refspec in `argv` walks past the floor when `remote.origin.push` maps a topic branch to `master` or `main`, or when `push.default` is `matching` in `.git/config`.

## Cases

1. `git push origin` in a repository whose `.git/config` has `[remote "origin"]\n\tpush = topic:refs/heads/master` is refused with `hydra/pattern-floor`.
2. `git push` (no remote named on the command line) in a repository whose `.git/config` has `[remote "origin"]\n\tpush = HEAD:main` is refused with `hydra/pattern-floor`.
3. `git push` in a repository whose `.git/config` has `[push]\n\tdefault = matching` is refused with `hydra/pattern-floor`.
4. `git push origin` in a repository on branch `hydra/test` whose `.git/config` has `[remote "origin"]\n\tpush = hydra/test:refs/heads/feature` is allowed under `standard`.

## Do

1. In `tests/gate.rs`, add a test `git_push_floor_reads_remote_push_and_push_default_from_repo_config` covering all four cases above by writing `.git/HEAD` and `.git/config` in a temporary directory and running the compiled gate under `standard`.
2. In `src/policy/gate.py`:
   - Add a helper `repo_config_texts(git_dir)` that reads `os.path.join(git_dir, "config")` and, when `commondir` is present, `os.path.join(shared, "config")`.
   - Add a helper `config_push_violation(git_dir, remote_name=None)` (or extend `bare_push_violation(git_dir, remote_name=None)`) that scans `repo_config_texts(git_dir)` section by section:
     - In `[push]`: if `default` is `"matching"` (case-insensitive), return `"git push with push.default=matching in config, which pushes every shared branch"`.
     - In `[remote "<name>"]`: if `remote_name is None` or `<name> == remote_name`, for every `push = <value>` line, check `push_refspec_violation(value.strip(), [git_dir])` and return its reason if non-None.
   - Call this check in `check_git` when `sub == "push"` and `len(positional) < 2`, passing `positional[0] if len(positional) == 1 else None`.
3. Update `SECURITY.md` lines 362 to 363 and `IMPLEMENTATION.md` around the build 357 note so neither file claims `push.default=matching` or `remote.<name>.push` in repository config walks past the floor.
4. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, and `cargo nextest run --release -p hydra --test gate`.

## Outcome
