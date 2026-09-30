# 155 - Block logos in colour, from the mascot's own pixels

Type: build
Status: resolved
Blocked by:

## What

Build 153 drew six logo candidates, four in plain ASCII and two in block characters. The human
looked at them on 2026-09-30 and said no to plain ASCII: the marks `claude` and `agy` print
at start are block art with colour, and the one candidate he liked was `splash-blocks`. This
ticket is the second round, in that class only.

The source is the mascot's own pixel art, not a drawing from memory:
`integrations/herdr/sprites/hydra-hydrapet-micro.txt` is the 24 by 24 HydraPet sprite with its
palette (`K`, `G`, `L`, `g` and their RGB), and `integrations/herdr/bin/hydra-mascot --sprite`
renders sprites as half blocks. `docs/logo/candidates.md` and `docs/logo/render.py` exist from
build 153; `splash-blocks` is the bar to beat.

## Do

1. Read the sprite and how `hydra-mascot` maps two pixel rows to one row of `▀` `▄` `█` with
   24-bit colour, so the logos use the same palette and the same half-block rule.
2. Add six candidates to `docs/logo/candidates.md`, all in the `splash-` class, block or
   half-block characters only, each in two forms: with 24-bit colour escapes for a terminal, and
   the same shape without colour for a README. Sizes:
   - two of one row, at most 24 columns: three heads side by side, the word `hydra` after them;
   - two of two or three rows, at most 32 columns, for the cockpit header;
   - two of four to six rows, at most 48 columns, for the README top: the sprite's three heads
     and necks cropped and downscaled from the 24 by 24 art, not redrawn.
   At least two of the six come from the sprite by downscaling with a script, so the creature
   is the same; say which.
3. `docs/logo/render.py` prints the colour form when stdout is a terminal or `--color` is
   given, and the plain form otherwise. `docs/logo/test_render.py` extends its limits to the new
   candidates and checks that the plain form of each has no escape codes and the colour form
   uses only the sprite's palette.
4. Keep the six of build 153 in the file, under a heading that says the human passed on the
   ASCII ones.

## Done when

- Twelve candidates in the file, `render.py` prints any by name in both forms, the test passes
  under `.venv/bin/python` if the worktree has it, else `python3`.
- Your report shows the six new ones rendered in their plain form, says which two came from the
  sprite by script, and starts with what you could not confirm.

## Outcome

Resolved 2026-09-30. Built by `claude` sonnet on run 01a0f07c-1bbd under `standard`, about 0.66M
tokens. The session applied it as written, plus the script the run added, `docs/logo/from_sprite.py`.

Six new candidates, all block art with a colour form in the sprite's palette and a plain form:
`splash-tiny-sprite`, `splash-tiny-hand`, `splash-bar-sprite`, `splash-bar-hand`,
`splash-top-sprite`, `splash-top-small`. Four come from the 24 by 24 sprite by downscaling with
the script; the two `-hand` ones are typed from the palette and are the weakest match to the
creature. `render.py` prints colour on a terminal or with `--color`, plain otherwise; the test
checks twelve candidates, no escapes in the plain form, and only the palette in the colour form.

Found by the run: the micro sprite holds two whole heads and a two-pixel sliver of the third at
its right edge, so "the sprite's three heads" is not literally there; the `top` candidates keep
the sliver, and the one-row ones repeat a head. `splash-tiny-sprite` is 25 columns with the word,
one over the limit.

This run is also the first with `config/mcp.md` saying `command = "hydra"`: its line 0 carries
`mcp.resolved: ["hydra"]`, so build 154's resolution works on a real launch.

The human picks. No README or cockpit change yet.
