# 366 - Bubblewrap gives every missing deny path a source, and refuses by name when it cannot

Type: build
Status: resolved
Blocked by: 364

## Objective

Carry out question 2 of map ticket 135, `.scratch/hydra/issues/135-what-bwrap-does-with-paths-that-do-not-exist-and-the-private-git-dir.md`, with its reasoning in `.scratch/hydra/research/135-synthesis.md`. Read both first. Today `compose_bwrap_for_launch` in `src/sandbox.rs` emits `--ro-bind SRC SRC` for deny paths that do not exist on a clean launch, and bwrap aborts with `Can't find source path`. Build 364's test printed the thirteen paths.

## Rules

1. **Protected config.** Where a `WriteDenySubpath` names a file under the Hydra checkout's `config/` directory, bwrap binds `config/` read-only whole instead, once. SBPL keeps its per-file rules.
2. **Layout directories.** `Layout::ensure()` in `src/paths.rs` line 391 also creates `pruned`, `puppets`, `approvals`, `skills`, `skills-snapshots` and `skill-worktrees`, through the layout's own accessors. This runs on both platforms. Check every caller of `ensure()` and every reader that treats a missing one of those as meaningful, such as `Store::open` in `src/skills/mod.rs`, and say what you found in the Outcome.
3. **Skill and plugin roots in the worktree**, `.claude/skills`, `.agents/plugins` and `.codex/skills`: on the bubblewrap path only, create an empty directory before the profile is written, in `write_bwrap_profile_for_launch` beside the 364 preparation, then bind it read-only as today.
4. **Settings files**, `.claude/settings.json` and `.claude/settings.local.json`, when absent: write a file holding `{}` under the run's spawn directory, for example `<spawn>/empty-settings.json`, and emit `--ro-bind <that file> <destination>`. bwrap creates the destination on the host as an empty file. `hydra reap` must remove a settings file that is untracked and empty or holds exactly `{}`, before it commits, the same way `drop_hydras_ignore` in `src/reap.rs` line 418 removes Hydra's own ignore file. Keep the removal narrow: never a tracked file, never one with other content.
5. **Backstop.** After the rules above, a deny source that still does not exist makes compose return an error naming the path, and the launch refuses with it. Replace the `expect("translatable spawn rule")` path with a `Result`, so neither case panics. Write a test that a clean launch fixture reaches zero missing sources for `claude`, `cursor-agent` and `codex` under `standard`, extending `bwrap_spawn_preparation_and_private_git_mounts` or beside it.

## Do

1. Tests first, in `src/sandbox.rs` `mod tests` and, for rule 4's reap removal, the existing reap tests in `tests/cleanup.rs`. Each rule gets a test that fails before the change.
2. `compose()` on macOS must render SBPL byte for byte as today. Run the SBPL tests unchanged.
3. Do not touch the private git directory handling, the sweep, or `src/worktree.rs` beyond what rule 4 strictly needs. Build 367 owns those and runs beside you.
4. Amend `IMPLEMENTATION.md` at the bubblewrap paragraphs near line 48 that say missing deny sources are open. Say what each rule does and that no Linux host has measured it.
5. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, `cargo nextest run --release -p hydra --lib sandbox`, `cargo nextest run --release -p hydra --test cleanup --test spawn --test sandbox_read_only`, and `cargo nextest run --release -p hydra --lib paths`.

## Outcome

Bubblewrap binds the checkout's protected config directory read-only once. Layout creation now supplies all six deny directories on both platforms. Bubblewrap preparation creates the three worktree skill and plugin roots and writes `{}` under the spawn directory. Missing settings bind that file. Existing settings retain their own source. Reap removes only untracked regular settings files that are empty or hold exactly `{}`. It preserves tracked files and every other byte sequence.

Compose now returns a Result. Missing deny sources produce an error naming the path. Translation errors propagate without an expect panic. The clean git worktree fixture reaches zero missing bind sources for Claude, Cursor and Codex under standard. It also checks existing settings, missing-source refusal and unchanged SBPL bytes. Existing argument tests now create their gate and git metadata sources. Their rule-order assertions remain. The settings reap test covers both filenames, tracked and untracked files, empty bytes, exact `{}`, a trailing newline and other JSON.

The ensure audit found four production callers: headless reservation, gated interactive launch, ungated interactive launch and evolve harness recording. Unit callers are in sandbox and run tests. None uses the absence of the six directories as a launch signal. Store::open checks for `.git`, so an empty skills directory still initializes the store. Approval listing, snapshot sweeping and orphan checkout listing treat missing and empty directories alike. Record readers still test record files. Puppet readers either enumerate entries or check goal files. An empty root supplies no goals.

One reader changes its diagnostic. Curate's store check tests whether skills is a directory before checking Git metadata and HEAD. An empty directory now reaches those Git checks. Git can discover an ancestor repository when the layout sits inside one. The session closed that in `src/rungs/curate.rs` when it applied this diff: the store check now requires `skills/.git`, so an empty `skills/` refuses as "no skill store", and `tests/curate.rs` plants an unreadable `.git` for the "not a git repository" half.

All six required commands passed: fmt check, release workspace clippy with warnings denied, the Linux target check, sandbox unit tests, the cleanup and spawn and sandbox_read_only integration group, and paths unit tests. The test groups passed 14, 92 and 10 tests. No Linux host measured these mounts, destination creation or Claude's treatment of `{}`. Build ticket 370 owns those measurements; 368 became the secret-classes table after map 108.
