# 42 - A record pins the policy and not the paths it resolved to

Type: grilling
Status: open

## Question

Contract 11 derives `permission_decision` at read time by running the run's policy again,
which is what makes an adapter fix reach last month's records. Contract 5.1 keeps
`policy_sha` on every `run_start` so a reader a month later has the rules exactly as they
were.

**The rules are not only the policy files.** A policy grants `file_read` scoped to
`worktree`, `skill_store` or `corpus`, and what those three names resolve to is computed
at compile time from `Layout` and stored nowhere. So the sha pins half of what decided a
call.

**Should a record carry the paths its gate was compiled against?**

Seen for real on 2026-09-06, and it is why the question is here rather than in the fog.
Two branches ran side by side. One pointed `skill_store` at a per-run worktree of the
store; the other resolved it to `~/hydra/skills/`. Replaying the first branch's run on the
second branch turned ten in-scope reads into ten `Finding::GateLeak`, one per file the
curator touched in its own checkout. No policy file had changed, so `policy_sha` did not
move and nothing warned. Both builds were correct. The record could not tell them apart.

Build ticket 12's fix was to make the scope a function of the run id and the policy name,
which both the launch and the replay compute. That works and it is the reason
`Layout::skill_store_scope` exists. It also means every future change to where a scope
points silently rewrites history, and the next one may not be caught by a corpus-wide test
that happened to exist.

## Settle

- **Whether `run_start` gains a `scopes` object.** Three names and three resolved paths,
  beside `policy` and `policy_sha`. It is a contract change and contract 15 versions the
  vocabulary, so it costs a schema bump and a reader that handles its absence on every
  record written before it.
- **Or whether the rule stays derived and gets a guard instead.** A test that says every
  scope is a pure function of `Layout`, the run id and the policy name, so a change that
  breaks the property fails the suite rather than the corpus. Cheaper, and it holds only as
  long as someone keeps the property true.
- **What a replay should do when it cannot know.** Today it silently derives a different
  answer. A finding that says "this record's scopes are not recomputable" is a third
  option, and it is the one that keeps the corpus honest without changing the format.
- **Whether this is the same question for `worktree`.** That scope is
  `~/hydra/worktrees/<run-id>/`, which `IMPLEMENTATION.md` already flags as a location no
  closed ticket names. Moving it would do to every record what the incident above did to
  ten calls.

## What it costs to leave open

Nothing, until someone moves a scope. Then it costs a corpus that reads differently than
it did, with no field saying so, which is the failure contract 11 spends a section
promising cannot happen.

## Where the evidence is

`IMPLEMENTATION.md`, "Rough edges worth knowing", the entry beginning "A record does not
carry the paths its scopes resolved to". `src/paths.rs` and `src/policy/mod.rs` for where
the resolution happens.
