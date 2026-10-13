# Map 120, the session's synthesis for the human

Two seats sat on 2026-10-13: Opus 5.5 on `claude` (`120-seat-opus.md`) and Gemini 3.8 Flash on
`agy` (`120-seat-gemini.md`). The GPT seat on `codex` died at its first turn: "You've hit your
usage limit", the codex quota exhausted at 21:59Z after the day's builds. No Grok seat while
Cursor is on the free plan. So this is a two-seat panel and says so. Both seats measured on
their own live gate from inside a Hydra worktree, which is the right place to measure this
question, and the session checked their claims against the repo and read the record the ticket
is about, which neither seat could.

## A finding before the questions: the ticket's premise is wrong, and so is Opus's replacement

The ticket says run `01a1332c` was refused for naming the absolute path of its own worktree.
Opus found that the carve-out for the own worktree already exists in both passes of the shell
check (`src/policy/mod.rs` line 964 leaves every denied path that holds an own path off the raw
substring list; `gate.py`'s `protected_root` returns None inside `own_write_paths`) and measured
four absolute own-path commands allowed on its live gate. Gemini measured the same: `rg` and
`git status` with the absolute worktree path, allowed. The session confirms the code reading. The
premise does not hold.

Opus then guessed what did happen: a shell read of a linked skill under `.claude/skills/`, whose
`realpath` lands in `~/hydra/skills-snapshots/` and is refused as `/users/user/hydra`. That is a
real refusal, measured twice in its run, and the test that would catch it (`tests/gate.rs` line
2601) never links the skill, so it passes for the wrong reason. But it is not what happened to
`01a1332c`. The session read the record: the refused command at seq 19 is
`mkdir -p src/sources; cat > src/sources/mod.rs <<'EOF'` and then the body, which codex's log
truncates at 63 characters, so the body itself is not on the record. The file being written was
Cerebro's reader of Hydra's run records, and Cerebro's ticket 03 says in its own words that "the
policy refuses a shell command that names `~/hydra/runs/`". So the refusal was almost certainly
the raw substring pass on a heredoc body that spelled `~/hydra/runs`: data, not a command, the
class question 5 is about. Nobody can prove the body's contents from what was recorded, and that
is a finding of its own: a refusal record that does not carry the command it refused.

## Where the two agree

- **Question 1.** No new carve-out for the own worktree is needed; it exists. Gemini still
  recommends one "with a trailing separator" for ancestor paths and globs, but what it measured
  refused (`ls /Users/USER/hydra/worktrees`) names the parent, which the design denies on
  purpose. Opus adds the reason a string-prefix carve-out must not ship: attack H3 of map 68
  (`sh -c 'cat <root>/worktrees/../runs/x.jsonl'`, `tests/gate.rs` line 2008) walks past the raw
  pass only because the worktrees parent is already off the substring list; anything wider
  reopens it.
- **Question 2.** The refusal names the first deny entry that matched, which on this machine is
  always `/users/user/hydra` because `hydra_home()` is first in `denied_write_paths` and the
  checkout is the runtime root. Both seats want a message that names the cause. They disagree on
  the words: Gemini's "your command names an absolute path under a protected directory; use a
  relative path" would have been false for this record and for the skill case; Opus's "name the
  longest matching deny entry, and the token that resolved to it, and the link it went through"
  is true in every case measured. Recommendation: Opus's.
- **Question 3.** No seatbelt change. Gate and profile read the same `own_write_paths`; the
  profile allows writes under the own worktree by absolute path (`src/sandbox.rs` line 204) and
  the two agree there. Where they disagree is reads of the pinned snapshot: the kernel allows,
  the gate refuses. That is a separate, smaller decision (below).
- **Question 4.** Cerebro is not special. Every run's checkout is under `~/hydra/worktrees/`
  whatever the repository (`src/paths.rs`), and the attack corpus has no class that relies on the
  own worktree path (Gemini counted 14 classes, 0 of them). Opus measured the same refusal class
  in a Hydra self-edit run.
- **Question 5.** Yes, distinguish the command line from the data it feeds, but only where bash
  does: the body of a heredoc with a quoted delimiter and anything in single quotes are literal,
  and the substitution rule should skip them; an unquoted `<<EOF` body expands and stays
  refused. The same applies to the protected-path substring pass, which is what refused
  `01a1332c`.

## Where they split, and the recommendation

1. **A hole beside question 5, measured by Opus and not seen by Gemini.** `split_segments`
   lists `"\n"` as an operator, but `shlex` treats a newline as whitespace and never yields it,
   so `exit 0` newline `git gc` passed Opus's live gate while `exit 0; git gc` was refused. A
   second line starting with `git push --force` passes the floor on any provider. Gemini read
   the same code and concluded the opposite, that the newline splits commands and so heredoc
   lines starting with `git` are checked as commands; it did not measure. Opus's measurement
   wins. Recommendation: fix the newline split and the quoted-heredoc exemption in one change,
   because once newlines separate commands every heredoc body line starting with `git` becomes
   a command to the floor unless the floor already knows where the quoted body ends.
2. **The snapshot read.** Only Opus raises it: a shell `cat` of a linked skill is refused on
   every run that has skills linked, on three of four providers (`codex` gets a copy since map
   71). The spec's sentence "a shell command naming the root is how `cursor-agent` and `agy`
   read a skill" is true of `ls` on the root and false of `cat` on a file in it. Opus proposes
   adding the run's own `skill_snapshot` directory to the token pass's own list, compiled only
   when `run_start.sandbox` is set, since the profile denies writes there. That is a design
   change to what `standard` lets a shell read, so the recommendation is to put it to the human
   as question 6 rather than fold it into question 1: the two seats never got to vote on it.

Fable was not called as judge: the only disagreement with weight (the newline) has a measurement
on one side and a reading on the other.

## What would become tickets if the human agrees

- Build: `gate.py` skips quoted-delimiter heredoc bodies and single-quoted text in both the
  substitution rule and the protected-path substring pass, and splits segments on a real
  newline; `tests/gate.rs` gains Opus's seven probes as cases, the `exit 0` newline `git gc`
  one first.
- Build: the refusal names the longest matching deny entry, the token that resolved to it and
  the link it went through; the two skill tests at `tests/gate.rs` lines 2543 and 2601 link the
  skill before asserting.
- Build: a refused shell command is recorded whole on the record, not as the provider's
  truncated log line; today `01a1332c` cannot say what it was refused for.
- Map, question 6: whether `standard` lets a shell read the run's own pinned snapshot on a
  sandboxed launch.
- Nothing for question 3, and question 4 closes as "every run".
