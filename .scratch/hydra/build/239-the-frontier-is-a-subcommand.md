# 239 - The frontier is a subcommand

Type: build
Status: resolved
Blocked by: 

## What

Map ticket 95, question D, "S1" of `.scratch/hydra/research/95-the-plan-for-the-pieces-left.md`: `.scratch/hydra/frontier.py` prints which tickets
are takeable and which are blocked, for the map and for the build. It becomes `hydra-puppets
frontier [build]`, since the helper is the orchestration tool and already reads this checkout.

## Do

1. Port it into `crates/puppets` with the Python's output byte for byte over the committed
   `.scratch/hydra/issues/` and `.scratch/hydra/build/` (record both commands' output once; the
   directory is found from the working directory as the Python finds it, and a `--dir` names
   another for the test). `\w+` in the Python is Unicode and `lstrip("# ")` strips a set of
   characters. A `Blocked by:` line may hold words as well as numbers (build 230 has one):
   record what the Python does with it and do the same.
2. Do not delete `frontier.py` and do not edit `CLAUDE.md` or `.scratch/`: the session does,
   after it has run both side by side.
3. `IMPLEMENTATION.md`: one sentence where the helper's commands are listed.

## Done when

- The two outputs are byte-identical to the recorded ones; fmt and clippy clean; `cargo test -p
  hydra-puppets` passes with no existing fixture changed.

## Outcome

Resolved 2026-10-01, in two runs: `01a0f781` (codex gpt-6.1-sol) wrote the tests and their
fixtures and stopped at the account's usage limit; `01a0f784` (cursor-agent composer-2.5, 0.9M
tokens) wrote the port from there. The session applied it with one change to the tests.

Built: `crates/puppets/src/frontier.rs`, `hydra-puppets frontier [build] [--dir DIR]`. It walks
up from the working directory to `.scratch/hydra`, reads `issues` or the named tracker, parses a
ticket as the Python does (`\w+` for status and type, `Blocked by: (.+)` with an empty tail
skipped, the title through `lstrip("# ")`), sorts by the number a file name starts with, and
prints the same sections.

The session's change: two of the three tests compared the binary with a recording of the live
`.scratch/hydra` trackers, which change with every ticket, though the task said not to. They and
their two fixtures are gone; a new test lays the frozen edge tickets out as a checkout's `issues`
and `build` and finds them from a directory two levels down.

Measured on master: fmt and clippy clean over the workspace; `cargo test -p hydra-puppets` with
the two frontier tests, no other fixture changed. The session ran `python3
.scratch/hydra/frontier.py` and `hydra-puppets frontier`, with and without `build`, on the live
trackers: byte-identical (5 and 40 lines), and again from a nested directory.

Left open, the session's to do with the human's word: `frontier.py` is still there and
`CLAUDE.md` still names it in four places. Both change together.

