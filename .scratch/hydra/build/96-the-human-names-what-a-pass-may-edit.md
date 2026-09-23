# 96 - The human names what a rung-two pass may edit, over a floor, and existing tests are refused

Type: build
Status: claimed
Blocked by: 89, 90

## What

Map ticket 73, resolved 2026-09-23, and build ticket 80 with it. Read the ticket's `## Answer` and
`.scratch/hydra/research/73-panel-opus.md` first.

1. **`--may-edit`.** `hydra evolve` (`src/cli.rs` ~165-168, beside `--task`) takes a repeatable
   `--may-edit <path>`, a file or a directory prefix, repository-relative. Without it, a default set
   applies: `src/normalize/` and `src/show.rs`. A patch whose changed paths (`changed_paths`, both
   sides of a rename) include any path outside the named set is sent to `rungs/blocked/<pass>` before
   admission and before any judge, with the reason naming the path, like `protected_path_hit` today.
   The pass record carries the named set.
2. **The floor.** `PROTECTED_CODE_PATHS` (`src/rungs/evolve.rs` ~448) plus the config rows stay a
   refusal no name lifts, and gain `src/pty.rs`, `src/watch.rs`, `src/paths.rs`, `Cargo.toml`,
   `Cargo.lock`, any path whose file name is `build.rs`, and `.cargo/`. A `--may-edit` naming a floor
   path, or a directory containing one, refuses the pass before the proposer is launched. `src/run.rs`
   is not on the floor (harness spec 15.2, map ticket 60); it is refused unless named. Keep the
   case-insensitive match. Update the comment above the list to say it is a floor under a named set,
   and remove "a proposer must not delete or rewrite the probes" where the list does not do that.
3. **Existing tests refused (build ticket 80).** A patch that modifies or deletes a file under
   `tests/` that exists at `base_tree` (`git ls-tree -r --name-only <base_tree> tests/`) is refused
   the same way, whatever `--may-edit` says. A new file under `tests/` is allowed if named or under a
   named prefix. Changing a test's mode or renaming it counts as modifying it.
4. The proposer's task text says which paths it may edit, so a proposer does not spend a pass on a
   refused file. Find where `evolve.rs` composes the proposer task.

## Done when

`tests/evolve.rs`: an unnamed path refused; a named one admitted; `src/run.rs` refused unnamed and
admitted named; a floor path refused even when named, and a `--may-edit` naming a floor path refusing
the pass before launch; `build.rs` anywhere refused; an edit to an existing test refused and a new
named test file admitted; a rename out of a named directory refused. Each fails before the change.
`tests/evolve.rs` ~218-308, which assert today that `src/lib.rs` and `src/run.rs` pass, change to
the new rule. Build ticket 80 closes with this one; the session writes its Outcome.

## Notes for the run

Tests take a scratch `Layout`, never `Layout::discover()`. Ignore
`no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. Run `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` before you stop. `docs/` is the session's: the spec
sections 4.4 and 15.2 change after this lands.
