# 239 - The frontier is a subcommand

Type: build
Status: claimed
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
