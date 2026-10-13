# 300 - The build index catches up: `plan.md` lists every ticket again

Type: build
Status: resolved
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

## Outcome

Built by `codex` `gpt-6.1-sol` on Hydra run `01a13431-b2d5-72b3-8842-f3b36f545e9a` under
`standard`, which died at its last turn on "Selected model is at capacity" after writing
`crates/puppets/src/index.rs`, the `index build` subcommand, its test on a three-ticket fixture
and the generated list. The session finished the rest on master on 2026-10-13: spliced the
command's output into `plan.md` (300 entries, byte-equal to what the command prints from the
directory), the sentence under `## Notes`, the paragraph in `IMPLEMENTATION.md`, and one clippy
fix the builder never got to run (`is_ascii_digit` for a manual range check). fmt, clippy and
`cargo test -p hydra-puppets` green on master.
