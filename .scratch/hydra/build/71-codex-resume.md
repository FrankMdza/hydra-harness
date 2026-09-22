# 71 - `hydra resume` on `codex`

Type: build
Status: open
Blocked by:

## What

`codex::resume_args` returns `exec resume <id>` and `run::append_resume_args` splices it ahead
of the recorded argv, which begins `exec --json -C <worktree> --sandbox ...`. The result starts
`exec resume <id> exec --json -C`, and `codex exec resume --help` on 0.154.0, read 2026-09-22,
takes `--json`, `-m` and `-c` and neither `-C` nor `--sandbox`. No run ever tried it:
`tests/resume.rs` covers the other three providers and had no `codex` case. The hiring-panel
review of 2026-09-22 found it by reading, and `src/resume.rs` refuses the provider with the
reason until this closes.

## Done when

A `codex` resume builds `exec resume <id> --json -c approval_policy="never"
--dangerously-bypass-hook-trust -c hooks.PreToolUse=[...] [-m model] <task>` with no `-C` and no
`--sandbox`, and the question of where that session runs is answered before the refusal comes
off: a resume gets a fresh run id and a fresh worktree, and `codex` resumes in the cwd the
session recorded, which is the reaped worktree of the source run. Either the resume seeds its
worktree at the old path, which `hydra reap` may have removed, or the launch passes the new cwd
some other way. Measure one resumed run on 0.154.0 and put the argv in `IMPLEMENTATION.md`.
