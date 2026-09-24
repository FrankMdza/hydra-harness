# 103 - Each run writes git objects into its own store

Type: build
Status: resolved
Blocked by: 99

## What

The third hiring panel, 2026-09-23. Implement `~/.claude/handoffs/2026-09-23-panel3/consolidated.md` "Before publishing" item map ticket 74's answer, and item 3, findings H1.
Read those findings in its Findings table first; each names the lines and what was measured.
Files: src/policy/mod.rs, src/sandbox.rs, src/worktree.rs, src/reap.rs, src/run.rs and the gate's write list.

Documentation changes go in the Outcome as proposed sentences, not into `SECURITY.md`,
`IMPLEMENTATION.md` or `README.md`: several tickets touch those files at once and the session
applies them together, build ticket 102.

## Done when

A run's `git add` works with the parent's `objects/` fully denied; reap moves the run's objects into the parent from outside after git checks them; a test plants a forged pack in the run's store and shows it never reaches the parent or a later worktree; `tests/spawn.rs` measures `git add` under the real profile outside a seatbelt. Every new test fails before the change, checked by reverting it. `cargo fmt --check` and
`cargo clippy --all-targets -- -D warnings` are clean and the suite passes outside a seatbelt.

## Outcome

Built on 2026-09-24, map ticket 74's answer as decided. No decided rule moved.

**The run's store.** `Layout::run_objects` is `spawn/<run-id>/objects`. `own_write_paths` grants
it in place of the parent's `objects/`, so the parent's `.git` is denied whole again, to the
seatbelt and to the gate, which reads the same list. The path is taken beside the gate script,
so a replay resolves it from the record the way it resolves the gate.

**How the run's git finds it: environment.** `worktree::prepare_run_objects` creates the
directory and answers `GIT_OBJECT_DIRECTORY=<store>` and
`GIT_ALTERNATE_OBJECT_DIRECTORIES=<common>/objects`. `run::compose_child_env` puts both on
`hydra_env`, so they travel by value to the watcher and the child and land in
`run_start.env_allowlist`. The gated arm of `hydra run --interactive` sets them too. I chose
environment because nothing else can do it. Git has no configuration key that moves an object
directory. `extensions.worktreeConfig` only scopes keys that already exist. A private
`commondir` would move the refs along with the objects. A run can unset either variable. Its
git then writes into the parent's store, which both layers deny, so `git add` fails instead of
landing anywhere shared. The cost: git inside a run, in any repository, reads and writes the
run's store. The curator is not affected, because `curate` grants no shell.

**Hydra's own git never reads the run's store as an alternate.** The watcher starts from the
child's environment, so it inherits both variables. `GitLocation::configured` now removes them
for every runner, checkout and plain alike. Reading the store as an alternate would be the hole
itself: `git add` skips writing an object it finds already stored, so a pack forged under the
name of a checkout file's bytes would have become Hydra's answer. The run-end diff stat, rung
two's read and `hydra status` need nothing from the run's store. Each one stages from an index
under the spawn directory that the run cannot write (`stat-index`, `seed-index`), so Hydra hashes
the checkout itself and writes the objects into the parent. Curate's proposal needs nothing
either. The store checkout's private index sits in the store's denied `.git`, so every entry in
it came from Hydra. Of the four callers the ticket named, reap is the only one the run's objects
reach.

**Reap moves what git verifies.** `commit_then_remove` takes the store for a run checkout and
`None` for a curate checkout. Before staging it calls `worktree::import_run_objects`. That
function lists the names the store's own files offer: loose fan-out files, plus each pack index
read with `git show-index`. It skips links and never follows one. It runs `git pack-objects
--stdout` with the store as the only object directory and pipes the result into `git
index-pack --stdin --strict` in the parent. `index-pack` names every object from its bytes. No
file in the store is copied. Then `worktree::unstage_missing_objects` finds each index entry
whose object the parent does not hold, using `ls-files --stage` and `cat-file --batch-check`
against the parent alone. It drops those entries with `update-index --force-remove`, so the
`git add -A` that follows hashes those paths again from the checkout. A store git will not verify
moves nothing, and reap says so on stderr and commits from the checkout.

**One seatbelt carve-out the ticket did not name.** The first run of the spawn probe failed on
`git status`: "fatal: Invalid path '<root>/spawn': Operation not permitted". Git resolves
`GIT_OBJECT_DIRECTORY` one component at a time, and the profile denies reads of the whole
spawn parent. `sandbox::compose` now adds `(allow file-read-metadata (literal <spawns>))` for
a spawn with a parent repository. The test checks that `ls` of the spawn parent is still
refused.

**The substring floor keeps the spawn parent.** `denied_spellings` drops a denied path from the
raw substring pass when an own path lives under it. Counting the store would have taken
`spawn/`, and every sibling's gate and `launch.json` with it, off that list for every run with a
checkout. The store is left out of that computation. The run's git never spells the path, and a
command that spells it is refused, which costs nothing.

**Measured under the real profile**, `tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory`,
outside a seatbelt, git 2.55.0, Darwin 25.2. With the two variables set, `git status` and `git
add` exit 0, the blob lands in `spawn/r1/objects/<fan>/<rest>`, and the parent's store lists the
same files before and after. Without them, `git add` is refused with "error: unable to create
temporary file: Operation not permitted". A `mkdir` and a write of a pack under the parent's
`objects/pack` are refused. `git commit` is still refused on the refs. The creation-only probe
that test carried since build ticket 95 is gone, because map 74 settled the question it
measured.

**Measured by hand on the forgery**, git 2.55.0, scratch repositories. When the forged pack is
the only copy of a name, `cat-file` serves the forged bytes. When the real object is loose,
whether in the same store or in the parent, git serves the loose copy. So substitution needs the
name to be missing or packed, which is why the tests forge a name the parent does not hold yet.

**Tests, each failed before its change, checked by reverting it:**

- `tests/cleanup.rs::reap_moves_the_runs_objects_and_a_forged_pack_reaches_neither_the_parent_nor_a_later_worktree`.
  The run stages a file with its own git, writes an object only its store holds, and plants a
  pack naming the sha of content the human has not written yet. The run's own git serves "EVIL"
  for that name. After reap, the staged file is on the branch, the run's own object is in the
  parent, and the forged name is not. The human then commits that content and a later worktree
  is seeded. With the import replaced by a plain copy of the store's files, the forged name
  reached the parent. With that assertion switched off, the human's own commit carried the
  forged bytes, and so did the later worktree's `future.txt`. With the import off, the run's
  object was missing from the parent.
- `tests/cleanup.rs::an_entry_the_run_staged_against_a_forged_object_is_hashed_again_from_the_checkout`.
  The pack is planted before the run's `git add`, so the index names the forged bytes. With
  `unstage_missing_objects` off, reap failed on git's missing object. The file's mtime is set an
  hour back. Without that, git treats the entry as racily clean and hashes it again, and the
  test passed with the fix off.
- `tests/spawn.rs::the_seatbelt_allows_git_add_in_the_worktrees_private_git_directory`, as
  above. With the parent's `objects/` granted again, the bare `git add` went through: "git add
  wrote into the parent's objects/: rc=0". Without the metadata carve-out, `git status` failed.
- `run::tests::a_run_with_a_repo_is_handed_its_own_object_directory`: with the variables not
  composed, both were missing from `hydra_env`.
- `worktree::tests::no_git_hydra_starts_inherits_a_runs_object_directory`: with the two
  `env_remove` calls gone, every runner inherited the variables.
- `policy::tests::a_spawn_with_a_parent_git_dir_carves_out_its_private_git_directory` now expects
  the run's store and not the parent's `objects/`. It also checks that `spawn/` stays on the
  substring list. Counting the store took it off.

`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` are clean. The whole suite
passed outside a seatbelt with `~/.local/bin` off `PATH`: 27 targets, 747 tests, lib 208,
cleanup 41, spawn 32.

**What the ticket assumed that was not true.**

- "Every git Hydra runs in a run's checkout must find the run's objects." Only reap needs them.
  Giving the others the store would have let the run answer for them.
- The gate's write list needed no change of its own. `gate.py` reads `own_write_paths`. It
  did need the substring-list exception above.
- The seatbelt needed a metadata read on the spawn parent, which no ticket named.
- Not measured: whether each of the four providers passes `GIT_OBJECT_DIRECTORY` through to its
  shell tool. `codex`'s default `shell_environment_policy` excludes only names that match
  `KEY`, `SECRET` and `TOKEN`, by its documentation. If a provider strips the variables, that
  run's `git add` fails at the seatbelt, and nothing reaches the parent. The next real run under
  each provider should run `git add` once and read the store.

**Proposed documentation, for build ticket 102.**

`SECURITY.md`, replacing the object-store bullet:

> - **A run writes git objects only into its own store.** A run with a checkout gets
>   `GIT_OBJECT_DIRECTORY` pointed at `~/hydra/spawn/<run-id>/objects` and reads the parent's
>   store as an alternate. The parent's `.git` is denied whole to the seatbelt and to the gate,
>   `objects/` included. A pack a run forges in its own store lies only to its own git. Reap
>   moves the run's objects into the parent from outside the seatbelt with `git pack-objects`
>   and `git index-pack --strict`, which names each object from its bytes, so a forged name
>   never reaches the parent or a later worktree. An index entry whose object did not arrive is
>   hashed again from the checkout. No other git Hydra runs reads the run's store: each one
>   drops the two variables and hashes the checkout itself.

`IMPLEMENTATION.md`, in "Git that Hydra runs in a run's checkout, since build ticket 95",
replacing the last bullet:

> - Since build ticket 103, map ticket 74's answer: a run's own git writes into
>   `Layout::run_objects`, `spawn/<run-id>/objects`, through `GIT_OBJECT_DIRECTORY`, and reads
>   the parent's store through `GIT_ALTERNATE_OBJECT_DIRECTORIES`. Both travel on `hydra_env`.
>   `own_write_paths` grants that directory instead of the parent's `objects/`. The profile also
>   allows metadata reads of the spawn parent itself, because git resolves the path one component
>   at a time. `GitLocation::configured` removes both variables, because the watcher inherits
>   them. Reap runs `import_run_objects` and then `unstage_missing_objects` before it commits.
>   The diff stat, rung two's read and `hydra status` stage from indexes under the spawn directory
>   and need nothing from the run's store. Git inside a run, in any repository, uses that store.

`README.md` and `CLAUDE.md`: no file count changes. `tests/cleanup.rs` now also plants a forged
pack in a run's store and reads the parent and a later worktree back.
