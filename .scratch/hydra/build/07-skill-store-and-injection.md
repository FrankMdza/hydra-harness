# 07 - The skill store, the snapshot, and per-run injection

Type: build
Status: open
Blocked by: 02

## Goal

One canonical store, snapshots pinned per run, links into the worktree, and the two
collision checks. `claude` first.

## Where the design says it

- Harness spec 6.4, all of it. The store layout with `hydra.toml` beside each
  `SKILL.md`, injection per run into the worktree and never into the user's home, the
  self-ignoring `.gitignore` holding `*`, snapshots at a pinned sha, committing a dirty
  store before pinning, the read-only index over user-scope roots, the two collision
  checks, and the write-time YAML validator.
- 6.1 items 5, the launch-time collision check as its own run outcome.
- Contract 5.3, `skills.store_sha`, `skills.linked` and `skills.resolved`, and why the
  third exists.

## Done when

- `~/hydra/skills/` is a git repo, snapshots are refcounted per sha, and links point at
  a snapshot rather than the live tree.
- A dirty store is committed before pinning, with authorship in the commit trailer.
- The launch check asks the provider what it resolved rather than comparing strings,
  and a hit fails the run as `skill_collision`.
- A description containing a colon followed by a space is refused at write time.
  `~/.claude/skills/handoff-frank` fails this today.
- `run_start.skills` stops being absent and carries all three fields.

## Watch for

**Injection has a measured side effect on `claude` and the spec states it rather than
hiding it.** Claude Code fits its whole skill listing into a character budget and renders
the overflow as bare names. Nine injected links moved bare entries from 8 to 14, and
three of the six newly stripped were the user's own plugin skills. Nothing warned. That
is why `skills.resolved` exists and why the record cannot claim what the model saw
without it.

**There is no shared notion of a skill's identity.** `claude` keys on the directory
name, `agy` on the frontmatter `name`, and Cursor's listing has no name field at all.
One probe directory named one thing and declaring another is a different skill to each
of them.
