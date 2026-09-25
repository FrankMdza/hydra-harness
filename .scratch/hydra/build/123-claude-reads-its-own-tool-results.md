# 123 - A `claude` run reads back the tool output Claude Code saved for it

Type: build
Status: resolved
Blocked by:

## What

Map ticket 78, resolved 2026-09-24. Read its `## Answer` first.

Claude Code 2.1.281 saves any tool output above its inline limit to
`~/.claude/projects/<slug>/<session-id>/tool-results/<id>.txt` and has the model `Read` it. On
run 01a0d3e4-4245 (`claude`, `standard`) the gate refused that `Read` with `hydra/out-of-scope`
and the model lost the output. `<slug>` is the run's cwd with every character outside
`[A-Za-z0-9]` turned into `-`. The cwd is the resolved path: `ls ~/.claude/projects/ | grep
worktrees` shows delegation runs as `-private-var-folders-...`, not `-var-folders-...`. Check
the rule against that listing before trusting this sentence.

Build ticket 38 is the precedent: `own_read_paths` in `src/policy/mod.rs` near line 790 carves
out Cursor's own `cursor-data/` for reads.

## Do

1. Spend ten minutes, no more, looking for a Claude Code 2.1.281 setting or environment
   variable that moves the tool-results directory (or `projects/`) without moving the rest of
   `~/.claude`. `claude --help`, and `strings` on the binary for `tool-results` and `projects`.
   `CLAUDE_CONFIG_DIR` does not count. If one exists, stop and say so in `## Outcome` with what
   you found: the answer on map 78 prefers it, and the ticket gets rewritten around it.
2. Otherwise, `own_read_paths` returns `~/.claude/projects/<slug of the run's worktree>/` on
   `Provider::Claude`, with the slug computed by one function that has a doc comment naming the
   rule and where it was measured. Cursor's entry stays as it is.
3. The entry reaches `run_start.scopes` the way Cursor's does, so the replay reads the compiled
   path back. Read `src/sandbox.rs` near line 145, which also consumes `own_read_paths`, and
   decide whether the seatbelt profile needs the path. Say which way you went and why.
4. Update the doc comment on `own_read_paths` and the comment in `compile` near line 897
   ("Empty on every provider but `cursor-agent`"). `IMPLEMENTATION.md`: one sentence where it
   covers the Cursor carve-out. `SECURITY.md`: if it lists what a `standard` run can read outside
   its worktree, add this directory.

## Done when

- A test in `tests/gate.rs` compiles a `standard` gate for `claude` and feeds it a `Read` of
  `<that dir>/<session>/tool-results/x.txt`: allowed. A `Read` of the same file under a
  different slug, a `Read` of `~/.claude/settings.json`, and a `Write` into the carved-out
  directory: each refused, with the code each gets today. Check that the first case fails with
  `hydra/out-of-scope` with your change reverted, and paste that line into `## Outcome`.
- `tests/spawn.rs` near line 700 asserts a `claude` launch carries no `own_read_paths` entry.
  It now asserts exactly one, equal to the directory above.
- A unit test on the slug function with a `/private/var/folders/...` path and a path holding
  a `.` and a `_`.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --test gate
  --test spawn` and `cargo test --lib` pass.

## Outcome

Resolved 2026-09-24. Built by `composer-2.5` on run 01a0d56c-a2b7-7552 under `standard`, about
3.03M tokens, 2.91M of them cache reads. The session read the diff and applied it with one
change.

Step 1 found no setting that moves `projects/` or `tool-results/` without moving all of
`~/.claude`. The agent ran `claude --help` and `strings` on the 2.1.281 binary, which names
`getProjectsDir` and `tool-results` but nothing that redirects them alone. So the carve-out
went in.

`src/provider/claude.rs` gains `projects_dir_for(home, cwd)` and `projects_dir(cwd)`, and
`transcript_path` now goes through them. The slug function already existed as `project_slug`
for build 38's probe transcripts. It is public now, and its test has a `/private/var/folders/...`
case. It keeps hyphens as well as letters and digits, which yields the same slug as the ticket's
rule. `own_read_paths` returns that directory on `Provider::Claude`, and Cursor's entry is
unchanged. The seatbelt profile did not change: `src/sandbox.rs` only acts on an entry that sits
under a denied parent, and `~/.claude/projects/` does not.

A departure from map 78's answer: the directory is not in `run_start.scopes`. Cursor's
`cursor-data/` was never there either. Both reach the gate and the replay through
`own_read_paths` in the compiled config, and the replay recomputes them from the record's
worktree path. A later change to `project_slug` would therefore read old `claude` records with
the new rule. Cursor's carve-out has the same property, and no ticket has asked for it to be
recorded.

The session changed the gate test. As written, it created `session-id/tool-results/x.txt` under
the real `~/.claude/projects/` for its scratch worktree and a sibling, and the agent's three runs
of it left five directories there, `*-T-hydra-gate-*-worktrees-own*`. `gate.py` resolves paths
with `realpath`, which only needs the existing prefix, so the test now creates nothing, and a
rerun left no new directory. The human removed the five directories on 2026-09-25.

`a_claude_run_may_read_its_own_tool_results` in `tests/gate.rs`: the own `Read` is allowed; a
`Read` under another slug, a `Read` of `~/.claude/settings.json`, and a `Write` into the
carved-out directory are each refused with `hydra/out-of-scope`. With the `Claude` arm of
`own_read_paths` removed, on master:

```
Read of own tool-results: hydra/out-of-scope: policy standard allows file_read inside worktree or skill_snapshot and /users/user/.claude/projects/-private-var-folders-...-worktrees-own/sess...
```

`tests/spawn.rs` asserts that a `claude` launch carries exactly that one entry. On master: lib
210, gate 99, spawn 34, all passing, and fmt and clippy are clean. Inside the worktree five
spawn tests and three `run::tests` failed on the inherited `GIT_OBJECT_DIRECTORY`.
IMPLEMENTATION.md and SECURITY.md each gained a sentence.
