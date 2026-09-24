# 123 - A `claude` run reads back the tool output Claude Code saved for it

Type: build
Status: open
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
