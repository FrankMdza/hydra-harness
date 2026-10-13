# 120, seat Opus: the floor and the run's own worktree path

Read on 2026-10-13 from run `01a13730`, a Hydra self-edit worktree under `standard` on `claude`.
Every measurement below went through this run's own compiled gate, which is live: a control
command, `echo $(echo probe) git`, came back `hydra/floor`. I could not read run `01a1332c`'s
record, because it lives under `~/hydra/runs/` and the policy refuses that, so what I say about
that run is inference until someone reads its refusal string.

The short version: the premise of question 1 does not hold in this code. The gate already carves
the run's own worktree out of both shell passes, and I measured it allowing an absolute path into
this worktree. What it refuses is a path *inside* the worktree that resolves out of it, and the one
that every run carries is a linked skill. The refusal then names `/users/user/hydra`, which reads
exactly like "your worktree is under a protected directory". I think that is what happened to the
Cerebro run, and a test that claims otherwise passes for the wrong reason.

## 1. Should the floor carve out the run's own worktree?

**What the code does.** The carve-out exists, in both passes of the shell check.

- The raw substring pass, `src/policy/gate.py:336`, searches the command for `denied_spellings`.
  `src/policy/mod.rs:964` builds that list and leaves off every denied path that holds an own path,
  so `~/hydra/worktrees`, `~/hydra` itself and the parent `.git` are not on it.
- The token pass resolves each argument with `realpath` and asks `protected_root`,
  `src/policy/gate.py:242`, which returns None for anything inside `own_write_paths`. That list is
  `src/policy/mod.rs:867`, and the run's worktree is its first entry.
- `tests/gate.rs:2578`, `the_raw_pass_leaves_an_ordinary_command_alone`, pins `ls -la <worktree>/src`
  and `rg TODO <worktree>` as allowed.

**What I measured.** On this run's live gate:

| Command | Answer |
|---|---|
| `ls /Users/USER/hydra/worktrees/01a13730-…/src` | allowed |
| `ls ~/hydra/worktrees/01a13730-…/docs` | allowed |
| `cat /Users/USER/hydra/worktrees/01a13730-…/../01a13730-…/Cargo.toml` | allowed |
| `rg -l … -g '<absolute own glob>' <absolute own path>` | allowed |
| `head -2 .claude/skills/hydra-self-edit/SKILL.md` | `hydra/protected-path: the command names /users/user/hydra` |
| `head -2 /Users/USER/hydra/worktrees/01a13730-…/.claude/skills/hydra-self-edit/SKILL.md` | same refusal |
| `Read` tool on that same absolute path | allowed, the file came back |

`.claude/skills/hydra-self-edit` is a link into `~/hydra/skills-snapshots/870b034…/`. `realpath`
follows it, the result is inside `~/hydra` and outside every own path, and `protected_root` returns
the first deny entry that matches. `denied_write_paths` lists `hydra_home()` at
`src/policy/mod.rs:817`, before the snapshots at 839 and the worktrees parent at 850, and on this
machine the checkout and the runtime root are the same directory. So every refusal under `~/hydra`
names `/users/user/hydra`, whichever child it really hit.

**The test that hides it.** `tests/gate.rs:2601`,
`a_shell_command_naming_the_skill_root_is_how_two_providers_read_a_skill`, asserts that
`cat .claude/skills/unslop/SKILL.md` is allowed under `standard`. It never calls `link_skill`, so
there is no link, `realpath` stays inside the worktree, and the assertion holds in the test and
fails on every real run that has a skill linked. Same for `tests/gate.rs:2543`. Harness spec 7.5
says "a shell command naming the root is how `cursor-agent` and `agy` read a skill". The root, yes:
`ls -la .claude/skills/` passed here. A file under a linked skill, no. Ticket 71 met this on `codex`
and answered it with a copy for `codex` only; the other three providers still hit it whenever the
model reaches for `cat` or `head` instead of its read tool.

**What I believe happened to `01a1332c`.** A Cerebro ticket that names a skill, a model that opens
it by shell with the absolute path it was shown, and a refusal naming `/users/user/hydra` that the
model and the session both read as "the worktree is protected". I have not seen the record. The
refusal string on it settles this in one read: if it names `/users/user/hydra` and the command
went through `.claude/skills/`, `.cursor/skills/` or `.agents/skills/`, this is the cause. If it
names `/users/user/hydra/worktrees` or the run's own path, I am wrong and the carve-out has a bug
I could not reproduce.

**Recommendation.** Do not add a carve-out; it is there and it works. Widening it as the ticket
sketches, one prefix compare before the pattern match, would be worse than nothing: a command that
names the own worktree and also names `~/hydra/runs/` inside a `sh -c` token would skip the raw
pass that is the only thing catching the second. The decision the ticket should ask instead is
what a shell read of the run's own pinned snapshot gets, and I would answer it by adding the run's
own `skill_snapshot` directory to the token pass's own list, on a launch where the seatbelt is on.
On macOS the profile already denies every write under `skills-snapshots/`, so the gate stops
refusing a read the kernel would allow and a write the kernel would refuse anyway. Whatever is
decided, fix the two tests to link the skill first, so they say what production does.

**What would change my mind.** The record of `01a1332c` naming the worktree path itself. Or a
measurement that some provider writes through a shell spelling the seatbelt does not see, which
would make the snapshot carve-out a write path rather than a read one.

## 2. If not a carve-out, what should the refusal say?

**What the code does.** `src/policy/gate.py:1139` prints "the command names {protected}", where
`{protected}` is the first deny entry that matched, not the most specific one, and never the token
that resolved to it. For a link it does not say that resolution went through a link.

**What I measured.** All three spellings above, relative, absolute and through a link, print the
same `/users/user/hydra`. A model reading that cannot tell a sibling worktree from its own skill.

**Recommendation.** Two small changes, neither a decision: have `protected_root` return the longest
matching deny entry rather than the first, so the message says `skills-snapshots`; and when the
token resolved through a link, print both, "`.claude/skills/x/SKILL.md` resolves to
`~/hydra/skills-snapshots/<sha>/x/SKILL.md`, read it with your read tool". The ticket's proposed
text, "your own worktree is under a protected directory", would be false in this code, so I would
not ship it. The one place the own-worktree wording would be true is a run with no checkout, whose
worktree scope is `<spawn>/no-worktree` and is not an own path, `src/policy/mod.rs:874`.

**What would change my mind.** A refusal record where a provider did not recover from a specific
message either. Then the message is not the cost and the snapshot question in 1 is.

## 3. Does this change the seatbelt profile?

**What the code does.** The gate and the profile read the same list. `src/sandbox.rs:204` takes
`policy::own_write_paths(spawn)` and emits `(allow file-write* (subpath <own>))` after the
`write_denies`, which come from the same `denied_write_paths`, `src/sandbox.rs:432`. So on the
run's own worktree they agree, by absolute path and every other spelling: I wrote nothing outside,
but every absolute own-path command above was allowed by both.

**Where they disagree.** On reads of the snapshot. The profile denies reads under the corpus, the
spawn parent, `pruned/` and the approvals, `src/sandbox.rs:117` to 139, and leaves the snapshots
readable; the gate's shell check refuses the same read whole. The floor wins, as the ticket says,
but over a snapshot path, not over the worktree.

**Recommendation.** No profile change for question 1. If the snapshot carve-out from 1 goes in, the
profile is what makes it safe, so the carve-out should be compiled only when `run_start.sandbox` is
not null, and Linux keeps today's refusal.

**What would change my mind.** A measured write into `skills-snapshots/` from inside the profile,
by any spelling. That would end the carve-out before it starts.

## 4. Is Cerebro special?

**What the code does.** `Layout::worktree`, `src/paths.rs:148`, puts every run's checkout under
`<root>/worktrees/<run-id>` whatever the repository, and `skills::inject` links skills into it for
every repository.

**What I measured.** The same refusal in this run, a Hydra self-edit, in two commands. So no,
Cerebro is not special, and the ticket's expectation is right for a different reason than it gives.

**The attack corpus.** No class in `tests/corpus/README.md` relies on naming the run's own worktree
absolutely. One recorded attack relies on the carve-out itself: H3 in
`.scratch/hydra/research/68-attack-corpus-opus.md:185`, pinned as a residual at `tests/gate.rs:2008`,
is `sh -c 'cat <root>/worktrees/../runs/x.jsonl'`. It walks past the raw pass because
`<root>/worktrees` cannot be on the substring list once the own worktree lives inside it. Any
widening of the own carve-out has to leave that residual no wider, which is the argument against the
string-prefix version in 1.

**Recommendation.** Treat it as every run's problem, and close the ticket's Cerebro framing.

**What would change my mind.** A Cerebro record whose refused command names no linked skill.

## 5. Heredoc bodies that name git

**What the code does.** `floor_violation`, `src/policy/gate.py:585`, refuses a command that holds
`$(`, `${` or a backquote anywhere and the substring `git`, `aws` or `slack.com` anywhere,
`FLOOR_TARGETS` at 435. It reads no quotes and no heredoc delimiter. The split into commands,
`split_segments` at 442, lists `"\n"` among its operators at 450, but shlex treats a newline as
whitespace and never yields it as a token.

**What I measured.** Harmless probes on the live gate, each writing to `/dev/null`:

| Probe | Answer |
|---|---|
| `<<'EOF'` body "the git log" | allowed |
| `<<'EOF'` body with `` `git status` `` in backquotes | refused, substitution |
| `<<'EOF'` body "an aws bill costs ${price}" | refused, substitution |
| `<<'EOF'` body line starting `git reset --hard` | allowed |
| `echo "${PWD}" > /dev/null; ls .gitignore` | refused, `.gitignore` counts as naming git |
| `exit 0; git gc` | refused, floor |
| `exit 0` newline `git gc` | **allowed** |

The GPT seat's memo almost certainly died on the second row: a markdown memo has backquotes, and a
quoted delimiter makes them literal to bash but not to the floor. The last row is a hole in the
floor beside this question, not inside it. `shlex` folds the newline into whitespace, so a second
line is read as arguments of the first line's binary, and `git push --force` on line two of any
command passes the floor. The shell did not run it here because `exit 0` came first. On `claude`
and `cursor-agent` the user's guards may still catch it; on `agy` nothing does.

**Recommendation.** Yes, distinguish data from the command line, but only where bash does: the
body of a heredoc with a quoted delimiter, `<<'EOF'`, `<<"EOF"` or `<<\EOF`, and anything inside
single quotes, are literal, and the substitution rule should skip them the way `shell_hazard` at
`src/policy/gate.py:639` already walks quote state. An unquoted `<<EOF` body expands `$(…)` and
stays refused. Fix the newline split in the same change, and the two are coupled: once a newline
separates commands, every heredoc body line that starts with `git` becomes a command to the floor,
so the floor has to know where a quoted heredoc body ends before the newline fix can ship. Meanwhile
the task strings already say "use the Write tool", and that is the cheap answer for memos.

**What would change my mind.** A provider whose shell tool hands the gate something other than the
literal command bash runs, for instance a wrapper that re-evaluates the string. Then quote state at
the gate means nothing and the coarse rule is right.

## Recommendation summary

1. No new carve-out: the own worktree is already allowed and measured so; the refusal comes from a linked skill resolving into `skills-snapshots/`, so read `01a1332c`'s refusal string, carve the run's own snapshot out of the token pass on sandboxed launches, and fix `tests/gate.rs:2601` and 2543 to link the skill.
2. Name the longest matching deny entry and the link it went through, and drop the proposed "your own worktree" wording, which this code would make false.
3. No profile change; gate and seatbelt share `own_write_paths` and agree on the worktree, and the seatbelt's write deny on snapshots is what makes the snapshot carve-out safe, so compile it only when `sandbox` is set.
4. Not special to Cerebro: measured in this Hydra self-edit run; no corpus attack names the own worktree, and H3 is why a string-prefix carve-out must not ship.
5. Exempt quoted-delimiter heredoc bodies and single-quoted text from the substitution rule, and fix the measured newline hole in the floor in the same change, because the two depend on each other.
