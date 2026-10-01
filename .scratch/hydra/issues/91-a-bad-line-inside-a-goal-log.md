# 91 - A bad line inside a goal log

Type: task
Status: resolved
Blocked by:
Opened: 2026-09-30, by the session, from build ticket 193. The run that ported the read side of
`puppets.py` to Rust found that the ticket's premise ("a bad line is skipped as `read_log` does")
is half false, copied what the Python does, and said the rest is a decision.

## Question

A goal log is one JSONL file per goal, append-only, written under an `flock`. Three pieces of code
read it, and they disagree about a line that is not JSON (or is blank) and is not the last line:

- `read_log` (`integrations/puppets/puppets.py:295`) raises `JSONDecodeError`. `show`, `board`,
  `pending`, `queue` and `fold` all read through it, so each dies with a traceback and exit 1. A
  blank line counts as bad here. Only a bad last line is skipped.
- `_locked_update` (`puppets.py:332`) skips any bad or blank line and appends. `seq` is the count
  of good lines plus one, so the writer keeps writing to a log the readers cannot open.
- The cockpit's fold (`src/cockpit/goal.rs:109`) skips the line, counts it, and prints
  `(N line(s) skipped)` in the goal's heading.

`crates/puppets` (build 193) copies the first two byte for byte, because map 90 asked for
identical output. So today a goal with one bad interior line shows in the cockpit with a count,
accepts new approvals and launches from the writer, and crashes every read command.

How a bad interior line gets there: not from a torn write, since a writer refuses to append after
an unterminated tail (`UnterminatedLog`) and `repair` moves the tail aside. It takes a hand edit,
a tool that wrote the file without the lock, or disk damage. The one goal log on this machine has none on
2026-09-30 (the real-log differential of build 193 read it).

Options:

- A. Leave it. Readers crash, the writer skips, the cockpit counts. The Rust keeps copying the
  Python through the cutover (build 196).
- B. Readers skip a bad interior line, as the writer and the cockpit do, and say so on stderr
  (`goal log <id>: N line(s) skipped`) with exit 0. One behaviour in three places.
- C. Nobody skips. Readers fail as today, and the writer also refuses to append while an interior
  line is bad, with a named error like `UnterminatedLog`. `repair` learns to move bad lines into
  the `.torn-` file. The cockpit shows the goal as damaged rather than folding around the hole.
- D. B for the readers, C for the writer: reading degrades with a warning, writing stops until
  `repair` has run.

Whichever is picked lands after build 196, in the Rust only, so the differential tests of 193 to
195 keep comparing against the Python as it is.

## Answer

Resolved 2026-09-30 by a panel of three under the human's delegation. Unanimous on both
questions the brief put: what a bad interior line does, **C**; when it lands, **now, in the
Python and the Rust, before build 194**. codex 0.87 and 0.76, cursor-agent grok-4.7-high 0.64 and
0.56, agy gemini-3.1-pro-high 0.85 and 0.90; all three named D second on the first and "after the
cutover" second on the second. Answers in the human's decision log,
`decisiones/2026-09-30/answer-91-*.md`.

Nobody skips. A newline-terminated line that is blank or is not JSON makes the log damaged:
readers fail with a named error and exit 1 instead of a traceback, every writer under the lock
refuses to append, `repair` moves the bad lines into the `.torn-` file with the torn tail, and the
cockpit shows the goal as damaged. The reason all three gave: a skipped line can be a `task` or an
`assignment`, the reader then computes a lower revision than the writer of that line meant, and
an approval sticks to a table the human did not see. The session checked that against the code:
`cmd_approve` folds a second time under the lock from `_locked_update`'s lines, bad ones skipped
(`puppets.py:2083`). Its first fold goes through `read_log` and crashes, so `approve` is blocked
today by accident; `append_event` callers that do not fold first are not.

It lands before 194 because 194 ports the writers: changing the Python now lets that ticket copy
refuse-and-repair once, with the differential test still comparing byte for byte. After the
cutover the Python is gone and nothing checks the new rule against anything.

Two things the session added, because the panel's option leaves them open, and the human can
undo either:

- A bad last line counts as damage too. Today the reader skips a newline-terminated bad last
  line; a writer that appended after it would turn it into a bad interior line.
- codex's condition: `repair` must not make an old approval valid by making the log readable.
  After moving interior lines aside, `repair --yes` appends a `repair` event carrying how many
  lines it removed, and the fold adds that count plus one to the revision there, so the revision
  ends above anything an earlier approval named and every table, launch, diff and degraded
  approval before it is void. Plan approvals do not carry a table revision and stay.

What reverses it, from the seats: one real log whose only fault is a blank interior line and
where the hard stop blocks recovery (cursor, then D); a parser that calls good lines bad more than
1 time in 100 (agy); the change proving too large for one review before 194 (codex, cursor).

Carried by build 197 (the Python, the crate, `repair`) and build 198 (the cockpit). 194 waits on
197.
