# 196 - puppets in Rust: the cutover

Type: build
Status: claimed
Blocked by: 195, 207

## What

Last of four tickets for map 90's `puppets` port. The cockpit runs `puppets.py` as a subprocess in
four places (`src/cockpit/app.rs`, `snapshot.rs`, `mod.rs`); CI runs the Python suite; the README
and `hy.zsh` name `puppets.py`.

## Do

0. Before anything is deleted, close the two findings of the last review that can compare the
   two (`cursor-agent` grok-4.7-high on build 195, 2026-10-01), each with a differential input or
   a unit test shown failing first:
   - `run_file_has_end` (`crates/puppets/src/launch.rs` near 485, through `read_text` in
     `write.rs` near 671; called by `wait`, `plans` and the judge panel). The Rust reads the whole
     run file and refuses it when any byte is not UTF-8. The Python reads it as text line by line
     and returns as soon as it has seen `run_end`, so a 12 KB record with `run_end` on line 0 and
     a `0xFF` later is ended for Python and an error for Rust. Read the file as bytes, line by
     line, and decide on the lines, so one bad byte in a provider's frame cannot stop `wait` or
     a panel that already sat a seat.
   - The Hunk pusher (`crates/puppets/src/panel.rs` near 1048 to 1064). It starts
     `current_exe()` again as `push-hunk-comments`; when that path is gone the spawn fails after
     the `review` event is in the log, the command exits 1, `--no-open` never prints `hunk show
     <branch>`, and a retry sits the panel again. When the pusher cannot start, say so on stderr
     and still print the command or exec Hunk; the review is recorded either way.
1. The session ran every differential test and the ignored real-log test on master at `e4c9447`
   on 2026-10-01, all green (62 unit, 5 in the binary, 18 differential, the real-log test), and
   one real launch, recorded below. Run them once more yourself before you delete anything.
2. The cockpit runs `hydra-puppets` (found beside the `hydra` binary, then on `PATH`) wherever it
   ran `puppets.py`; its stubs in `tests/cockpit.rs` follow.
3. Delete `integrations/puppets/puppets.py`, `sheet.py`, `models.py`, `jev_probe.py` only if
   ported (else say so and keep it) and the Python tests the Rust ones replace; keep the JSON data
   files where the crate reads them.
4. `pending --json` prints an `argv` that starts `python3 <repo>/integrations/puppets/puppets.py
   approve` (`crates/puppets/src/main.rs`, `fold::approve_argv`, kept by build 193 because
   `approve` was still Python). Point it at `hydra-puppets approve`, and drop
   `log::python_dir()`, which bakes the build checkout's path into the binary; the JSON data
   files need a home that does not.
5. CI: `cargo test --workspace`, and drop the puppets Python suite step. README, `hy.zsh`, the
   Herdr plugin and `IMPLEMENTATION.md` name `hydra-puppets`.

## Notes from 195

- Wait for the review of 195 and for build 207 before this ticket: once the Python is deleted
  nothing checks the Rust against it.
- `sheet.py`'s `build`, `budget` and `corpus_records`, and `models.py` beyond `load_aliases` and
  `canonical`, have no caller in the helper's commands and were not ported. Find who runs them
  (`grep -rn 'sheet.py\|sheet build\|models.py' --include='*' .` outside `.scratch/`), and
  either port them or keep those two files and say so.
- The differential tests need the Python. Decide per test: turn its inputs and the Python's
  recorded outputs into fixtures the Rust is checked against, or delete it. Do not leave a test
  that silently skips.
- The first real launch: run `hydra-puppets` once against the real `hydra` on a throwaway goal
  (the session does this, with the human's quota in mind) before the Python goes.

## The first real launch, 2026-10-01

The session drove one goal through `hydra-puppets` against the real `hydra`, in a scratch
`HYDRA_PUPPETS_HOME`: `new`, `choose <path>`, `approve --plan`, `tasks`, `set`, a `launch` refused
for the missing table approval (exit 1), `approve --table`, `launch` (run `01a0f666`,
cursor-agent composer-2.5, 22,342 tokens, `completed`), `wait`, which read the real `hydra show`
and appended `run_end`, `board`, `pending --json`, and `verdict rejected`. `show` from the Rust
and from `puppets.py` on the resulting log were byte-identical before and after `wait`. Not
exercised against the real thing: `plan`, `plans`, `choose --judge`, `review`, a Herdr pane, Hunk.

## Done when

- fmt and clippy over the workspace; `cargo test --workspace`; `grep -rn puppets.py` finds only
  history in `.scratch/`.
