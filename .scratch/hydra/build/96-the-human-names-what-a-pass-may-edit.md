# 96 - The human names what a rung-two pass may edit, over a floor, and existing tests are refused

Type: build
Status: resolved
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

## Outcome

Built by `claude` on Opus 5.5 under `standard`, run 01a0cfa7-d9c9, 3.9M tokens, then fixed by the
session after an Opus review, on top of build ticket 95. `--may-edit` is a repeatable flag,
normalized by `may_edit_set` and refused before launch when it reaches the floor; the default is
`src/normalize` and `src/show.rs`; `admission_refusal` runs the floor, build ticket 80's
existing-test refusal and the set, in that order; the harness `run_start` carries `may_edit`; the
proposer's brief lists the set. evolve went from 26 to 35 tests, each of the agent's new ones
failing or not compiling against the old code.

What the review changed:
- `--review` ran none of the refusals, so a fresh review of a branch blocked for `Cargo.toml` or
  an edited baseline test could come back offerable. It now runs all three under the set the
  pass recorded, or the default. `a_review_uses_the_set_its_pass_recorded_or_the_default`
  covers the set it reads; the wiring into `review_blocked` is covered by reading, since that
  path needs a live layout.
- The floor and the set compared paths with `normalize::paths::inside`, and `src/normalize/` is
  in the default set, so an undirected pass could weaken the floor's own comparison. `evolve.rs`
  has its own copy now, and a test runs the default set through the floor.
- `existing_test_hit` read `base_tree` through the proposer's checkout, the discovery build 95
  removed elsewhere; it goes through the pinned context with `--no-replace-objects`.
- `rust-toolchain` and `rust-toolchain.toml` joined the floor: they point rustup at a toolchain
  the human's own `cargo` then runs, the same reason as `Cargo.toml`.
- The brief says a `build.rs` is refused anywhere; the `--may-edit` help says `tests` cannot be
  named whole, so a new test is named by its path.

Contract 5.5 does not list `may_edit` yet, and spec sections 4.4 and 15.2 describe the old list;
both are the session's edits to `docs/`. The whole suite passes on master outside a seatbelt
except the corpus walk in `tests/normalize.rs`, which fails before this change on one record,
noted in build ticket 95.
