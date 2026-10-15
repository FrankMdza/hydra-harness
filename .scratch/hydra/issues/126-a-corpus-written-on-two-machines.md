# 126 - A corpus written on two machines: may the reader take the layout off the record, and does Rungs split by enforcement population

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 119's answer, question 4. Grilling: the human
answers, the session only asks and brings what it measured.

## Question

Map 119 settled that the VM's records live in their own root, read with `HYDRA_ROOT`, and that
`run_start.host` (build 316) says which machine wrote a record. Two things that answer leaves
open, both found by the 119 panel when it asked what happens to a Linux record read on the Mac:

1. `gate.py`'s path check runs `os.path.realpath`, and on macOS `/home/ubuntu/x` becomes
   `/System/Volumes/Data/home/ubuntu/x`, so a replay of a Linux record on the laptop either fails
   the path check (Gemini's reading) or is filed `Foreign` by `GateMark::of` and never
   re-evaluated (Opus's reading); which one depends on whether `gate_sha` matches. A pooled corpus
   needs the reader to take the layout (root, home, the deny list's absolute paths) off the record
   it is reading, and contract 7.7 forbids that today: the reader compiles the gate from its own
   layout so a record cannot lie about what it was gated by. Does the contract get a carve-out
   for records whose `host` is not the reader's, and what does the reader do with them instead:
   replay with the record's layout, mark `Foreign` and count them, or refuse to walk them?
2. Rungs reads the corpus for `hydra curate` and `hydra evolve`. Until bubblewrap wraps a Linux
   launch (build 318), a Linux record is hook-only evidence. Does Rungs split its evidence by
   enforcement population (`sandbox.kind`), weigh hook-only records less, or exclude them from
   proposals that touch the gate?

The session's own view, which is not an answer: `Foreign` and counted for question 1, excluded
from gate proposals for question 2, until a Linux wrapper exists.

## Answer
