# 78 - Claude Code writes a big tool output under `~/.claude/projects/` and reads it back; `standard` refuses the read

Type: grilling
Status: open
Blocked by:
Opened: 2026-09-24, by the session, from the corpus walk failing on run 01a0d3e4-4245 (build ticket 116 holds the normalizer half).

## Question

Claude Code 2.1.281 no longer returns a large tool output inline. It writes the output to
`~/.claude/projects/<cwd-slug>/<session-id>/tool-results/<id>.txt`, tells the model the output
was saved there, and the model's next call is a `Read` of that path. On run 01a0d3e4-4245 (Opus
under `standard`, the fourth hiring panel's `claude` seat) the run's `cwd` slug was
`-Users-USER-hydra-worktrees-01a0d3e4-4245-...`, and the gate refused the `Read` with
`hydra/out-of-scope`, correctly: the path is under `~/.claude`, outside the worktree and the
skill snapshot. The model lost the output of that command and carried on without it.

So every `standard` run on 2.1.281 loses any tool output above the inline limit. That is a
functional loss the policy did not intend, and it will grow as the limit or the feature changes.

The precedent is build ticket 38: `cursor-agent` answers `awaitToolCall` by reading under the
run's own `spawn/<id>/cursor-data/`, and `own_read_paths` in `src/policy/mod.rs:785` carves that
directory out for reads, per provider, because it is the run's own data. The difference here is
where the directory lives: under `~/.claude`, which `CLAUDE.md` names as refused by design and
which holds the user's skills, settings and other sessions' transcripts.

Options the human should choose between:

1. Carve out `~/.claude/projects/<slug of the run's worktree>/` for reads on `claude`, the way
   build 38 did for Cursor. Narrow, but it is a directory under `~/.claude` whose name Hydra
   computes from the cwd, and the slug rule is Claude Code's, not ours; a change to the rule
   silently reopens the loss.
2. Point Claude Code's project directory somewhere Hydra owns, if a setting or environment
   variable moves it (`CLAUDE_CONFIG_DIR` moves all of `~/.claude`, which would also move the
   skills the run is meant to reach). Needs a probe on 2.1.281.
3. Accept the loss under `standard`, document it in `SECURITY.md`'s known exposures as a scope
   limit, and tell runs in the preamble to page long output through files in the worktree.

The session's recommendation is 1, with the slug computed by Hydra at launch and written into
`run_start.scopes` so the replay reads it back, plus a test that the carve-out is read-only and
covers no other slug. Option 2 first if the probe is cheap.

## Answer

(pending, needs the human)
