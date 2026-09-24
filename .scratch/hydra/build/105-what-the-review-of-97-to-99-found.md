# 105 - What the review of 97 to 99 found

Type: build
Status: resolved
Blocked by: 103

## What

An Opus review of build tickets 97 to 99 as merged, 2026-09-24, in
`~/.claude/handoffs/2026-09-23-panel3/review-97-99.md`. Implement every numbered item under 98 and
99 there. The measured one comes first: since build 99, files the human ignores only through a
global `core.excludesFile` are seeded into a run's checkout and committed by reap.

## Done when

Each item has a test that fails before the fix, checked by reverting it, and the suite passes
outside a seatbelt. Documentation sentences go in the Outcome for the session.

## Outcome

Built on 2026-09-24 by a Claude Code session working on Hydra's source, not through `hydra run`.
Every numbered item under 98 and 99 is done. None needed a decision.

**98.1, cargo configuration beside the toolchain.** `admission_profile` no longer skips a `.cargo`
that holds the toolchain. For each ancestor of the copy it denies the literal
`<a>/.cargo/config` and `<a>/.cargo/config.toml`, as the review said, and also the literal
`<a>/.cargo` name, except for the `.cargo` the toolchain lives under. The review's fix alone was
not enough, measured with `sandbox-exec` on 2026-09-24. A `.cargo` replaced by a symbolic link to
another directory is resolved past a deny on the two files. The deny on the name catches that. A
second thing turned up while testing that. A rule on `file-read-metadata` outranks a rule on
`file-read*` whatever their order in the profile. So the metadata allow 98 added for each
ancestor's direct children beat a later `(deny file-read* (literal <a>/.cargo))`, and the link
still resolved. Each ancestor now gets two deny lines with the same filters, one `file-read*` and
one `file-read-metadata`. With only the first line, the test reads the linked `config.toml`. This
means the 98 profile as merged had the same hole: its `.cargo` subpath deny lost to the metadata
allow on a `.cargo` that is a link, so "SBPL takes the last matching rule" holds only within one
operation.

**98.2, the profile outside the writable directory.** `prepare_admission` writes the profile as
`<dir>.sb`, that is `spawn/<pass-id>/admission.sb`, beside `admission/` rather than inside it, and
removes a stale one first. The proposer's profile denies writes to `spawn/`, and the admission
profile opens only `admission/`, so no step can rewrite it.

**98.3, the pass record closes.** An outer error from `prepare_admission` becomes an admission
refusal, "admission gate: the copy of the read tree could not be prepared: ...". The pass then
commits `rungs/blocked/<pass-id>` and `write_harness_end` runs, as for a failing step.

**98.4** is a statement for the documents, below.

**99.1, 99.2 and 99.5, what Hydra's git takes from wider configuration.** `GitLocation::command`
now appends `worktree::pinned_config`, which reads `core.excludesFile` and every `safe.directory`
through `configured()` in one `git config -z --get-regexp` and passes them back with `-c`:

- `core.attributesFile=/dev/null` always. Measured by hand first with git 2.55.0: with
  `GIT_CONFIG_GLOBAL=/dev/null`, git still reads `$XDG_CONFIG_HOME/git/attributes`, and `*.txt
  text` there turned a CRLF file into LF on `git add`.
- `core.excludesFile` set to the configured file, or to git's default
  `$XDG_CONFIG_HOME/git/ignore` (`~/.config/git/ignore`) when unset. Anything but a regular file
  becomes `/dev/null`, so a fifo cannot hang git. A leading `~/` is expanded against `HOME`.
- `-c safe.directory=<value>` for each configured value, in order. Measured by hand: git honours
  it from `-c`, and `GIT_TEST_ASSUME_DIFFERENT_OWNER=1` reproduces the refusal in a release git.

`identity` reads through the same helper. Every such read has a 5 second timeout,
`worktree::CONFIG_READ_TIMEOUT`. On timeout the child is killed, the read counts as finding
nothing, and a line on stderr says so. The doc comment on `identity` now says Hydra's commits are
unsigned because `commit.gpgSign` lives in global configuration.

The cost is one extra `git config` process per git call Hydra makes, not measured against the
suite's time before. I did not cache the read: a cache keyed by location would go stale when the
human edits `~/.gitconfig` during a long reap.

**99.3, the interactive path.** `prepare_policy_launch` runs every step after `worktree::create` in
`finish_policy_launch`. On an error, `abandon_policy_launch` calls `worktree::roll_back`, now
public, and removes `spawn/<run-id>`. The rollback removes whatever skills were linked or copied
into the checkout, so there is no partial slot left to record. `run_gated` now calls
`Provider::Claude.version()`, `ChildEnv::build` and the `sandbox-exec` check before the checkout
exists. `prepare_run_objects` moved into the prep and travels as `PolicyLaunch::object_env`. A
failed spawn of `claude` rolls back too. Rollback over a failure record, the same choice build
ticket 99 made in `create`: nothing ran in the checkout.

**99.4, the refusal.** `refuse_launch_inside_checkout` checks `layout.skill_worktrees()` as well as
`layout.worktrees()`. `rungs::evolve::pass_repo(layout, dir)` calls it before `repo_root`, and both
`evolve` and `review_blocked` use it.

**Tests, each checked by reverting its fix and watching it fail:**

- `tests/git_config.rs::a_file_ignored_only_by_a_global_excludes_file_is_neither_seeded_nor_committed`.
  Without the `core.excludesFile` argument, "the seed copied a file the human's global excludes
  file ignores".
- `tests/git_config.rs::a_repository_that_needs_safe_directory_is_still_found`. Without the
  `safe.directory` arguments, `repo_root` answered `None`.
- `tests/git_config.rs::an_attributes_file_under_xdg_config_home_changes_nothing_reap_commits`.
  Without `core.attributesFile=/dev/null`, the committed blob was LF.
- `tests/git_config.rs::a_global_config_that_includes_a_fifo_does_not_hang_hydras_git`. With the
  timeout set a thousand times longer, `identity` and `repo_root` had not answered after 20
  seconds. The test takes about 10 seconds when it passes, two timeouts.
- `tests/spawn.rs::a_gated_interactive_launch_that_fails_after_its_checkout_leaves_none_behind`. A
  directory at the gate script's path fails `write_spawn_config`, the last step. Without the
  rollback, the checkout outlived the launch.
- `tests/worktree.rs::a_launch_from_under_skill_worktrees_is_refused`. With only `worktrees()` in
  the list, the launch went ahead.
- `tests/evolve.rs::a_pass_started_inside_a_runs_checkout_is_refused_before_discovery`. Without
  the refusal in `pass_repo`, discovery inside a real checkout answered a repository.
- `tests/evolve.rs::the_admission_profile_denies_cargo_config_beside_the_toolchain_and_cannot_be_rewritten`.
  With the old skip restored, `cat` of the `config.toml` beside the toolchain succeeded. With
  the profile back inside `admission/`, a step appended to it. With the `file-read-metadata` line
  removed, `cat` read `config.toml` through a linked `.cargo`. It also checks the toolchain file
  stays readable and the copy stays writable.
- `tests/evolve.rs::a_copy_that_cannot_be_prepared_blocks_the_pass_and_closes_its_record`. A file
  where `admission/` goes. With `?` back, `land_proposer_patch` returned the error and the record
  stayed at one line.

`tests/git_config.rs` holds five tests now, serialised by a mutex because each sets process-wide
variables. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean. Whole
suite outside a seatbelt, `~/.local/bin` off `PATH` so `tests/drift.rs` skips: 29 targets with doc
tests, 775 passed, 0 failed.

**What the review assumed that was not quite true.**

- 98.1: denying the two literal files is not enough on its own, for the two reasons above.
- 99.3: the steps after the spawn in `run_gated`, `procinfo::start_time` and the record's first
  write, can still fail with `claude` already running and no record. Rolling back there would
  pull the checkout out from under a live session, so I left them. They are the same shape as
  H39 and need a failure record written with the pid, not a rollback.

**Proposed documentation sentences.**

`IMPLEMENTATION.md`, "Git that Hydra runs in a run's checkout, since build ticket 95", after the
paragraph build ticket 99 proposed:

> Since build ticket 105, three values still come from the wider configuration, each read as data
> by `git config` and passed back with `-c` by `worktree::pinned_config`: `core.excludesFile`, or
> git's default `~/.config/git/ignore` when it is unset, and every `safe.directory`.
> `core.attributesFile` is pinned to `/dev/null`, because git reads `~/.config/git/attributes`
> even with global configuration off, and an attribute there changes the bytes reap commits. An
> excludes file that is not a regular file becomes `/dev/null`. Every one of these reads, and
> `identity`'s, gives up after five seconds, so an `[include]` naming a fifo in `~/.gitconfig`
> cannot hang reap. Hydra's commits are unsigned, because `commit.gpgSign` is global
> configuration. A launch is refused under `skill-worktrees/` as well as `worktrees/`, and
> `hydra evolve` refuses there before it discovers a repository. A gated interactive launch that
> fails after its checkout exists rolls the checkout, branch and spawn directory back.

`IMPLEMENTATION.md`, rung two, replacing the admission profile sentences in the paragraph build
ticket 98 proposed:

> The steps run under `spawn/<pass-id>/admission.sb`, beside the admission directory and outside
> what the steps can write. It is the proposer's profile plus three things: `admission/` opened,
> metadata of each ancestor and its direct children opened, and in every ancestor of the copy,
> reads of `.cargo/config`, `.cargo/config.toml` and the `.cargo` name itself denied. The `.cargo`
> holding the toolchain keeps its name readable. Each deny is written twice, for `file-read*` and
> for `file-read-metadata`, because a seatbelt rule on `file-read-metadata` outranks one on
> `file-read*` whatever the order. A copy that cannot be written blocks the patch like a failing
> step. Every pass needs the network: `CARGO_HOME` starts empty, so cargo downloads the crates in
> `Cargo.lock` each time, 416 seconds measured by the review. Admission needs `rustc` on every
> platform, Linux included, because the toolchain is found with `rustc --print sysroot`.

`SECURITY.md`, the admission paragraph build ticket 98 proposed, replacing "the admission profile
denies reads of every `.cargo` directory above the copy":

> the admission profile denies reads of cargo's configuration files in every `.cargo` above the
> copy, and of the `.cargo` name, so a link planted in its place resolves nowhere

`README.md` and `CLAUDE.md`: no test-file count changes. `tests/git_config.rs` now also checks the
excludes file, `safe.directory`, the XDG attributes file and the fifo timeout.
