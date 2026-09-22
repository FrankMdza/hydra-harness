# The delegation policy set

Protected config #1, harness spec 15.1. `policy_sha` in every run record is the sha of
this directory, the floor included, so an edit here is visible in every record written
after it.

Five policies, harness spec 7.2, plus the pattern floor of 7.4. Deny by default: a
policy is an allowlist and there is no `denied` list, because a policy where two rules
can disagree is a bug with a schema.

`allow` entries name a capability and, where the capability takes a path, a scope.
`file_read` and `file_write` are the two scoped capabilities. `shell`, `network_read`,
`mcp_call`, `subagent_spawn`, `skill_invoke` and `provider_native` take no scope.
`opaque_shell`, build ticket 59, is the capability no policy grants: a row that writes to
the stdin of a command already running carries nothing the gate can read.

The scopes are `worktree` (the run's own checkout), `skill_store` (`~/hydra/skills/`),
`skill_checkout` (`~/hydra/skill-worktrees/<run-id>/`, a `curate` pass's own git worktree
of the store), `skill_snapshot` (`~/hydra/skills-snapshots/<sha>/`, the checkout this
run's links point into), `corpus` (`~/hydra/runs/`) and `anywhere`.

`skill_checkout` arrived with map ticket 40 on 2026-09-06, and `skill_store` went back to
meaning the store on every spawn. One name was resolving to two directories, which is the
ticket. `curate` reads four scopes, the corpus, the store, its checkout and its snapshot,
and writes one, its checkout. No policy writes `skill_store`: harness spec 7.5 denies the
store's working tree under all five, and its only writers are `Store::pin` and a human.

`skill_invoke` and `skill_snapshot` arrived with map ticket 39 on 2026-09-06. A run that
is linked a skill and cannot invoke it was given a listing entry rather than a skill, so
every policy that reads at all grants both. `judge` reads nothing and gets neither.
`skill_snapshot` never appears on `file_write`: a run that could edit its snapshot could
edit its own instructions mid-session.

The capability table, one row per vendor tool, is `docs/harness-spec.md` section 7.2 and
`src/policy/table.rs`.

A sixth policy should arrive with a reason. Adding one means a file here, a row in
`Policy` in `src/policy/mod.rs`, and a place in the partial order of 7.3.
