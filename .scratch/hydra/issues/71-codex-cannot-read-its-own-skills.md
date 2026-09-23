# 71 - `codex` cannot read the skills Hydra links into its worktree

Type: grilling
Status: open
Blocked by:
Opened: 2026-09-22, by the session, from build ticket 81's first run.

## Question

Run `01a0cc2a-f20b` (`codex`, `standard`, build ticket 81) stopped before writing anything. Its
first command was `cat .codex/skills/hydra-self-edit/SKILL.md; sed -n ... src/live.rs; ...`, and
the gate refused it whole: `hydra/protected-path: the command names /users/user/hydra, denied
under every policy by harness spec 7.5`. The link resolves into `~/hydra/skills-snapshots/<sha>/`,
which is under the denied checkout and on the raw pass's list. `codex` then declined to go on
without the skill it had been told to read.

`policies/standard.json` grants `file_read` on `skill_snapshot`, and on `claude` and `cursor-agent`
that grant works, because they read through a file tool the gate scopes by path. `codex` has no read
tool. Everything it reads, it reads by shell, and the shell check refuses a command that names a
protected path whole, because a shell command can write as easily as read. So on `codex` the
`skill_snapshot` grant is unreachable, and every skill Hydra injects is a link the run is refused
for opening. The same is true of `agy` wherever it reads by `run_command` rather than `view_file`.

1. **Copy rather than link on `codex`.** Seed the snapshot's files into the worktree as ordinary
   files. The run can read them by shell; it can also edit them, and `hydra reap` would commit
   whatever it left, so the reap has to drop them. Harness spec 6.4 chose links so a run could not
   edit its own instructions mid-session.
2. **A read-only shell carve-out.** Let a shell command that names the snapshot path and no other
   protected path through. It is the same argument 7.5 rejected for the worktrees directory: a
   substring test cannot tell a read from a write.
3. **Do not inject skills on `codex`.** Record `skills.injected` as empty for the provider, and
   say in spec 6.4 that the store reaches three providers.
4. **Leave it and tell the run.** The preamble says the link will be refused. The skill is then
   an advertisement for something the run cannot open.

## Answer

Open.
