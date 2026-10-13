# 313 - What `IMPLEMENTATION.md` says that the code no longer does

Type: build
Status: claimed
Blocked by:
Opened: 2026-10-13, by the session. `IMPLEMENTATION.md` is the file `CLAUDE.md` tells every
session to read to know what is real, and on 2026-10-13 one of its sentences said `sheet build`
was still Python when build 219 ported it on 2026-10-01. Six thousand lines, amended at every
ticket close and never reread whole; the next stale sentence costs a session a wrong decision.

## What

A read-only pass over `IMPLEMENTATION.md`, checking every claim of the form "X is not built",
"X is not ported", "X has no caller", "no command reaches X", "nothing checks X", "X is still
Y", "the only place X" against the code as it is on this commit. The deliverable is a list, not
edits: each stale sentence quoted, the line number, what the code does instead with the file
and function that proves it, and a replacement sentence in the file's voice. The session applies
them; this ticket does not touch `IMPLEMENTATION.md`, because a reviewer that edits the thing it
reviews leaves no trail.

## Do

1. Read `CLAUDE.md` and then `IMPLEMENTATION.md` whole, in order. For every sentence that makes
   a checkable claim about the code (the forms above, plus any count: "twenty-seven files",
   "ten outcomes", "76 test binaries"), check it: `grep`, `ls`, read the function. Do not check
   measurements (times, token counts, dates); those were true on their day.
2. Write the list to `.scratch/hydra/research/313-stale-claims.md` with the Write or edit
   tool, not a shell heredoc. One entry per stale claim: line, the sentence quoted, the
   evidence (file and function or command and its output), the replacement. Order by line.
   Then a second list, shorter: claims you could not check in the time you had, with why.
3. Say at the end how many sentences you checked, how many were stale, and the three sections
   with the most stale sentences.

## Done means

The file above, and nothing else changed. No build, no test run needed; this is a reading.

## Outcome
