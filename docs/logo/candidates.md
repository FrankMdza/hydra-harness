# Hydra logo candidates

Twelve drafts of a small mark, built from the three-headed HydraPet dragon. The human picks.
Each heading is the candidate's name, then the art in a fenced block, then for round two a
`paint` block, then one line on where it fits. `python3 docs/logo/render.py NAME` prints one,
in colour on a terminal or with `--color`, in plain block characters otherwise.

## Round two: block art in colour

The human passed on plain ASCII and liked `splash-blocks`, so these six are all `splash-`. The
art block is the plain form. The `paint` block has two pixel rows per art line, one letter per
pixel from the sprite's palette (`G` scale, `L` highlight, `g` wing, `y` chest, `E` eye, `H` horn),
`.` for nothing and `~` for a letter of the word. The colour form draws each cell as the
mascot does, top pixel as the foreground of `▀`, bottom pixel as its background. In the plain
form an eye pixel is a hole. The sprite is 24 pixels wide, so its third head is only a two pixel
sliver at the right edge; the sprite-made candidates carry that sliver as it is.
`docs/logo/from_sprite.py` makes the four marked "Made by" and prints the two typed by hand.
Limits: `splash-tiny-` is one row, 24 columns. `splash-bar-` is two or three rows, 32 columns.
`splash-top-` is four to six rows, 48 columns.

## splash-tiny-sprite

```
▄█▀█▄ ▄█▀█▄ ▄█▀█▄  hydra
```

```paint
.HgH...HgH...HgH...~~~~~
gGEGg.gGEGH.gGEGH..~~~~~
```

Fits: a shell prompt or a tab title. One row. Made by `from_sprite.py`: the sprite's head crop shrunk to 5 by 2 pixels, the left head once and the middle head twice.

## splash-tiny-hand

```
█▄█ █▄█ █▄█  hydra
```

```paint
GEG.GEG.GEG..~~~~~
LGL.LGL.LGL..~~~~~
```

Fits: a shell prompt or a tab title. One row. Typed by hand from the sprite's palette: scale, eye and highlight per head.

## splash-bar-sprite

```
▄█▀█▄ ▄█▀█▄█
 ▀██▀ ▀██▀▀█  hydra
```

```paint
.HgH...HgH.H.......
HGEGg.gGEGHG.......
.GGGg.gGGGgG..~~~~~
..GG...GG..g..~~~~~
```

Fits: the cockpit header. Two rows. Made by `from_sprite.py`: the sprite's top 8 rows shrunk to 12 by 4 pixels.

## splash-bar-hand

```
 ▄█▄█▄ ▄█▄█▄ ▄█▄█▄
 █▄█▄█ █▄█▄█ █▄█▄█
  ▀██▄▄▄███▄▄▄██▀    hydra
```

```paint
..H.H...H.H...H.H.........
.gGGGg.gGGGg.gGGGg........
.GEGEG.GEGEG.GEGEG........
.LGGGL.LGGGL.LGGGL........
..LGL...LGL...LGL....~~~~~
...LGGGGGGGGGGGL.....~~~~~
```

Fits: the cockpit header. Three rows. Typed by hand: three heads on one shared neck bar, colours from the sprite.

## splash-top-sprite

```
  ▄▄█▄▄█▄   ▄█▄▄█▄▄  █
 ████▀████ ████▀████▄█
 ▀███████▀ ▀███████▀██
   ▀████     ████▀   █
    ▀███     ███▀
     ▀██▄▄█▄▄██▀        hydra
```

```paint
....H..H.....H..H....H.......
..HHggHgH...HgHggHH..H.......
.HgGGgGGgH.HgGGgGGgH.H.......
.gGGGEGGGg.gGGGEGGGgHG.......
.gGGGGGGGg.gGGGGGGGgGG.......
..gGGGGGg...gGGGGGg.gG.......
...gGGGg.....gGGGg...g.......
....LGGL.....LGGL....L.......
....LGGL.....LGGL............
.....LGL.....LGL.............
.....LGL..y..LGL........~~~~~
......LGyygyyGL.........~~~~~
```

Fits: the README top. Six rows. Made by `from_sprite.py`: the sprite's heads and necks (rows 0 to 12) shrunk to 22 by 12 pixels. The third head is the sliver the 24 pixel sprite has at its right edge.

## splash-top-small

```
 ▄█▄█▄ ▄█▄█▄ █
 ██▄██ ██▄████
  ███   ███  ▀
   ▀█▄█▄██      hydra
```

```paint
..H.H...H.H..H.......
.HGgGH.HgggH.H.......
.gGEGG.gGEGGgG.......
.gGGGg.gGGGGgG.......
..gGG...gGG..g.......
..LGG...LGL..........
...LG.y.LG......~~~~~
....LygyGL......~~~~~
```

Fits: the README top when the mark should be quiet. Four rows. Made by `from_sprite.py`: the same crop shrunk to 14 by 8 pixels.


## Round one: the human passed on the ASCII ones

Six drafts from build 153. The four `line-` and `head-` ones are plain ASCII and were passed on.
`splash-blocks` is the one liked and the bar to beat. `splash-box` is kept for the record.
The limits for these: `line-` one line, 24 columns, ASCII. `head-` three lines, 32 columns,
ASCII. `splash-` three to five lines, 40 columns, box and block characters.

## line-eyes

```
(o)(o)(o) hydra
```

Fits: a status line or a shell prompt. Three round heads, 15 columns.

## line-fangs

```
>o >o >o ~ hydra
```

Fits: a prompt or a tab title. The heads face right like the sprite's jaws, 16 columns.

## head-necks

```
 <o  <o  <o
  \  |  /
   \_|_/  hydra
```

Fits: the cockpit header. Three necks join one body, the word sits at the base. 20 columns.

## head-crest

```
  __    __    __
 (o >) (o >) (o >)
   \___ | ___/   hydra
```

Fits: the cockpit header when there is room for bigger heads. 26 columns.

## splash-blocks

```
 ▄▀▄  ▄▀▄  ▄▀▄
 █●█  █●█  █●█
  ▀█▄ ▐█▌ ▄█▀   hydra
```

Fits: the README top and a splash. Block heads with an eye each, like the HD skin's silhouette.

## splash-box

```
 ┌o┐ ┌o┐ ┌o┐
 └╲┘ └│┘ └╱┘
   ╲╲ │ ╱╱
    ╲╲│╱╱
     ╘╧╛   hydra
```

Fits: the README top when the terminal has no block glyphs worth trusting. Box lines only.
