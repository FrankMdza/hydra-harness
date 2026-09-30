# 152 - The mascot in the cockpit, one head awake per live seat

Type: build
Status: open
Blocked by: 150

## What

The human asked on 2026-09-30 that the cockpit of map ticket 86 carry the mascot. The mascot is
`integrations/herdr/bin/hydra-mascot` and the vendored `integrations/herdr/mascot/hydrapet.py`,
Python with numpy: the HydraPet dragon the human picked on 2026-09-24, three heads that sway,
sleep with a bubble, derp and breathe fire, in two skins (the 24-pixel pet and the HD 160-pixel
canvas in half blocks, 48 columns). `MASCOTS.md` lists every pose and how to see it. The watch
pane already wakes one head per live pane, up to three, then shows `+N`.

The cockpit is Rust. Porting the renderer is not the job. The job is to show the same dragon in
the cockpit's header and make its heads follow the seats.

## Do

1. **Frames, not a port.** `hydra-mascot --save` already writes a frame as ANSI. Add to
   `hydra-mascot` a `--frames <dir>` mode that writes every pose of the pet skin (idle, working,
   one head asleep, two asleep, all asleep, refused, meme, fire), each as a small file of ANSI
   half-block rows at the pet's 24-column size, plus an `index.json` naming the pose of each
   file and its rows and columns. Python, in the mascot's own file; a test on a scratch dir
   checks the index and that every file has the rows it says.
2. **A Ratatui widget** in `src/cockpit/mascot.rs` that loads those frames from a directory
   (default `$HYDRA_HOME/integrations/herdr/mascot/frames/`, generated and committed, with the
   generating command in the README so a change to the dragon regenerates them), parses the ANSI
   into styled cells (an `ansi-to-tui` crate or a small parser of the half-block sequences the
   mascot emits; measure which is smaller), and draws the pose for the current state.
3. **State from the seats.** Heads awake equal the number of live seats of the session, up to
   three; more than three shows `+N` beside the dragon, as the watch pane does. A seat that just
   ended `gated` flashes the refused pose once; a seat that ended not ok shows the meme head for
   one refresh; a seat request launched breathes fire once. Idle when nothing is live.
4. **Placement.** The header, left of the title, at the pet size. A `--no-mascot` flag hides it
   and gives the row back to the table.

## Done when

- The Python test for `--frames` and a Rust test in `tests/cockpit.rs` that loads the committed
  frames, maps each seat state to a pose, and renders into a `Buffer` of the right size.
- The committed frames render in a real terminal if you can drive one; if not, say so.
- fmt, clippy, `cargo test --test cockpit` and the Herdr suite clean.
