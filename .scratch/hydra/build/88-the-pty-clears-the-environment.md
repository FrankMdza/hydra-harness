# 88 - The pty clears the inherited environment before the allowlist

Type: build
Status: resolved
Blocked by:

## What

Finding G1 of the second hiring panel, 2026-09-23, raised by the Claude Opus reader: `src/pty.rs`
said `CommandBuilder::new` starts from an empty environment, and portable-pty 0.9.0's
`CommandBuilder::new` calls `get_base_env()`, which copies `std::env::vars_os()`. The detached
watcher is spawned without clearing its environment, so a recorded run inherited the whole shell
that ran `hydra run`. The probes and the interactive launch call `env_clear` and were not affected.

## Done when

The pty clears the environment before applying the caller's; a test fails without the clear;
a real launch on each provider still starts.

## Outcome

`cmd.env_clear()` before the allowlist in `pty::open`, and the comment corrected.
`tests/pty_tee.rs::the_child_gets_the_environment_it_is_handed_and_nothing_else` spawns
`/bin/sh -c env` handing it only `PATH`: without the clear it failed listing the test process's
whole environment, `GEMINI_API_KEY`, `SSH_AUTH_SOCK` and `CLAUDE_CODE_MESSAGING_TOKEN` among it;
with the clear it passes. pty_tee 5, detached 4, spawn 32, status 20, drift 4 passed. Real launches
under `read-only` after the fix: `claude` on haiku (run 01a0ccb1-2fea), `cursor-agent` on
composer-2.5 (01a0ccb1-681d) and `agy` on gemini-3.8-flash-low (01a0ccb1-83c0) all answered
`pong`; `codex` was out of credits and is measured by the panel's relaunch. `SECURITY.md` states
the exposure under known exposures, dated. Done by the session by hand, no agent run.
