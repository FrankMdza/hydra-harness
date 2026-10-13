# 313 - What `IMPLEMENTATION.md` says that the code no longer does

Type: build
Status: resolved
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

Read by `codex` `gpt-6-astra` on Hydra run `01a13718-b0ec-7823-8f98-b3369ac9149f` under
`standard`, read-only as asked, 2026-10-13. It checked 84 sentences and listed 57 as stale, each
with line, quote, evidence and a replacement, in `research/313-stale-claims.md`, plus 27 it
checked and kept and seven groups it could not check from a worktree (live corpus facts,
measurements, the port equivalence claims).

The session applied 45 of the 57 to `IMPLEMENTATION.md`, each after checking the evidence
itself with `grep` or a read on master: 1, 2, 5 to 11, 13 to 18, 22 to 24, 26 to 29, 32 to 47,
49 to 51, 54 to 57. The paragraphs that changed were reflowed to 95 columns, which is why the
diff is 424 lines for 45 sentences. Ten were left as the file had them because the session did
not confirm the evidence in the time it had: 3 and 4 (the counts of `protected_worktree_paths`
and `write_denied_worktree_paths`), 12 and 19 (row counts in `src/policy/table.rs`), 21 (how
`gate.py` keys the dialect; the file reads `argv[1]`, the memo says `HYDRA_CONFIG`), 25
(`judge` skipping skill injection), 30 (`gate_sha`'s `skip_serializing_if`), 31 (the count of
`Finding` variants), 52 and 53 (whether the run id is minted before the cap and the `PATH`
check). Two the session confirmed but the script could not find in the file as quoted, 20 and
48; they stay stale and are the first two lines of work for whoever reads this next.

What the memo found that is not a sentence: `config/mcp.md`'s own prose (lines 41 and 42)
still says `mcp_call` is granted by `permissive` alone, while `policies/standard.json` line 11
grants it to the `hydra` server. That file is protected config, so the human changes it.

The three sections with the most stale sentences were the policy set and the gate (6), reading
a record back (5) and rung two (5). The pattern behind most of them is the same: a later
ticket changed the code and amended the paragraph it was told to amend, and an earlier
paragraph that stated the old fact in passing stayed. The file's size is the cost; a reread
whole once a month is cheaper than the decisions a stale sentence costs.
