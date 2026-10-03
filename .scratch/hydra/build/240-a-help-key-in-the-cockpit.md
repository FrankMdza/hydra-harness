# 240 - A help key in the cockpit

Type: build
Status: resolved
Blocked by:

## What

Found by the human on 2026-10-02, in the first gated session anyone drove by hand
(`hydra cockpit --session claude --policy standard`, run `01a0fdeb`). The cockpit has sixteen
keys and prints none of them. The status line says which side has the keys, `F2` and `Ctrl-q`
(`src/cockpit/host.rs:270`); the policy panel is the only view that lists its own keys
(`src/cockpit/app.rs:1774`). The human asked what "the chat has the keys" meant and where the
navigation was.

## Do

1. `?` with the keys on the cockpit opens a help view over the right-hand panels and `?` or `Esc`
   closes it. It lists every key the cockpit answers, grouped as the README's table groups them
   (`README.md`, `## Commands`, the cockpit rows): moving, opening, the four writes that wait for
   a `y`, the views (`k`, `r`, `i`, `p`). One line each. With `--session` it also says what `F2`
   and `Ctrl-q` do and that every other key goes to the chat while the chat has the keys.
2. One table in the code feeds both the help view and the key dispatch's test, so a key added to
   `App::on_key` without a help line fails a test. Do not parse the README.
3. The status line gains `? help` while the cockpit has the keys, and says `F2 then ? for help`
   while the chat has them. `?` is not taken from the chat: with the keys on the chat it goes to
   the provider as today.
4. Reword the two side labels if a plainer pair fits the width: the human read "the chat has the
   keys" as jargon. `keys go to: chat` and `keys go to: cockpit` is one candidate.
5. `README.md` and `IMPLEMENTATION.md`: the key in the table, one sentence.

## Done when

- A test opens the help view on a synthetic app, reads every bound key back from the buffer, and
  closes it with `Esc`; a second test fails when a key is bound and not listed.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test`
  pass. Nothing under `config/`, `policies/` or `docs/` changes.

## Outcome

Resolved 2026-10-03 by run `01a1018e-49fe` (`cursor-agent` composer-2.5, `standard`, 3.68M
tokens), applied by the session from the ticket's files only.

Built: `src/cockpit/keys.rs`, one `HELP` table of `(KeyCode, MainKey)` bindings that both
`App::key`'s main dispatch and the `?` overlay read; `?` toggles the overlay, `Esc` closes it.
The status line reads `keys go to: chat | F2 then ? for help` or `keys go to: cockpit | ? help`.
Tests: `question_opens_help_lists_every_key_and_esc_closes_it` and
`every_main_dispatch_key_is_listed_in_the_help_table` in `tests/cockpit.rs`, and
`every_main_key_has_a_help_line` in the module.

Measured by the session on master: with the `i` binding taken out of `HELP`, both table tests
fail; restored, they pass. `cargo test --test cockpit` 114 passed, 1 ignored; `--lib` 227; fmt
and clippy clean.

Not applied: the run tried a `git stash`, whose stack every worktree shares, and applied the one
entry there (`ticket27-wip-check-preexisting-failure`, from September) into its worktree. That
brought a change to `src/outcome.rs` and to `.scratch/hydra/map.md` and `plan.md`, which the run
then repaired by hand. The entry is still on the stack, nothing was lost, and none of it reached
master. Not checked by hand in a real terminal: the overlay's look. `?` pressed during a `y`
confirmation toggles the overlay rather than cancelling; harmless, noted.
