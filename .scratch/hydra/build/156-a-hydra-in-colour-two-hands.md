# 156 - A hydra in colour, two hands, three designs each

Type: build
Status: resolved
Blocked by:

## What

Rounds one and two (builds 153 and 155) drew small logos, first in ASCII and then in blocks cut
from the 24-pixel sprite. The human passed on all twelve on 2026-09-30: too small, too tame, and
not what he meant. What he meant is the mark a frontier lab prints when its CLI starts, but
hand-made and with teeth: a HYDRA, in colour, pixel art the way `pokemon-colorscripts` draws a
pokemon in the terminal (half blocks `▀` `▄` with a 24-bit foreground and background per cell,
about 40 columns by 20 rows, black outlines, flat colours).

Two runs, one on `codex` gpt-6-astra and one on `claude` Opus 5.5, three designs each, no
restrictions on size, palette or style beyond the reference. The human picks.

## Do

1. Look at the reference once: `pokemon-colorscripts -n charizard --no-title` prints the style
   (it is on `PATH`; if it is not, `~/.local/bin/pokemon-colorscripts`). Look at the mascot's
   sprite and skins under `integrations/herdr/sprites/` and `integrations/herdr/mascot/` so the
   creature is recognizable as Hydra's three-headed dragon, but draw your own.
2. Three designs, each a pixel grid you draw by hand as text: one character per pixel, a legend
   mapping characters to RGB, transparent as `.`. Any width and height you want. Three heads.
   One of the three should carry the word `hydra` inside or beside the art, in your own letters.
3. `docs/logo/round3/<your provider>/design-N.txt` for the grids, and one script
   `docs/logo/round3/render.py` (stdlib only; if the other hand wrote it first, reuse it and do
   not rewrite it) that renders a grid as half blocks with 24-bit foreground and background, the
   way the reference does, and a `--plain` flag that prints the shape without colour.
4. `docs/logo/round3/<your provider>/README.md`: one paragraph per design, what you were going
   for. Nothing else in the repository changes.

## Done when

- Three grids and the README under your provider's directory, the renderer prints each one, and
  your report shows the three in `--plain` form and says what each looks like in colour.

## Outcome

Resolved 2026-09-30. Two hands at once: `codex` gpt-6-astra on run 01a0f08c-f3f8 (about 0.72M
tokens) and `claude` Opus 5.5 on run 01a0f08d-1379 (about 1.24M). Each drew three grids by hand
and its own renderer; the session kept both renderers, `render.py` (claude) and
`render_codex.py` (codex), since the grid formats differ in where the legend sits.

Codex: a broad emerald gatekeeper with wings (46 by 35 pixels), a turquoise serpent coiled with
three heads at different heights, and a violet crest over pixel letters `hydra`. Claude: the
rearing hydra standing like the reference (52 by 40), a crimson mirror-symmetric crest with
three open jaws, and a smaller hydra beside block letters `hydra`. Neither hand saw its work in
colour; the descriptions come from their legends.

The human picks. Nothing else changed.
