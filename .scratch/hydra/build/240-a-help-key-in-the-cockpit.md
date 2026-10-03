# 240 - A help key in the cockpit

Type: build
Status: claimed
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
