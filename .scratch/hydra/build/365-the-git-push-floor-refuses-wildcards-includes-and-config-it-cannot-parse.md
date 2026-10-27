# 365 - The `git push` floor refuses wildcards, includes, and config it cannot parse

Type: build
Status: claimed
Blocked by: 362

## Objective

Close the push-floor gaps the four-seat review of `2e99a25..05a9e21` found in build 362 and the build 357 code under it. Seats: Opus `01a17e81-3770`, GPT `01a17e81-cf35`, Grok `01a17e82-1d83`, Flash `01a17e82-b1a5`. Opus and Grok ran each input below against git 2.55 and the gate. The gate's config scanner is a line reader, not git's parser, so the floor takes the safe side: anything git may read as a push target the scanner cannot judge is refused.

## Cases

Each one is a `git push` the gate allows today under `standard` and must refuse with `hydra/floor`, except case 11.

1. Refspec `refs/heads/*:refs/heads/*`, on the command line and as a `remote.origin.push` value. `push_refspec_violation` in `src/policy/gate.py` line 1160 compares the target `*` with the protected names.
2. Refspec `refs/heads/*:refs/heads/release/*` and `topic:refs/heads/mas*`. Any target containing `*` is refused.
3. Refspec `:` on the command line, `git push origin :`. Git pushes every matching branch. An empty target after the last `:` with an empty source is refused. A delete, `:topic`, is judged by its target as today.
4. `[remote.origin]` followed by `push = HEAD:refs/heads/master`. Git reads the dotted form as remote `origin`. `config_section` at line 1076 has to split `name.sub`, lowercasing the whole dotted form.
5. `[remote "origin"] push = HEAD:refs/heads/master`, with the key on the header line, and `[push] default = matching` on one line.
6. `[remote "origin"] # note`, a header followed by a comment.
7. A value continued with a trailing backslash: `push = HEAD:refs/heads/\` and next line `master`.
8. Any `[include]` or `[includeIf ...]` section in either config file the scanner reads. The scanner does not follow includes, so their presence refuses a push that names no refspec.
9. `git -c include.path=<file> push origin` and `git -c includeIf.<cond>.path=<file> push origin`. Refused like `--config-env`.
10. A shell assignment before `git` that names `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM`, `GIT_CONFIG` or any `GIT_CONFIG_KEY_<n>`, followed by a push. The gate does not read the variable, so it refuses, as it does `--config-env`. Find where the gate strips assignments before it calls `check_git`, around line 639, and pass them through.
11. Still allowed: `git push origin hydra/test:refs/heads/feature`, `git push origin hydra/test`, a bare `git push` on `hydra/test` with no include and no push config, and `git push origin :topic`.

## Do

1. Tests first in `tests/gate.rs`, one test per group of cases above, in the style of `git_push_floor_reads_remote_push_and_push_default_from_repo_config`. Each must fail before the change; say in the Outcome which ones did.
2. Change `src/policy/gate.py`. Keep the safe-side rule in one comment: the scanner is not git's parser, so a shape it does not understand refuses.
3. Do not touch `src/sandbox.rs`, `src/reap.rs` or `src/worktree.rs`. The private git directory findings from the same review are a separate decision.
4. Amend `SECURITY.md` around line 362, which says build 362 "closed the repository config gap". Say what the floor reads, what it refuses because it cannot read it, and that `config.worktree` is still not read. Amend the build 362 paragraph of `IMPLEMENTATION.md` near line 1115 the same way.
5. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo nextest run --release -p hydra --test gate`, and `cargo test --release --workspace --doc`.

## Outcome
