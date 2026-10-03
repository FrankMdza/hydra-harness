# 241 - Ctrl-Z in the cockpit's chat leaves a session nobody can resume

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-03 by run `01a1019b-d792` (`cursor-agent` composer-2.5, `standard`, 0.95M
tokens), applied by the session with one change: the status message.

Built: the first of the ticket's two options. `Chat::send_key` withholds `Ctrl-Z` and returns
false; the host says once on the status line `Ctrl-Z is not sent to the chat: nothing behind it
could resume the provider`. The run's wording told the person to suspend the provider "from the
orchestrating terminal", which does not exist; the session rewrote it. Test:
`ctrl_z_with_the_chat_focused_does_not_suspend_the_child` in `tests/cockpit.rs`, on the stub
echo provider.

Measured by the session on master: with the early return removed the test fails; restored, it
passes. `--test cockpit` 115 passed and 1 ignored, `--lib` 229, fmt and clippy clean.

Step 2, `hydra run --interactive` outside the cockpit, was read and not run: both launches
inherit the terminal's stdin with no pty of Hydra's, so the shell's job control applies and `fg`
should bring the provider back. Unmeasured.
