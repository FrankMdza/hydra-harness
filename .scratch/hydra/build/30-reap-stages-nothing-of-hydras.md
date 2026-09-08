# 30 - `hydra reap` stages nothing of Hydra's own

Type: build
Status: resolved
Blocked by: 06

## Goal

`worktree::commit_everything` runs `git add -A`, so a Hydra file the checkout does not ignore
lands on `hydra/<run-id>` for the human to merge. Hydra drops a `.gitignore` beside its own
files in `.cursor/` and `.agents/`, but leaves an existing one alone, so a repo that tracks its
own `.cursor/.gitignore` or `.agents/.gitignore` gets Hydra's hook config and MCP server list
committed by reap. `provider::claim` warns about it today. Make reap exclude those paths from
what it stages, which is the fix that doc comment names.

## Where the design says it

- `docs/harness-spec.md` section 6.4 for the ignore rule, and the `hydra reap` row of section 17.
- `src/provider/mod.rs::claim`, the doc comment that says the fix belongs to reap.
- `src/provider/cursor.rs::gitignore` and `src/provider/agy.rs::gitignore`, which list the
  names: `.cursor/.gitignore`, `.cursor/hooks.json`, `.cursor/mcp.json`, `.agents/.gitignore`,
  `.agents/hooks.json`, `.agents/mcp_config.json`.
- `IMPLEMENTATION.md`, "Rough edges worth knowing", the first bullet.

## The slices

**One list.** A function in `src/provider/mod.rs`, `own_worktree_paths() -> Vec<&'static str>`,
returns the six relative paths above, built from the two providers' `gitignore` functions rather
than retyped, so a provider that adds a file changes one place.

**Reap excludes them.** `commit_everything` in `src/worktree.rs` takes the exclusions and stages
with pathspecs: `git add -A -- . ':(exclude)<path>'` per path. The emptiness check before it
must use the same exclusions, so a checkout whose only untracked files are Hydra's own commits
nothing and returns `None`. `hydra reap` and the curate-store sweep pass the list; any other
caller of `commit_everything` passes an empty list and behaves as today.

**The warning stays.** `claim` keeps warning, with its text amended to say reap will not commit
the file, since the human still sees an untracked file in their checkout while the run is live.

**Tests.** `tests/cleanup.rs`: a throwaway repo that tracks its own `.cursor/.gitignore` with
unrelated content; the run's worktree holds a real edit plus `.cursor/hooks.json` and
`.cursor/mcp.json`; after reap the branch's commit carries the edit and neither Hydra file. A
second test: a worktree whose only untracked files are Hydra's own reaps with `committed: None`
and the branch is dropped as empty.

## Done when

- The two tests above pass and the rest of the suite is untouched.
- `git show --stat` on a branch reaped from such a repo lists no `.cursor/` or `.agents/` file of
  Hydra's.
- The orchestrating session removes the rough-edge bullet from `IMPLEMENTATION.md` and says in
  the cleanup section what reap stages.

## Outcome

Resolved 2026-09-08. `cursor-agent` on Cursor Grok 4.6 high fast, run 01a07ffe-4142 under
`standard`: 1.92M tokens, 1.79M of them cache reads, five files, 11 minutes, two cleanup tests
and one unit test, every test green in its worktree. `own_worktree_paths` is built from the
two `gitignore` functions as the ticket asked, and the model found the ticket's one false
assumption: excluding from the commit is not enough, because `git worktree remove` without
`--force` refuses a checkout with untracked files, so the excluded files have to go after the
commit.

**Its staging did not survive contact with a real worktree.** The model wrote `git add -A --
. ':(exclude)<path>'`, and the first two reaps on this machine after the diff landed failed
with "The following paths are ignored by one of your .gitignore files": on a real checkout
Hydra's own files are always ignored, by the ignore file Hydra drops, and `git add` refuses
a command that names an ignored path even to exclude it. Its tests passed because the
throwaway repo ignored nothing. The orchestrating session rewrote `commit_everything` to
`git add -A`, `git reset -- <paths>`, commit, then `remove_file` on whichever of the paths is
untracked, and dropped the `git clean -f` the model had reached for. The model's tests pass
unchanged against the rewrite, which is what they were for.
