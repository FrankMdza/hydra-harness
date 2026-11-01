# 134 - The public gate: for each exposure the repository admits, state it, fix it before, or fix it after

Type: grilling
Status: resolved
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

The session's recommendation per item, revised after the panel of 2026-10-15 (plan section 6 and
8), for the human to accept or overrule:

1. Fix before: build 339 (a denied non-shell call leaves no row, so SECURITY.md lines 7 to 8 are
   false on `codex`), build 348 (a relative MCP path walks past the protected-path check, so
   lines 21 to 24 are false), build 349 (a bare `git push` reaches a protected branch), and the
   prose of builds 341 and 340, in that order. All four seats agreed on these.
2. The human's hand before publication, map 99: name `~/.claude.json` in SECURITY.md lines 73 to
   74, and measure whether `claude` starts without the file before the allow at
   `src/sandbox.rs:348-356` changes. Three seats put this first; the first version of this ticket
   had it under fix after.
3. State: the shell floor's assembled-path residual, the sibling-worktree token, the `claude` and
   `codex` fail-open shapes, the seatbelt as defence not boundary, admission's reach to `HOME` and
   `~/.ssh`, Linux unconfined, the live walk absent from CI, build 130's unmeasured interactive
   gate. Each is measured and pinned open in a test on purpose; the honest public version says so
   in one place.
4. Fix after, with the map ticket named: map 108 (the write-time sweep flags five classes where
   export has twelve), map 114 (the bench cannot tell a refusal from a breach), build 338
   (`webrun` unknown to the table, which fails closed). Map 110 is the human's: if `jev ask` ever
   comes back it sends diffs off the machine.
5. `config/mcp.md` lines 41 and 42, "`mcp_call` is granted by `permissive` alone", are stale since
   map 82 and protected: the human edits them before publication.

Two publication decisions left by map 130 belong here too: whether `hydra` publishes to
crates.io at all, given its path dependency on a `publish = false` crate, and the repository's
topics, homepage and social preview.

## Where it stands on 2026-10-28

Checked by the session, which did not sit a panel on it: what is left is the human's hand or a publication step, and the human's delegation of 2026-10-27 covers decisions, not publishing.

- Item 1 is done. Builds 338, 339, 340a, 340b, 341, 348 and 349 are resolved.
- Item 2: `SECURITY.md` names `~/.claude.json` under the seatbelt bullet. Map 99, measuring whether `claude` starts without the file, is still open.
- Item 3: `SECURITY.md` has "Known exposures, stated before someone else does" since build 340a, and builds 356 to 377 kept it current.
- Item 4: map 108 is resolved and build 338 is resolved. Map 114 and map 110 are open.
- Item 5 is still open. `config/mcp.md` lines 41 and 42 say `mcp_call` is granted by `permissive` alone, and `policies/standard.json` line 11 grants it for the `hydra` server. The file is protected, so the human edits it.
- crates.io: `Cargo.toml` already sets `publish = false`. The repository is private on GitHub, with no topics and no homepage.

What the human has left before publication: the `config/mcp.md` edit, map 99's measurement, map 110, the GitHub topics, homepage and social preview, and the switch to public.

## Answer

Resolved on 2026-11-01 by a panel the human asked for, three seats of four after Grok's `auth_failed` (`research/134-synthesis.md`).

- Map 99 is fix before: build 378 denies both files in the gated session, and resolves 99.
- Maps 110 and 114 are fix after, stated in `SECURITY.md`. Nothing under `src/` sends a diff off the machine, and `hydra bench` refuses to launch without `--dry-run`.
- The audit found no false claim that needs a new build. Two stale sentences, the bench line and the copilot's write list, go into build 378.
- crates.io: no, `publish = false` stands.

What the human does before publication, in order: apply build 378 and type its 30-minute acceptance session; replace `config/mcp.md` lines 40 to 43 with the text in the synthesis; set the eight topics in the synthesis, no homepage; push; switch the repository to public.

Later on 2026-11-01 the session went through the rest of what a visitor meets. CI had started no job on master since 2026-10-28, because of a YAML error in `ci.yml`; fixed, with `tests/workflows.rs`. The four Dependabot pull requests are taken on master. gitleaks over master's history finds only planted fixtures, now fingerprinted in `.gitleaksignore`. A fresh clone built from zero and launched a recorded run that ended `completed`. Two stranger readers' findings went into build 379. Two items join the human's list: `config/export-allowlist.json` names `/Users/USER/hydra`, so `hydra export` allows nothing on another machine, and whether `.scratch/` goes public as it is, with `private-repo` task summaries under `labeling/`. Removing a file from the tree does not remove it from the history.
