# 258 - The copilot starts on `codex`

Type: build
Status: claimed
Blocked by:

## What

Map 100, resolved 2026-10-03: the ungated copilot runs on any provider once its transcript is
read back. Build 255 made `hydra run --interactive --provider codex` work ungated and harvest its
session file, so `hydra cockpit --session codex` launches it. Build 253 made `hydra-copilot`
render one source document, `integrations/copilot/COPILOT.md`, from a list of source and
destination pairs, and pass `--disallowed-tools` to `claude`.

1. `hydra-copilot` takes the provider: `hydra-copilot [model] [repo] [provider]`, provider
   `claude` by default, `codex` the only other one accepted; anything else exits 1 with one line
   naming the two. It passes `--session <provider>` to the cockpit. Keep the argument order of
   build 253 so `suprema.sh sonnet ~/repo` keeps working.
2. The script renders `COPILOT.md` as `AGENTS.md` too, the file `codex` reads in its working
   directory. Check from `codex --help </dev/null` or the repository's `src/provider/codex.rs`
   that a `codex` started in a directory reads `AGENTS.md` there; if nothing says so, say that in
   the report.
3. The document stops assuming `claude`: no sentence says "Claude Code chat" or names `claude`'s
   tools; the edit-tools sentence says file changes go through `hydra run`, whichever provider
   the chat is. Keep it under 150 lines (`tests/copilot.rs` checks).
4. `codex` has no `--disallowed-tools`. Find in `codex --help` whether the TUI can start with
   file edits off (a read-only sandbox mode, for example) that still lets its shell run `hydra
   run`, which writes under `~/hydra`. If one exists and keeps `hydra run` working, pass it in
   `src/provider/codex.rs`'s `interactive_argv` on the ungated copilot only; if it would block
   `hydra run`, do not pass it and say so. Map 101 accepts a copilot that keeps its shell.

## Done when

- `tests/copilot_repo_and_tools.rs` gains: `codex` as third argument renders `AGENTS.md` and
  execs the stub with `--session codex`; an unknown provider exits 1. Each fails with its change
  reverted; say so. fmt, clippy over the workspace, `cargo test --lib` and the copilot tests are
  clean. `COCKPIT.md`: one sentence. `IMPLEMENTATION.md`: one sentence.

## Outcome
