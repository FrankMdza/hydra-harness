# 300 - The build index catches up: `plan.md` lists every ticket again

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session, which found `plan.md`'s `## The tickets` list ending at 134
while the directory holds tickets to 300.

## What

`CLAUDE.md` says `.scratch/hydra/build/plan.md` indexes the build. It did until ticket 134; the
166 tickets after it exist only as files. The frontier tool reads the files, so nothing broke,
but the one-load overview the index promised is gone, and a reader who trusts `CLAUDE.md` gets
half the build.

## Do

1. Regenerate the `## The tickets` list from the directory: one line per `NNN-slug.md`, in
   number order, `- [NNN - <title from the first line>](NNN-slug.md)`, exactly the form the
   existing entries use. Keep the prose above and the `## Out of scope` section below unchanged.
2. Add to `crates/puppets/` a `hydra-puppets index build` subcommand that prints that list from
   the directory, so the next drift is one command away, and say so in one sentence under
   `## Notes` in `plan.md` and in `IMPLEMENTATION.md`.

## Done when

- `plan.md` lists every ticket in the directory and nothing else; `hydra-puppets index build`
  printed against the directory equals the list in the file (a test in `crates/puppets/tests/`
  on a fixture directory of three tickets). fmt, clippy, `cargo test -p hydra-puppets`.

## Outcome

