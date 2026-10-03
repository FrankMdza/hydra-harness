# 248 - `hydra wait` blocks until the runs it names have ended

Type: build
Status: open
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
