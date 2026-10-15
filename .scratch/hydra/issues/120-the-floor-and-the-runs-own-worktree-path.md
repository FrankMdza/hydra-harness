# 120 - The pattern floor refuses a run's own worktree path: a scope it cannot name

Type: grilling
Status: resolved
Blocked by:
Opened: 2026-10-13, by the session, from the human's note on Hydra's list of 2026-10-13 ("es
decisión de mapa, no de build"). Grilling: the human answers, the session only asks and brings
what it measured.

## Question

On 2026-10-13 a recorded run for another repository, `~/cerebro`, was refused by the gate for
naming the absolute path of its own worktree, `/Users/USER/hydra/worktrees/<run id>/...`, in a
shell command (run `01a1332c`, 146k tokens, nothing written). The pattern floor of harness spec
7.4 refuses any shell command that names `~/hydra/`'s runtime directories, and `worktrees/` is
one of them, so a run that names where it stands is refused by the rule that protects the other
runs. Every Cerebro ticket since then says "relative paths only", and the builders comply, but
a provider that prints `pwd` and pastes it into its next command, or an `rg` with an absolute
glob, hits the floor by accident, and the refusal reads like a policy violation when it is not.

1. Should the floor carve out the run's own worktree, meaning the one path under
   `~/hydra/worktrees/` that equals the run's `cwd` from `run_start`? The gate knows that path:
   `spawn_config.rs` writes it into the hook config. The carve-out would be one prefix compare
   before the pattern match. What it must not do: let the run name a sibling worktree, the
   `runs/` directory or `spawn/`.
2. If the carve-out is wrong, what should the refusal say? Today it reads like any other floor
   hit. The alternative is a refusal that names the cause ("your own worktree is under a
   protected directory; use a relative path") so a provider recovers in one turn instead of
   spending 146k tokens.
3. Does this change the seatbelt profile too? On macOS the profile already allows writes under
   the run's worktree by absolute path, so the seatbelt and the floor disagree about the same
   string, and the floor wins.
4. Is the Cerebro case special because its repository lives outside `~/hydra`, or does a Hydra
   self-edit run hit the same thing? Measure it: every run's worktree is under `~/hydra/worktrees/`
   whatever the repository, so the answer should be no, but the attack corpus of map 68 should
   say whether any recorded attack relied on an absolute worktree path.

5. The same floor refuses any shell command with a heredoc or substitution that names git, aws
   or Slack. On 2026-10-13 it stopped build 238's first attempt (a `python3 - <<'PY'` edit) and
   cost the GPT seat of the map 117 panel its memo file (a `cat <<'EOF'` whose body mentioned
   git; the memo survived only in `result_text`). Both were the classifier working as written.
   Should a heredoc whose body, not whose command, names git be refused, or should the gate
   distinguish the command line from the data it feeds?

## Answer

Resolved 2026-10-15 by the human, who took the panel's recommendation ("sigo con las
decisiones del panel"). The premise was wrong: the own worktree is already carved out in both
passes of the shell check and measured allowed on two live gates; run `01a1332c` was refused for a
heredoc body that named `~/hydra/runs`, and the record does not carry the body.

1. No new carve-out. A string-prefix one would reopen attack H3 of map 68.
2. The refusal names the longest matching deny entry, the token that resolved to it and the link
   it went through; never "use a relative path", which was false for the record in question.
3. No seatbelt change.
4. Every run. Cerebro is not special; the attack corpus has no class that relies on the own
   worktree path.
5. Yes: the body of a heredoc with a quoted delimiter and anything in single quotes are literal,
   and both the substitution rule and the protected-path substring pass skip them; an unquoted
   body expands and stays refused. In the same change the floor splits segments on a real newline,
   the hole Opus measured (`exit 0` newline `git gc` passed, `exit 0; git gc` was refused), because
   once newlines separate commands every heredoc line starting with `git` is a command to the floor
   unless the floor knows where the quoted body ends. A refused shell command is recorded whole.

Question 6, whether `standard` lets a shell read the run's own pinned snapshot on a sandboxed
launch, goes to map 127.

Tickets: builds 319 (the floor's heredoc, quote and newline rules), 320 (the refusal names its
cause), 321 (the refused command on the record).

## Panel

Two seats sat on 2026-10-13, Opus 5.5 and Gemini 3.8 Flash; the GPT seat died on codex's usage limit at its first turn and there is no Grok seat while Cursor is on the free plan. Memos: `research/120-seat-opus.md`, `120-seat-gemini.md`. The session's synthesis with a recommendation per question: `research/120-synthesis.md`. Its first finding: the ticket's premise does not hold, the own worktree is already carved out and measured allowed; run `01a1332c` was refused for a heredoc body that named `~/hydra/runs`, and the record does not carry the command. Nothing under Answer is written; the human answers.
