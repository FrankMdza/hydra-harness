# 71 - `hydra resume` on `codex`

Type: build
Status: resolved
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

## Outcome

Measured on `codex-cli` 0.154.0 on 2026-09-22. A thread started in scratch directory A
reported `pwd` as A. Resumed from scratch directory B, the turn reported the caller's cwd as
B, not A, so the refusal in `src/resume.rs` came off.

`codex::resume_args` now returns `exec resume <id> --json -c approval_policy="never"
--dangerously-bypass-hook-trust -c hooks.PreToolUse=[...]`. `append_resume_args` replaces the
recorded `exec --json -C ...` argv for `codex` rather than splicing three words ahead of it,
then appends `-m` and the task. `tests/resume.rs` gained `codex_argv_carries_exec_resume_and_the_thread_id`;
`src/provider/codex.rs` gained `resume_argv_names_the_thread_and_the_belt_flags_without_a_worktree_pin`.

`cargo clippy --all-targets -- -D warnings`: clean. `HYDRA_ROOT=$PWD/.scratch-root cargo test
--release --test resume`: 9 passed. `HYDRA_ROOT=$PWD/.scratch-root cargo test --release --lib`:
179 passed, 1 failed (`procinfo::tests::our_own_start_time_matches_ps`, seatbelt, unrelated).

The ticket assumed a resumed session might run in the source run's recorded cwd; the
measurement showed it runs in the caller's cwd instead, which is the fresh worktree Hydra
seeds for the continuation.

Built by `cursor-agent` on Composer 2.5, run 01a0c924-7d46 under `standard`, 746k tokens.
