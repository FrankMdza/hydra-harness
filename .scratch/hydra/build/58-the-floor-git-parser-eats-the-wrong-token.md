# 58 - The floor's `git` parser consumes the wrong token for one flag

Type: build
Status: resolved
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

## Outcome

Resolved 2026-09-21. The human ran the two-entry edit on `policies/floor.json` from `~/hydra`,
which added `-c` and `--config-env` to `consume_before_subcommand`, and the orchestrating
session took the `#[ignore]` off `a_config_override_does_not_hide_the_subcommand_from_the_floor`
in `tests/gate.rs`. The test had been written on 2026-09-15 against the unedited floor and
failed then, four commands allowed that the push rules should refuse; on the edited floor it
passes, and `tests/gate.rs` is 72 of 72 with nothing ignored.

The edit moves `policy_sha`. Every record written before 2026-09-21 carries the old sha, which
is the case build ticket 49 built `policy_moved` for: a replay under today's set files its
disagreement with an old record as a limit that names the sha rather than as a leak. The
corpus walks were rerun against the new sha before this landed.
