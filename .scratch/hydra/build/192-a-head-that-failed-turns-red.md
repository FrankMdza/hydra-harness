# 192 - A head that failed turns red

Type: build
Status: claimed
Blocked by: 191

## What

The human's cockpit redesign: "if an agent dies, crashes or is rate limited, its head blinks red
or is cut". Build 152 gave the dragon eight fixed poses rendered by
`integrations/herdr/mascot/hydrapet.py` (`render_hd_dragon`, `render_compact_dragon`, which take a
state per head: `awake`, `sleeping`, `firing`) and written by `integrations/herdr/bin/hydra-mascot
--frames <dir>` into `integrations/herdr/mascot/frames/` with `index.json`. Build 177 made the
heads count the watched session's seats, and names a seat that ends not ok for one refresh, with
the whole dragon flashing `refused` or `meme`.

## Do

Tests first: the Python test for `--frames` in `integrations/herdr/tests/test_mascot.py`, and
`tests/cockpit.rs`, each shown failing before the change.

1. A head state `hurt` in both renderers: the head drawn in red tones, eyes shut, tilted down.
   Keep it readable at 24x12 half-blocks; say in the report how you checked that (for example,
   print the frame to the terminal and describe it).
2. `--frames` also writes `hurt_0`, `hurt_1`, `hurt_2`: head `i` hurt, the others awake. Regenerate
   and commit the frames with the README's command; `index.json` lists them.
3. `src/cockpit/mascot.rs`: seats map to heads in the order they started (first seat head 0).
   When a seat on head 0 to 2 ends `crashed`, `failed`, `rate_limited` or `stopped`, the dragon
   shows `hurt_<i>` for that refresh instead of `meme`; `gated` keeps `refused`. A seat past the
   third head keeps today's flash.
4. `IMPLEMENTATION.md`, the mascot paragraph.

## Done when

- Each test failed before the change; the herdr Python tests; fmt, clippy, `cargo test --lib`,
  `--test cockpit` pass.
