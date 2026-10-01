# 219 - grader in Rust: the sheet of rankings

Type: build
Status: claimed
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
