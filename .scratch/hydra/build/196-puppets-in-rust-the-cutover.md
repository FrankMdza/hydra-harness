# 196 - puppets in Rust: the cutover

Type: build
Status: open
Blocked by: 195

## What

Last of four tickets for map 90's `puppets` port. The cockpit runs `puppets.py` as a subprocess in
four places (`src/cockpit/app.rs`, `snapshot.rs`, `mod.rs`); CI runs the Python suite; the README
and `hy.zsh` name `puppets.py`.

## Do

1. The session first runs the ignored real-log differential test from 193 and every differential
   test, and records the result in the ticket.
2. The cockpit runs `hydra-puppets` (found beside the `hydra` binary, then on `PATH`) wherever it
   ran `puppets.py`; its stubs in `tests/cockpit.rs` follow.
3. Delete `integrations/puppets/puppets.py`, `sheet.py`, `models.py`, `jev_probe.py` only if
   ported (else say so and keep it) and the Python tests the Rust ones replace; keep the JSON data
   files where the crate reads them.
4. CI: `cargo test --workspace`, and drop the puppets Python suite step. README, `hy.zsh`, the
   Herdr plugin and `IMPLEMENTATION.md` name `hydra-puppets`.

## Done when

- fmt and clippy over the workspace; `cargo test --workspace`; `grep -rn puppets.py` finds only
  history in `.scratch/`.
