# 99 - Git outside the seatbelt reads no configuration a run can write, and a failed worktree setup always leaves a record

Type: build
Status: resolved
Blocked by: 

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item 4, 6, 13 and the code half of 15, findings H5, H47, H39, H40, H6 and H17.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/worktree.rs, src/run.rs and src/skills/mod.rs.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

`CheckoutGit` and the plain git runner ignore global and system git configuration and pass the identity explicitly; a launch from inside `worktrees/` is refused; every fallible step after `git worktree add` in `worktree::create` either reaches the failure path or is rolled back, with a test; partial skill injection is recorded; `phase_spawn` is under 150 lines; the unused `_layout` parameter is gone. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome

Built on 2026-09-23, all six findings.

**H5, global and system config.** `GitLocation` has two methods now. `configured()` is git at a
location reading every level of configuration, and `command()` is that plus
`GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`. Every call through `git`, `git_bytes`
and `git_with_index` uses `command()`, for `CheckoutGit` and for the plain `Path` runner alike.
`repo_root`, `git_common_dir`, `head_exists` and reap's `drop_branch` built their own `Command`
and now go through the runner too. The identity is passed explicitly: `worktree::identity` reads
`user.name` and `user.email` through `configured()` as data, and `git_as_author` puts them in
front as `-c`. Three calls write a commit through it: `commit_everything`, which reap and curate
use, and the two `commit-tree` calls in `src/rungs/evolve.rs`. The skill store's commit already
passed `hydra <hydra@localhost>` and is unchanged. The seed's `git diff HEAD` also passes
`--no-ext-diff --no-textconv`, which H47 named.

Measured by `tests/git_config.rs`, a new file holding one test, because it sets
`GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` for the whole process. The planted global file
carries a `core.hooksPath` with a `post-checkout` hook, a clean filter and an external diff, and
the planted system file carries a second filter. `worktree::create` and `commit_everything` then
run against a parent with no identity of its own. With the two variables taken out of
`command()`, the hook, the global filter and the system filter all fired. With the identity
taken out, the commit went through anyway under `Francisco Mendoza
<frank@laptop.local>`, git's guess from the hostname. So dropping global config
without passing the identity would not have failed loudly. Reap commits would quietly have
changed author.

**H47, a launch from `worktrees/`.** `run::refuse_launch_inside_checkout(layout, dir)` refuses
any directory whose canonical path is under `layout.worktrees()`. `preflight_inputs` calls it
before `repo_root`, so no git runs there, and `interactive::run_gated` calls it before its own
`repo_root`. `tests/worktree.rs::a_launch_from_under_worktrees_is_refused` checks a directory
inside a checkout, the same directory reached through a symlink, and a directory outside.

**H39, rollback in `create`.** The four steps after `git worktree add` are `bind_and_seed`
now. On an error, `roll_back` runs `git worktree remove --force`, `git branch -D` and removes
the context file. If the rollback fails too, the error names both failures. I chose rollback over
recording through `PostWorktree::fail`. The launch has no record yet at that point, and a
checkout that no longer exists needs none.
`tests/worktree.rs::a_seed_that_fails_after_worktree_add_leaves_no_checkout_and_no_branch` fails
the seed with an untracked file at mode 000. With the rollback off, the checkout stayed.

**H40, partial skill injection.** `skills::inject` returns `InjectError { partial, error }` on
failure. It pushes each name onto `linked` before the link or copy is made, so the failing skill
is listed as well, since a copy can stop half made. `run::inject_skills` takes a
`slot: &mut Option<InjectedSkills>`. Once `inject` has returned, even with an error, the slot
holds a `skills` group naming what reached the checkout, so a later `?` cannot drop it.
`phase_handoff` passes `recording.injected` as the slot, and `PostWorktree::fail` composes
line 0 from it. Two tests: `tests/skills.rs::an_injection_that_fails_partway_names_what_it_left_in_the_checkout`,
and `run::tests::a_failed_injection_leaves_the_names_it_wrote_where_the_failure_record_reads_them`,
which drives `inject_skills` itself. Moving the push back after the write failed both, listing
only `alpha`. Dropping the slot write on error failed the second.

**H6 and H17.** The agy gate-mount check moved out of `phase_spawn` into `agy_gate_mounted`.
`phase_spawn` is 124 lines, counting from `fn` to the closing brace. Moving injection into
`inject_into_checkout` keeps `phase_handoff` at 129, down from 149. `CheckoutGit::for_run` takes
no `Layout` now, and its five callers were updated.

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean. The whole suite
passed outside a seatbelt, 25 test files plus 203 unit tests.

**What the ticket assumed that was not true.**

- "The plain git runner" is more than `worktree.rs`. Evolve's two `commit-tree` calls go through
  it and need the identity, and reap's `drop_branch` built its own `Command`. Both files changed:
  `src/rungs/evolve.rs`, `src/reap.rs`, plus `src/interactive.rs` for the refusal and the new
  `inject_skills` signature, and `src/status.rs` and `src/normalize/mod.rs` for `for_run`.
- Ignoring global config changes more than hooks and filters. A human's `core.excludesFile`
  set in `~/.gitconfig` to a custom path is no longer read. The default
  `~/.config/git/ignore` still is, because git reads that path when the key is unset. So the
  seed's `ls-files --others --exclude-standard` can copy files the human ignores only through a
  custom global excludes file into the run's checkout. A repository that uses Git LFS gets
  pointer files in its checkout, since `filter.lfs` lives in global config. Neither applies on
  this machine: `~/.gitconfig` holds only `[user]`, and the system file holds only
  `credential.helper`. I left both alone. Reading `core.excludesFile` as data the way the
  identity is read would close the first, and whether to do that is a small call for the human.
- The plain runner still runs the parent repository's own hooks, as `IMPLEMENTATION.md` says it
  does. The panel's "give the plain runner the same belt" could also mean `core.hooksPath=/dev/null`.
  I did not add it. With the launch from `worktrees/` refused, the parent is the human's own
  repository, whose `.git` a run cannot write, and turning its hooks off would change documented
  behaviour.
- The refusal also refuses delegation from inside a run on a platform without the seatbelt. On
  macOS the delegate face already refuses from inside the seatbelt, map ticket 64, so nothing
  that works today stops working. A Linux port would need to decide this again.
- `hydra evolve` still calls `repo_root` on its own current directory, which this ticket did not
  touch.

**Proposed documentation, for build ticket 102.**

`SECURITY.md`, replacing the bullet "Git that Hydra runs outside the seatbelt does not trust the
checkout's `.git` file" and the last two sentences of the object-store bullet:

> - **Git that Hydra runs outside the seatbelt trusts neither the checkout's `.git` file nor any
>   configuration a run can write.** Every git call Hydra makes in a run's checkout sets
>   `GIT_DIR`, `GIT_COMMON_DIR` and `GIT_WORK_TREE` from the run record's
>   `scopes.parent_git_dir`, or from launch data saved beside the checkout under the deny list,
>   and turns off hooks and fsmonitor. Every git call Hydra makes anywhere sets
>   `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`, so a filter, an external diff or a
>   hook path a run writes into `~/.gitconfig`, or into a system file the user owns, runs nowhere.
>   Commits keep their author: Hydra reads `user.name` and `user.email` as data and passes them
>   with `-c`. A launch from under `~/hydra/worktrees/` is refused before any git runs there. One
>   exception: `git worktree remove` reads the checkout's `.git` file, so reap rewrites it to the
>   trusted directory first. A checkout with neither trusted source is left for a human, and reap
>   says how to remove it.

and the object-store bullet ends at "what to do about it is map ticket 74, open."

`IMPLEMENTATION.md`, in "Git that Hydra runs in a run's checkout, since build ticket 95", after
the first paragraph:

> Since build ticket 99, no git Hydra starts reads global or system configuration.
> `GitLocation::command` sets `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1` for the
> checkout runner and the plain one alike. Commits pass the identity git would have resolved,
> read as data by `worktree::identity`, because without it git guessed one from the hostname and
> committed anyway. A custom `core.excludesFile` and Git LFS's filters live in global config and
> are not read either. `run::refuse_launch_inside_checkout` refuses a launch whose directory is
> under `worktrees/`, before discovery runs git there. `worktree::create` rolls the checkout, its
> branch and its context file back if any step after `git worktree add` fails. A failed skill
> injection records a `skills` group naming every skill it started, so reap holds those names back.

`README.md` and `CLAUDE.md`: the test-file count is 25, and `tests/git_config.rs` plants a
global and a system git config and checks that nothing they name runs from Hydra's git.
