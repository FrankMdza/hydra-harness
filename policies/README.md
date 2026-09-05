# The delegation policy set

Protected config #1, harness spec 15.1. `policy_sha` in every run record is the sha of
this directory, the floor included, so an edit here is visible in every record written
after it.

Five policies, harness spec 7.2, plus the pattern floor of 7.4. Deny by default: a
policy is an allowlist and there is no `denied` list, because a policy where two rules
can disagree is a bug with a schema.

`allow` entries name a capability and, where the capability takes a path, a scope.
`file_read` and `file_write` are the two scoped capabilities. `shell`, `network_read`,
`mcp_call`, `subagent_spawn` and `provider_native` take no scope.

The scopes are `worktree` (the run's own checkout), `skill_store` (`~/hydra/skills/`),
`corpus` (`~/hydra/runs/`) and `anywhere`.

A sixth policy should arrive with a reason. Adding one means a file here, a row in
`Policy` in `src/policy/mod.rs`, and a place in the partial order of 7.3.
