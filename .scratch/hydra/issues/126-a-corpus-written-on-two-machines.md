# 126 - A corpus written on two machines: may the reader take the layout off the record, and does Rungs split by enforcement population

Type: grilling
Status: resolved
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

Resolved 2026-10-15 by the human, who took the session's recommendation from the panel 130 synthesis as it stands ("me gustan tus recomendaciones", "sigue tus recomendaciones para el resto").

1. No carve-out in contract 7.7. A record is never replayed with its own layout, and never refused
   a walk. Once `run_start.host` exists (build 316), the walk tells "another host" from "another
   build": a record whose `host` is not the reader's is counted under its host and never
   re-evaluated, apart from `GateMark::Foreign`, which keeps meaning a gate compiled from another
   build on this machine. Grok's reason stands: `skill_snapshot` is always rebuilt from the
   reader's layout, so a cross-machine record lands `Foreign` every time and the `realpath` failure
   the ticket feared needs a matching sha it will never have.
2. "Exclude hook-only records from proposals that touch the gate" is an empty set: `hydra evolve`
   refuses any patch to `src/policy`, `policies`, `src/sandbox.rs` or `src/containment.rs` before
   review, and `curate` writes skills only. What is decided instead: Rungs refuses a corpus that
   mixes hosts at load, with a message naming both hosts, until a Linux wrapper exists (build 318).
   No weight for hook-only records; nobody can justify a number yet.

Both wait on 316's `host` and `sandbox_kind`. Tickets: build 336 (the walk's per-host count and
the mixed-corpus refusal, blocked by 316). Panel memos and synthesis under `research/130-*`.
