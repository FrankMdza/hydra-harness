# 255 - `codex` runs interactive, and its transcript is harvested

Type: build
Status: resolved
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

Resolved 2026-10-03 by run `01a103a8-4b5c` (`claude` opus 5.5, `standard`, 7.08M tokens). Applied
with two stale sentences fixed by the session; no real interactive `codex` has run through Hydra
yet, and that check is the human's, in a terminal.

Built: `hydra run --interactive --provider codex [--model m] [--task t]` execs the `codex` TUI
ungated in the caller's cwd; with `--policy` it is refused, since the gated mode compiles a
`claude` gate. `codex` takes no session id, so `run_start.provider_session_id` is null and the
harvest finds the session file afterwards: the earliest `session_meta` with `source: cli`, the
run's cwd, and a start at or after the process start, under `~/.codex/sessions/YYYY/MM/DD/`.
`run_end.error` names the session when later ones also matched. `normalize/codex.rs` reads session
lines (messages, reasoning summaries, tool calls and results) and takes the last running total as
usage. `hydra status` harvests a dead interactive `codex` like a `claude` one. The measured file
shapes are a comment block in `src/provider/codex.rs`. Tests: two in `tests/harvest.rs`, two unit
tests; the agent reverted each part and saw them fail.

What the session changed: the `--interactive` help in `src/cli.rs` and one sentence of the
cockpit section of `IMPLEMENTATION.md` still said `claude` only.

Found by the agent, not fixed: every harvested `codex` tool result reads `unclassified`, so a
`completed` outcome derives as unknown; a session that hits its usage limit reads `completed`
because `outcome.rs` reads the limit only off `turn.failed`; `codex` `input_tokens` includes the
cache, against contract 9.1, as the streamed path already does; and two Hydra `codex` sessions
started in the same cwd at once would both harvest the earlier file. The TUI filter rests on one
TUI file measured, written by 0.154.0.

Corrected 2026-10-03 after the first real run, the human's (`01a103bd-e45e`, `codex` 0.159.2, in
`~/another-private-repo`): it closed `failed` with "the session wrote no codex session file". The
file was there, but 0.159.2's terminal TUI writes `source: "vscode"`, where the one 0.154.0 file
the agent measured said `cli`. The finder now matches `originator: "codex-tui"`, which both
versions write and `codex exec` (`codex_exec` on all 139 exec files) does not. The harvest test's
own session now says `vscode`, and fails against the old filter. That run's record stays closed
as it was. Also seen on that run: 0.159.2 printed "Disconnected from this task. Any running work
continues", so the TUI process can exit while its turn goes on.
