# 248 - `hydra wait` blocks until the runs it names have ended

Type: build
Status: resolved
Blocked by:

## What

Map ticket 97, D7 (resolved 2026-10-03): a copilot waits with one blocking command, not by
polling. On 2026-10-02 four Opus copilots spent 0.43M to 1.77M tokens, nine tenths of it cache
read, much of it from turns that called `hydra status` and `hydra show` until their agents ended.

## Do

1. `hydra wait <run>... [--any] [--timeout 30m]`: resolves each prefix as `show` does, blocks until
   every named run has ended (or the first, with `--any`), then prints one line per run: the full
   id, provider, model, outcome, total tokens. Exit 0 when all ended, 2 on timeout with the ones
   still live listed. Liveness is `live.rs`'s, never a guess from the file.
2. It reads only the run files and the pid test, writes nothing, and costs no CPU while waiting
   beyond a poll of the files every few seconds (a filesystem watch is fine if it is simpler).
3. `README.md`'s command table and `IMPLEMENTATION.md`: one row, one sentence.

## Done when

- Tests drive it on stub records and a stub live pid the way `tests/status.rs` builds states by
  hand: returns when the last run ends, `--any` on the first, the timeout exit code and its list.
- fmt, clippy over the workspace and `cargo test` pass.

## Outcome

Resolved 2026-10-03 by run `01a101a0-b0ae` (`codex` gpt-6.1-sol, `standard`, 2.56M tokens),
applied by the session with one change.

Built: `hydra wait <run>... [--any] [--timeout]` in `src/wait.rs`. It resolves each prefix as
`show` does, polls `live::read_view` every two seconds, and prints one tab-separated line per run:
full id, provider, model, outcome, total tokens. Exit 0 when the runs ended, 2 on timeout with
`still live: <id>` for each. Tests: three in `tests/wait.rs` (all, `--any`, timeout).

Changed by the session: the run derived the outcome itself, rebuilding a `normalize::Run` with
permission decisions guessed from the record rather than replayed, so `wait` and `show` could
disagree about one run. The summary now reads `status::projection`, the object `hydra show`
prints. Tried on master against real records: two ended runs printed `completed` with the token
counts `hydra show` gives; a reaped run's projection also prints its usual `no diff stat` line on
stderr.

Not measured: whether a copilot actually waits with it instead of polling. That is build 249 and
the human's next session.
