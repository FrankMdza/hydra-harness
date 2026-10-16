# 134 - The public gate: for each exposure the repository admits, state it, fix it before, or fix it after

Type: grilling
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from the human's words: SECURITY.md reads as full of holes or
out of date, and every one should be resolved before the repository is public. Section 3 of
`.scratch/hydra/research/133-the-plan-of-2026-10-15.md` lists what the repository admits in open
tickets and in IMPLEMENTATION.md, what the seats of that night found that nothing admits, and the
grok seat's table (`research/133-seat-grok-security-audit.md`) says claim by claim what SECURITY.md
states, what code and test back it, and what it does not say. Grilling: the human answers.

## Question

For each exposure in that section, one of three answers:

- **State**: SECURITY.md gains a line under "Known exposures" naming it and the ticket that owns it
  (build 340). The repository goes public with it stated.
- **Fix before**: a build ticket, and publication waits for it.
- **Fix after**: stated and ticketed, fixed in the first public release cycle.

The session's recommendation per item, for the human to accept or overrule:

1. The two found tonight, build 338 (`codex`'s `webrun` unknown to the table) and build 339 (a
   denied non-shell call leaves no row): fix before, each one afternoon.
2. The twelve stale sentences of build 313 and the `src/mcp.rs` comment: fix before, prose only
   (build 341; the comment is done).
3. The shell floor's assembled-path residual, the sibling-worktree token, the `claude` and `codex`
   fail-open shapes, the seatbelt as defence not boundary, admission's reach to `HOME` and
   `~/.ssh`, Linux unconfined: state. Each is already measured and pinned open in a test on
   purpose; the honest public version says so in one place.
4. Map 99 (`~/.claude.json` writable from a gated interactive session), map 108 (secret classes),
   map 114 (the bench cannot tell a refusal from a breach): fix after, with the map ticket named.
5. `git push` with no arguments unchecked against protected branches: fix before, small.

Two publication decisions left by map 130 belong here too: whether `hydra` publishes to
crates.io at all, given its path dependency on a `publish = false` crate, and the repository's
topics, homepage and social preview.
