# 219 - grader in Rust: the sheet of rankings

Type: build
Status: resolved
Blocked by: 212, 214

## What

Map tickets 93 and 94: `integrations/puppets/sheet.py` kept three hand-run commands, `build`,
`budget` and `show`, which build a ranking sheet per provider and model from run records through
`hydra show`. They go into `hydra-grader`. `hydra-puppets assign` already reads the sheet with
`rank` and `beta_lower` in `crates/puppets/src/sheet.rs`; those stay there.

## Do

Port the three commands as `hydra-grader sheet build|budget|show`, recorded and compared as the
other grader tickets do, with a stub `hydra` for `hydra show`. Then delete `sheet.py`, its tests,
and `models.py` if nothing loads it any more (`grep -rn 'models.py' integrations`); the puppets
README and the session skill `.claude/skills/puppets/SKILL.md` name the new command.

## Done when

- The sheet files the Rust writes are byte-identical to the recorded ones; `hydra-puppets assign`
  ranks the same from them (one test). fmt and clippy clean; the workspace's tests pass.

## Outcome

Resolved 2026-10-01. Run `01a0f72d-5e6c` (codex gpt-6.1-sol, `standard`, 11.3M tokens). The
session took the port on its tests and a live comparison of its own, merged it with build 218
(one conflict, two `mod` lines in the same place), and applied it with one change to CI.

Built: `crates/grader/src/sheet.rs` and `sheet_cli.rs`, `hydra-grader sheet build|budget|show`;
`collect.rs` shares its `hydra show` call; `isotime.rs` gained the budget history's date
arithmetic. `integrations/puppets/sheet.py` and `tests/test_sheet.py` are deleted; the puppets
README and `.claude/skills/puppets/SKILL.md` name `hydra-grader sheet budget`. `rank` and
`beta_lower` stay in `crates/puppets/src/sheet.rs`, and `crates/puppets/tests/sheet_build.rs`
checks that `hydra-puppets assign` ranks the same from sheets the Rust built; for that
`crates/puppets` has a dev-dependency on `hydra-grader`. `models.py` stays: `records.py` loads it.

The session's change: CI ran `python3 -m unittest discover -s integrations/puppets/tests`, which
now finds no test and exits non-zero. The step is gone.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-grader` 34 unit
and 17 differential tests; the sheet test makes 542 comparisons over 71 commands in 32 cases
(482 KB of fixtures); `hydra-puppets` 45, 5, 18 and the new assignment test. Before the
implementation 176 of 216 differed.

The session ran `sheet build` with the deleted `sheet.py`, taken from git into a scratch
directory, and with the Rust, over the real runs directory and the real goal log: the 24 sheets
under `agents/` and `sheet-cache.json` are byte-identical, and so is stdout. `sheet show` agrees
on five provider and model pairs, and on a pair with no sheet both exit 2 with the same sentence
after their own program name.

Normalized: the budget history's dates; usage lines by program name.

Left open: `sheet budget` did not run on real data. The crate's fixtures are 8.3 MB.
`sheet.py`'s `rank` subcommand had no caller but `hydra-puppets assign`, which has its own.

