# 241 - Ctrl-Z in the cockpit's chat leaves a session nobody can resume

Type: build
Status: open
Blocked by:

## What

Usability probe of 2026-10-02, session `01a0fe2b` (`hydra cockpit --session claude`, no policy).
The human pressed `Ctrl-Z` with the keys on the chat. `claude` printed "Claude Code has been
suspended. Run `fg` to bring Claude Code back" and stopped. The chat's pty has no shell behind
it: `claude` is the child of `hydra run --interactive`, so the `fg` the human typed twice went to
nobody. The orchestrating session resumed it from outside with `kill -CONT <pid>`.

`src/cockpit/chat.rs:503` turns every `Ctrl-<letter>` into its control byte and writes it to the
pty, `0x1a` included.

## Do

1. Decide in code, not on the map, between the two that keep the session alive: do not forward
   `Ctrl-Z` to the chat (say so once on the status line), or forward it and send `SIGCONT` to the
   chat's process group when the child reports stopped. Pick the first unless a provider needs
   the byte for something else; say which in the Outcome.
2. The same check for `hydra run --interactive` outside the cockpit, where a shell is behind the
   launcher: a suspended provider there must come back with the shell's `fg`. Measure it and
   write what happens; fix it only if it hangs.
3. A test on the stub provider of `tests/pty_tee.rs` or the cockpit's own: the key arrives, the
   child is not stopped afterwards.

## Done when

- The test passes; fmt, clippy `--workspace --all-targets -- -D warnings` and `cargo test` pass.
- `IMPLEMENTATION.md` says what the cockpit does with `Ctrl-Z`.
