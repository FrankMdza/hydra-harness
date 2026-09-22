# 60 - Row 4 of harness 15.2 is not a file, and protecting it means cutting `run.rs`

Type: grilling
Status: resolved
Blocked by:

## Question

Build ticket 40 made admission refuse the code rows of harness spec 15.2, and found that one of
them has no file to name. Row 4, "the hook-config writer", is two things: `src/policy/` compiles
the gate and the hook command, and `write_spawn_config` in `src/run.rs` writes the
`settings.json` the vendor reads. `src/policy/` is now a protected row. `src/run.rs` is not,
because it is also the whole launch sequence, and refusing patches to it would refuse most of
what rung two could usefully change.

So today a rung-two patch can edit `write_spawn_config` and the panel is what stands between
that edit and master, which is the situation ticket 40 was opened to end for the other rows.

1. **Does the writer move out of `run.rs` into its own file, so it can be a row?** A mechanical
   split with one function and its tests. The cost is that the launch sequence, which
   `CLAUDE.md` names as the place to start reading, gains one more hop.
2. **Or does `run.rs` become a row as it is?** Rung two then cannot touch the launch, and the
   set of things it may propose shrinks to the normalizers, the commands typed by humans,
   and the integrations. That may be the right size for a loop that has approved nothing yet.
3. **Or does the panel stay the guard for this one row**, with the judge item that names it
   marked critical, and the spec's table amended to say so? Honest, and it is the one row
   where the design's "refused at admission, not by the panel" line is bent on purpose.

The build ticket's Outcome and `IMPLEMENTATION.md` record the gap. Nothing chooses until
this is answered.

## Answer

Resolved 2026-09-21 with the human, after a research run on `cursor-agent` (01a0c5ab-7364). The
writer moves out: `write_spawn_config`, `SpawnFiles` and their tests go to `src/spawn_config.rs`,
re-exported from `run.rs` so the call sites keep their path, and the new file joins
`PROTECTED_CODE_PATHS`. `run.rs` stays editable by rung two, which build ticket 40 measured as the
right size for the loop, and the panel stops being the only guard of row 4, because spec 4.4's
"refused at admission, not by the panel" is load-bearing and a judge item is probabilistic where
`protected_path_hit` is not. The research found harness spec 15.2 claiming row 4 refused at
admission while the code lists only `src/policy/`, and the `PROTECTED_CODE_PATHS` comment mapping
`src/policy/` to rows 4 and 9 both; the build fixes the code and the session fixes the spec. Build
ticket 66.
