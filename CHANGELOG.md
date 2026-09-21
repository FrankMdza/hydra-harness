# Changelog

Dates are when the change landed on `master`. Run ids are the evidence.

## Unreleased

- `hydra evolve --task` gives the proposer a target; the first directed pass produced the
  first patch a panel reviewed as a patch (2026-09-09).
- `hydra show` and `hydra reap` take a run-id prefix (2026-09-10).
- Admission refuses patches to the code rows harness spec 15.2 names, not only `policies/`
  and `config/` (2026-09-12, build ticket 40).
- A curate pass no longer tries to delete the proposal branch it just made (2026-09-12,
  build ticket 43).
- A run can no longer rewrite the hook or MCP config Hydra wrote inside its worktree, on any
  provider, under any policy; the skill roots it links are denied to writes and readable by
  shell (2026-09-12, build ticket 44).
- Several runs launching on one skill-store sha no longer race to create the snapshot
  (2026-09-12, build ticket 47).
- The rate-limit abort reads the field the vendor writes and fires (build ticket 41); a curate
  run's record says `command: curate` (build ticket 42).
- A `Write` Claude Code refuses before reading the file is no longer read as ungated
  (2026-09-12).
- The replay no longer calls an `agy` command a gate leak when the stream cut its argument at
  513 characters; it records a limit and the hook's own allow (2026-09-12, build ticket 45).
- Two parallel calls to one tool on `claude` each get their own hook answer; the first used to
  read as ungated (2026-09-12, build ticket 46).
- README rewritten around the measurement Hydra exists for; `llms.txt` added; demo GIF
  recorded from a real `agy` run (2026-09-12).
- `codex` is the fourth provider, under `standard` and `permissive`: the gate rides a `-c`
  override naming a hook file in the spawn directory, because Codex reads no project hook
  file inside a git worktree (2026-09-21, build ticket 48, run 01a0c231-6583).
- A proposer rate-limited by its last frame ends an `evolve` pass `completed` on a blocked
  branch, the way one rate-limited by outcome already did (2026-09-20).
- The floor's `git` parser consumes the value after `-c` and `--config-env`, so `git -c
  core.pager=cat push --force` is refused the way `git push --force` was (2026-09-21, build
  ticket 58, a change to protected config #1 made by the human).
- `codex` sits on the `evolve` panel, four judges now; `read-only` stays refused on it
  (2026-09-21, build ticket 61, map ticket 63).
- The self-improvement module is `src/rungs/` and its spec `docs/rungs-spec.md`; new passes
  write `rungs/<pass-id>` branches and the old `ouroboros/` ones still resolve (2026-09-21,
  build ticket 62).
- The tenth `evolve` pass is the first the panel offered: four judges, `codex` among them,
  all pass on a two-word patch, on `rungs/01a0c4b4-81ec`; the human merged it as 24cade6, the
  first change to land through rung two (2026-09-21).

## 0.2.0, 2026-09-08

All eleven commands of harness spec 17. Three routed providers, five policies, the run record
contract, the skill store with per-run snapshots, the MCP delegate face, the Herdr bridge,
both Rungs rungs. `IMPLEMENTATION.md` carries every departure from the design.
