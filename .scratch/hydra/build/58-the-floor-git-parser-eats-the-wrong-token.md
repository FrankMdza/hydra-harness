# 58 - The floor's `git` parser consumes the wrong token for one flag

Type: build
Status: open
Blocked by:

## What

`git_subcommand` in `src/policy/gate.py` consumes the value after `-C`, `--git-dir`,
`--work-tree`, `--namespace` and `--exec-path`, from `consume_before_subcommand` in
`policies/floor.json`. `git -c <key>=<value>` also takes a separate value and is not on that
list, so the value is read as the subcommand and the push rules never run. Verified on the real
compiled gate: `git push --force origin main` is refused by the floor and
`git -c core.pager=cat push --force origin main` is allowed.

`--config-env` has the same shape.

**The fix is two entries in `policies/floor.json`, which is protected config #1.** A human
makes that change; an agent does not. The test belongs in `tests/gate.rs` beside the existing
floor cases, and it can be written first.

Found by the adversarial review.
