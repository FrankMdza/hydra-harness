# 24 - Verify ticket 10's injection shape, and map skill precedence

Type: research
Status: claimed

## Question

Ticket 10 specified injection from evidence that is complete for `agy` and partial for the other
two. Before ticket 16 writes it into the harness spec, build the exact shape and watch all three
providers load it. Then answer the precedence question ticket 10 left open.

### Part one: does the shape work

Build it in a throwaway git worktree outside `~/hydra`, one directory per provider:

- `<worktree>/.claude/skills/` for `claude`
- `<worktree>/.cursor/skills/` for `cursor-agent`
- `<worktree>/.agents/skills/` for `agy`, launched with `--add-dir <worktree>`

Each holding a `.gitignore` of `*` and a symlink to a probe skill checked out somewhere else, the
way a `~/hydra/skills-snapshots/<sha>/` worktree would be. Confirm per provider that the skill is
discovered, that `git status` in the worktree stays clean, and that `git add -A` stages nothing.

Known already, do not re-derive: `agy` discovers a linked skill beside that `.gitignore`, and needs
`--add-dir` or it sees nothing. Claude Code follows symlinked skill directories (ticket 03). Cursor
follows symlinked files and directories at project scope (ticket 10, second pass). What is untested
is the whole shape at once, and specifically whether Claude Code and `cursor-agent` read a
project-scope root from a **linked git worktree** rather than an ordinary directory.

### Part two: precedence

Ticket 10 fails a run when a store skill collides with a name in the user-scope index. It lets a
repo's own project-scope skills load alongside Hydra's links without deciding what happens when
those collide. Settle, per provider:

- The scope order. Builtin, plugin, user, project: which wins on a duplicate name?
- Whether the loser is dropped silently or reported. Gemini deduped on frontmatter `name` once
  already and said nothing, which is how an earlier finding in this map went wrong.
- Whether the directory name or the frontmatter `name` is the identity being compared. Ticket 10's
  collision check needs to compare the same thing the provider compares, or it checks the wrong
  string.
- Whether a provider surfaces the collision anywhere Hydra could read it.

`agy -p "/skills" --output-format json` is free, returns `num_turns: 0` and `total_tokens: 0`, and
reports a resolved `path` and a `builtin` flag per skill, so Google's half costs nothing. Cursor and
Claude need a real run; keep the prompts tiny and say what each cost.

Clean up every probe. Write nothing under `~/.claude`, `~/.cursor` or `~/.gemini` that is not
removed afterwards, and say in the findings what the CLIs themselves wrote while running.

Output: `research/24-verify-injection-and-precedence.md`, plus a plain statement of which parts of
ticket 10's answer need amending.

## Partial results, 2026-09-02, agent killed by a rate limit

The agent working this ticket died on a session rate limit before writing a findings file. What
follows was recovered from the raw output of its paid runs, which survived on disk. Treat every
claim as evidence recovered rather than as a report someone wrote, and re-verify anything load
bearing. Its probe rig may still be at
`/private/tmp/claude-501/-Users-USER/f2c096c9-40e2-4c31-bdb1-bdcf8d1f4683/scratchpad/t24/`, which
is session scratch and will not survive long. Store sha under test was `57dd5052`.

The probe left six skill directories in `~/.claude/skills`, `~/.cursor/skills` and
`~/.gemini/config/skills`. All six were removed and the two directories the probe created,
`~/.cursor/skills` and `~/.gemini/config/skills`, were removed with them. `~/.claude/skills` is back
to its 11 skills.

### Part one: the shape works

All three providers loaded skills out of a real linked git worktree, at project scope, through a
directory hidden by a `.gitignore` holding `*`, linking into a snapshot checkout of a separate store
repo. That is ticket 10's shape end to end, and the agent's own note records the git side staying
clean in the linked worktree. Ticket 10's Q14 stands.

### Part two: precedence, and the finding that hurts

**The three providers resolve a name collision three different ways.**

| Provider | Same name in user scope and project scope |
|---|---|
| `claude` | user scope wins, project entry silently absent from the listing |
| `agy` | project scope wins, user entry silently absent |
| `cursor-agent` | neither wins, both are listed under the same name |

Cursor does not dedupe at all. Its listing carried `t24-clash` twice, once as
`CLASH-USER-CURSORROOT` and once as `CLASH-PROJECT`. Claude and `agy` each drop one silently, in
opposite directions.

**Identity is not the same string in all three.** A directory named `t24-dirname-only` whose
frontmatter said `name: t24-frontmatter-name` rendered as `t24-frontmatter-name` in `agy` and as
`t24-dirname-only` in both `claude` and `cursor-agent`. So `agy` keys on the frontmatter name and
the other two key on the directory name.

**`agy` breaks a tie by directory order.** Two directories both declaring `name: t24-alpha`
resolved to the alphabetically first directory, `t24-aaa-clash`, and the other vanished.

### What this does to ticket 10

Ticket 10's Q10 fails a run on a launch-time collision, and this evidence argues it was right, for a
better reason than the one given: the failure modes are not merely different, they are opposite, so
no single rule about who wins could be written into the spec.

It also breaks one sentence of ticket 10's reasoning. Q10 says the check must "compare the same
thing the provider compares". There is no such single thing. The check has to compare **both** the
directory name and the frontmatter name, against every scope, or it will miss a collision on one
provider while catching it on another.

### Unfinished, and one lead

- Cursor's own `--help` and its raw on-disk session record were being used to read the skill listing
  without paying for another model round trip. That technique is worth reusing: both `claude` and
  `cursor-agent` write a transcript to disk, so the listing can be mined rather than bought.
- The agent's last note before it died: **a skill can render with an empty description in Cursor**,
  which it called an injection hazard worth isolating. No detail survived. Reproduce before
  believing it.
- Nothing was established about `~/.grok/skills` or `~/.codex/skills`, which the rig had set up.

### Costs paid, so a resumed session does not repeat them

`claude` one run, 11,326 cache-creation plus 8,149 cache-read plus 280 output. `cursor-agent` three
runs, roughly 25,000 input across them. `agy` free, two runs, zero tokens both times.
