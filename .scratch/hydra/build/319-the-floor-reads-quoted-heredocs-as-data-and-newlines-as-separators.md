# 319 - The floor reads a quoted heredoc body as data, single quotes as data, and a newline as a command separator

Type: build
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from map 120's answer, question 5 and the hole Opus
measured. Two defects in the shell check, fixed in one change because each makes the other
worse alone. First, the substitution rule and the protected-path substring pass read a heredoc
body as if it were the command line: `cat <<'EOF'` whose body mentions `git` is refused, and a
body that spells `~/hydra/runs` is refused, which is what stopped run `01a1332c` (146k tokens,
nothing written) and cost the map 117 GPT seat its memo file. Second, `split_segments` lists
`"\n"` as an operator but `shlex` treats a newline as whitespace and never yields it, so `exit 0`
newline `git gc` passed a live gate while `exit 0; git gc` was refused: a second line starting
with `git push --force` passes the floor on any provider today.

## What

The gate script is compiled by `src/policy/` from `gate.py` (find the source under `src/policy/`
and the compiled copy a run gets under its spawn directory; `tests/gate.rs` compiles a real gate
per policy and feeds it synthetic hook payloads). Change the shell check so that:

1. A newline outside quotes separates commands, the same as `;`. Split on it before `shlex`
   sees the segment, or make the tokenizer yield it; either way `exit 0` newline `git gc` is two
   segments and the second is checked.
2. The body of a heredoc whose delimiter is quoted (`<<'EOF'`, `<<"EOF"`, `<<\EOF`) is data: the
   substitution rule and the protected-path substring pass skip it, from the line after the
   operator to the line that is exactly the delimiter. An unquoted `<<EOF` body expands and stays
   subject to both passes.
3. Text inside single quotes is data for the substitution rule and for the substring pass; double
   quotes are not, since they expand.
4. Everything the floor refuses today outside those three cases is still refused. Attack H3 of
   map 68 (`sh -c 'cat <root>/worktrees/../runs/x.jsonl'`, `tests/gate.rs` line 2008) must stay
   refused: it is a path argument, not quoted data in the sense above, so decide how the
   substring pass tells a quoted path argument from a quoted heredoc body and write the rule
   down in the gate's comments. If the only consistent rule is "single quotes exempt the
   substitution rule but not the protected-path pass", take that and say so in the Outcome.

## Do

1. Read `.scratch/hydra/research/120-seat-opus.md` for the seven probes Opus measured on a live
   gate, and `120-synthesis.md`. Each probe becomes a case in `tests/gate.rs`, the `exit 0`
   newline `git gc` one first, written so it fails before the fix.
2. Fix `gate.py` in the compiler's source, rebuild, run `tests/gate.rs` and the whole suite. The
   corpus under `tests/corpus/` must still refuse every attack it refuses today.
3. Measure on your own live gate (you are inside a run): a `cat <<'EOF'` whose body names git is
   allowed after your change only in your worktree's compiled copy, not in the gate that runs
   you, so measure by running the compiled gate script by hand against a payload, as
   `tests/gate.rs` does.

## Done means

The new cases green, the corpus unchanged in what it refuses, `cargo fmt --all --check`, `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --release --workspace`,
`cargo test --release --workspace --doc`. Amend `IMPLEMENTATION.md` where the pattern floor and
the substitution rule are described, and `docs/harness-spec.md` 7.4 if it states the old rule.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs (the gate that runs you still has the old rule), and
the test suite is green inside a run since build 302 without any `env -u`.

## Outcome

Resolved 2026-10-15 by Opus 5.5 on `claude`, inside a Hydra run under `standard`.

**Built.** `shell_layout` in `src/policy/gate.py`, one walk over the command with the quote
state kept. It knows single quotes, `$'...'` with its escapes, double quotes, backslashes, line
continuations, comments, here-strings and heredoc operators, and returns four things: the
command lines split at every newline outside quotes, the heredoc bodies with their spans, the
text the substitution rule reads, and the text the protected-path substring pass reads.
`split_segments` now runs shlex per line with `commenters = ""`, because the walk has already
removed comments and shlex read the `#` in `topic#x` as one. `floor_violation` and
`protected_path_named_by` read the layout. The rule is written in `shell_layout`'s docstring:

- A newline outside quotes separates commands.
- A marker inside single quotes is not a substitution. The floor target is still searched for
  in the whole command, because `$(echo 'git') push` reads its word out of single quotes.
- A heredoc body is data, skipped by both passes, when four things hold: its delimiter is
  quoted, the command it feeds is `cat` or `tee`, its line has no pipe, and the command holds no
  `$(`, `${`, backquote, `<(` or `>(` outside single quotes. A non-data body is read by the
  substitution rule, the raw pass and the token pass as before, but its lines are never floor
  segments, since they are stdin.
- When the walk cannot be sure it reads the command as bash will (`((`, an unbalanced quote, a
  delimiter holding `$` or a backquote), nothing is data and body lines are checked as command
  lines.
- Single quotes exempt the substitution rule and not the substring pass. That is the fallback
  item 4 offered, and I took it: `sh -c '<runs>/x'` hands its quoted word to a shell, and the
  gate cannot tell that from `echo '<runs>/x'` without knowing what every program does with an
  argument. A heredoc's consumer is named on its own line, so a body can be judged and a quoted
  argument cannot.

**Tests.** Five new cases in `tests/gate.rs`, after `the_floor_checks_every_segment_of_a_chained_command`:
the newline one first, the quoted bodies that are now data, what stays under the floor (12
commands), single-quoted text, and the `01a1332c` shape against six spellings that must stay
`hydra/protected-path`. All five failed before the fix and pass after. `tests/gate.rs` went
from 117 to 122 tests, all green, and the corpus refuses what it refused: no existing assertion
changed.

**Measured** by hand on one compiled `standard` gate, built into the scratch layout the test rig
uses, old template and new template on the same compiled config, `/usr/bin/python3` 3.9:

| Probe | Before | After |
|---|---|---|
| `exit 0` newline `git gc` | allow | `hydra/floor` |
| `exit 0; git gc` | `hydra/floor` | `hydra/floor` |
| `cat <<'EOF'` body "the git log" | allow | allow |
| body with backquoted `git status` | `hydra/floor` | allow |
| body "an aws bill costs ${price}" | `hydra/floor` | allow |
| body line `git reset --hard` | allow | allow |
| `echo "${USER}" > /dev/null; ls .gitignore` | `hydra/floor` | `hydra/floor` |
| `cat > src/sources/mod.rs <<'EOF'` body naming the runs dir | `hydra/protected-path` | allow |
| `sh -c 'cat <runs>/x.jsonl'` | `hydra/protected-path` | `hydra/protected-path` |
| `sh <<'EOF'` body `cat <runs>/x.jsonl` | `hydra/protected-path` | `hydra/protected-path` |
| `cat <<'EOF' \| sh` body `$(echo git) push --force` | `hydra/floor` | `hydra/floor` |
| `echo $((1<<'X'))` newline `git push --force` | `hydra/floor` | `hydra/floor` |
| `git push origin topic#x --force` | allow | `hydra/floor` |
| `if true; then git push --force; fi` | allow | allow |

Full suite: 1393 passed, 4 skipped under nextest.

**What the ticket assumed that was false.**

- Item 4 says attack H3, `sh -c 'cat <root>/worktrees/../runs/x.jsonl'`, "must stay refused".
  It is not refused today. `tests/gate.rs` pins it as an allowed residual ("if this now denies,
  the residual is closed"), because `<root>/worktrees` cannot be on the substring list while the
  run's own worktree sits under it. It stays allowed after 319, no wider.
- Item 2 says the body of any quoted heredoc is data. Taken literally that opens `sh <<'EOF'`
  and `cat <<'EOF' | sh` in one command, both refused today, so 319 narrows it to bodies fed to
  `cat` or `tee` with no pipe and no substitution. Both shapes that cost a run (`01a1332c`, the
  map 117 GPT memo) are `cat > file <<'EOF'` and pass.
- Opus's probe `echo "${PWD}" > /dev/null; ls .gitignore` comes back `hydra/protected-path`, not
  `hydra/floor`, when the suite runs inside a Hydra run: the gate expands `$PWD` from its own
  environment, and there PWD is a worktree the scratch gate does not own. The test uses
  `${USER}`.

**Found, not fixed.** A shell keyword in front of the binary hides it from the floor:
`if true; then git push --force; fi` reads as binary `then` and passed both templates. `do`,
`else`, `!` and `time` should behave the same by the code; I did not probe them. It is the same class as the newline hole and a few lines to fix, but
it is not this ticket's; it wants its own build ticket with its own probes. The other residual,
`cat > x.sh <<'EOF'` then `sh x.sh`, is the Write tool's residual and is written down in
`shell_layout` and `IMPLEMENTATION.md`.

`IMPLEMENTATION.md` is amended under the pattern floor and the raw pass. `docs/harness-spec.md`
7.4 did not state the old rule in full, so it gained two sentences rather than a correction.
