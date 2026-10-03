# 255 - `codex` runs interactive, and its transcript is harvested

Type: build
Status: claimed
Blocked by:

## What

Map 100, resolved 2026-10-03: the ungated copilot goes to all four providers, `codex` first, and a
provider is not a copilot until Hydra can read its transcript back into a run record (D2). Today
`hydra run --interactive` is `claude` only (`src/interactive.rs`, its header says why): `claude`
writes an append-only `.jsonl` at a path Hydra predicts from the session id it passes.

`codex` keeps dated session files under `~/.codex/sessions/` (harness spec 10). Whether their
path can be known before or after the run, what a line holds, and where usage sits is for this
ticket to measure from files already on this machine, not to guess.

## Do

1. Read `src/interactive.rs` (launch and `harvest`), `src/provider/codex.rs` and
   `src/normalize/codex.rs`. Read two or three existing files under `~/.codex/sessions/` with
   shell (`ls`, `head`), enough to write down: the path pattern, how a file names its session,
   the line shapes for a user turn, an assistant turn, a tool call and token usage. Put that in
   the report and as a comment block in the code. Copy no user content into the repository: the
   test fixture is synthetic, built from the shapes.
2. Ungated only: `hydra run --interactive --provider codex [--model m]` execs `codex` in the
   caller's cwd with stdio inherited, as the `claude` path does, and writes `run_start`. Find the
   interactive top-level command in `codex --help </dev/null`. With `--policy`, refuse as today
   with a line that says the gated interactive mode is `claude` only.
3. The harvest: when `codex` exits, find the session file this run wrote (by the session id if
   `codex` takes or prints one, else by the newest file created after `run_start` whose cwd
   matches; say which and why) and write `run_end` with the outcome and `usage` from it.
   `hydra status` harvests a dead one the same way it does `claude`'s.
4. `hydra cockpit --session codex` then works with no other change, or say what blocks it.
5. Tests: a synthetic session file in a scratch `HOME` harvests into a valid record
   (`record_contract`'s validator) with the right token counts; a run with no session file ends
   with usage unknown, not a crash. Each fails with its part reverted; say so.
6. `IMPLEMENTATION.md`: amend the interactive section's "`claude` only".

## Done when

- fmt, clippy over the workspace, `cargo test --lib`, `cargo test --test harvest` and the new
  tests are clean. The session launches one real interactive `codex` from master and harvests it;
  you cannot from inside a run.

## Outcome
