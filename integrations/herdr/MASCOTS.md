# Every hydra mascot so far, and how to see each one

All of them run from `~/hydra` in Ghostty (24-bit colour). `M` is the renderer:

```
M="python3 ~/hydra/integrations/herdr/bin/hydra-mascot"
```

Add `--demo 30` to any of the sprite commands to see it move for 30 seconds (idle, working,
one head asleep with zzz and a snot bubble, a refused flash, the meme head, fire), or `--png
/tmp/x.png` to save a picture instead of printing.

## 1. The HydraPet dragon, 24 pixels, the one the human picked (2026-09-24)

The default. Drawn per frame by the vendored `mascot/hydrapet.py`: the necks sway, the chest
breathes, a sleeping head droops and blows a bubble, fire from all three jaws.

```
$M                       # one frame, 48 columns by 24 rows
$M --demo 30             # the pose cycle
$M --scale 1 --demo 30   # the raw 24 columns, half the size
$M --save ~/.cache/hydra-mascot.ans   # then in .zshrc: $M --load ~/.cache/hydra-mascot.ans
```

The original standalone pet, with its own keys (space fire, 1 2 3 sleep a head, S all, P pet,
Tab size, Q quit). Needs numpy.

```
python3 ~/hydra/integrations/herdr/mascot/hydrapet.py --size 20 --mode ansi
python3 ~/hydra/integrations/herdr/mascot/hydrapet.py --size hd --mode kitty    # the 160 px canvas art, Kitty graphics
```

## The watch pane switches between them

`$M --watch` (the `mascot` pane, and what `hy` opens above the session) starts on the last skin
you chose and cycles with Tab or `m`: the HD dragon (HydraPet's 160-pixel canvas art in half blocks, 48
columns), the 20-pixel pet, codex sprite, session sprite, agy sprite. `q` quits.
`--skin codex` picks the first one. The choice is kept in `~/.cache/hydra-mascot-skin`.

## 2. The pixel-art sprites, 48 to 60 columns, one file each under `sprites/`

Pokemon-colorscripts style: one character per pixel, a palette, the jaws marked for fire.

```
$M --small                                              # codex's, the best of the art panel, 60 x 36
$M --sprite integrations/herdr/sprites/hydra-codex.txt  # the same, by name
$M --sprite integrations/herdr/sprites/hydra-session.txt        # the session's first, 48 x 28
$M --sprite integrations/herdr/sprites/hydra-agy.txt            # agy's, purple wings, 56 x 32
$M --sprite integrations/herdr/sprites/hydra-hydrapet-micro.txt # the 24 x 24 micro sprite from the HydraPet web app
```

To draw a new one, copy `sprites/hydra-session.txt`, edit the map (`.` is transparent), keep the
rows the same width, and give it a palette and three `mouth` lines.

## 3. The reference image rendered in half blocks

The three-headed dragon from the Gemini image, cropped, shrunk and printed as `▀` cells with
true colour. Reads as a dragon at 60 columns and up, a blob below that.

```
IMG="$HOME/Downloads/Gemini_Generated_Image_a6b15xa6b15xa6b1.jpeg"
$M --image "$IMG" --width 96 --demo 30
$M --image "$IMG" --width 60
```

## 4. The ones that are gone from the tree

- Opus's hand-drawn block sprite (11 rows by 36 columns of `▟█▙`): the human called it ugly.
  It is still in commit ccc7a58 and in the opus proposal beside the panel brief,
  `~/.claude/handoffs/2026-09-24-puppets-panel/proposal-claude.md`.
- The HydraPet web app (React, canvas, GIF studio) lives at github.com/FrankMdza/hydra-mascot,
  not here: `npm install --legacy-peer-deps && npx vite --port 3000`, then open the page.

## Sizes, for choosing

| Mascot | Columns x rows | Moves on its own | Reads real runs |
|---|---|---|---|
| HydraPet (default) | 48 x 24, or 24 x 12 at scale 1 | yes, per frame | after build 113 |
| codex sprite | 60 x 18 plus fire margins | bob, blink, overlays | after build 113 |
| session, agy, micro sprites | 48 to 56 wide | same | same |
| image render | 60 to 96 wide | overlays only | no |
