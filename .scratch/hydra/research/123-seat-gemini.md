# Map 123, research seat memo (Gemini 3.8 Flash)

Written 2026-10-13 by one seat of the four-seat panel on map ticket 123. My mandate is to answer the three design questions on the run's git pointers reaching sub-repositories, based on implementation facts, file and line citations, measurements in this worktree, and verified evidence. Citations reference relative paths within this repository. No provider was launched.

---

## 1. Is this the design?

### What the code does today
1. In `src/run/mod.rs:441-445` (`compose_child_env`), when a run has a repository (`repo.is_some()`), Hydra calls `crate::worktree::prepare_run_objects(layout, run_id, repo)`.
2. In `src/worktree.rs:368-376`, `run_object_env` constructs two process environment variables:
   - `GIT_OBJECT_DIRECTORY` pointing to `layout.run_objects(run_id)` (`~/hydra/spawn/<run-id>/objects`).
   - `GIT_ALTERNATE_OBJECT_DIRECTORIES` pointing to `<common_git_dir>/objects` (`~/hydra/.git/objects`).
3. In `src/run/mod.rs:446-448`, both variables are inserted into `hydra_env` and `child_env`. `src/interactive.rs:219` sets them identically for gated interactive launches. They travel by value to the detached watcher and the child process environment.
4. The comment in `src/worktree.rs:362-367` acknowledges the design trade-off:
   `"Environment, because git has no other way to move a checkout's object directory. No configuration key names one, and a private commondir would move the refs with it... The cost is that git inside the run, in any repository, reads and writes this store."`
5. Because these variables reside in process environment, every child process—including any `git` command executed in a newly initialized sub-repository inside the worktree—inherits them. When `git init` or `git commit` runs in a subdirectory, Git writes newly created objects into `spawn/<run-id>/objects` instead of `<subrepo>/.git/objects/`. In fact, Git does not even create an `objects/` directory inside the sub-repository's `.git/`.
6. At run cleanup, `hydra reap` calls `crate::worktree::import_run_objects(&main, store)` (`src/reap.rs:500-505`). In `src/worktree.rs:477-521` (`pack_into_parent`), Git packs every object in the store via `git pack-objects --stdout` and pipes it into `git index-pack --stdin --strict` in the parent repository (`main`).
7. Consequently, every object authored in any sub-repository is imported into the parent repository's `.git/objects/`. Because no branch or tag in the parent repository references those commits, they exist in the parent store as unreferenced (dangling) objects.
8. Meanwhile, when `reap` commits the worktree to `hydra/<run-id>` (`src/reap.rs:510`, `commit_everything` in `src/worktree.rs:1049-1110`), the sub-repository's `.git/` directory lacks its object database. Any reader that drops the environment variables (such as `GitLocation::configured` in `src/worktree.rs:129-136`) finds a hollow `.git` directory without an `objects/` folder, treats it as not a git repository, walks up to the worktree root, and operates against the parent repository.

### What was measured and verified
1. Inherited environment in this worktree:
   `env | grep -E '^GIT'` confirmed that both `GIT_OBJECT_DIRECTORY` (`/Users/USER/hydra/spawn/01a13756-d51d-7621-aeba-6831003acd8a/objects`) and `GIT_ALTERNATE_OBJECT_DIRECTORIES` (`/Users/USER/hydra/.git/objects`) are active in this run's process environment.
2. Sub-repository behavior:
   - Initialized a scratch repository: `git -C .scratch/test_subrepo init` and committed a file.
   - Inspected `.scratch/test_subrepo/.git/objects`: `find .scratch/test_subrepo/.git/objects -type f` failed with `No such file or directory`. Git skipped creating the directory because `GIT_OBJECT_DIRECTORY` was already defined.
   - Executed `git -C .scratch/test_subrepo log -1`: succeeded and printed the subrepo commit (`cf87581`), verifying that Git used the run's object store.
   - Executed `env -u GIT_OBJECT_DIRECTORY -u GIT_ALTERNATE_OBJECT_DIRECTORIES git -C .scratch/test_subrepo rev-parse --git-dir`: returned `/Users/USER/hydra/.git/worktrees/01a13756-d51d-7621-aeba-6831003acd8a`. Because the subrepo had no `objects/` directory, Git disregarded `.scratch/test_subrepo/.git`, walked up the directory tree, and bound to the parent worktree.
   - Executed `env -u GIT_OBJECT_DIRECTORY -u GIT_ALTERNATE_OBJECT_DIRECTORIES git -C .scratch/test_subrepo log -1`: printed commit `2ffd1515f9d0` (the parent worktree's HEAD commit), confirming that unsetting the pointers causes Git to treat the sub-repository as transparent.
3. Historical confirmation in build 309:
   Build 309's Outcome (`.scratch/hydra/build/309-a-runs-worktree-is-born-with-a-cloned-target.md:101-106`) documented this exact behavior: a fixture repository created from a run's shell had no `objects/`, Hydra's git dropped the variables, walked up to the parent checkout, and attempted to write a ref into `~/hydra/.git`, which was refused by the seatbelt.
4. Historical confirmation in build 302:
   Build 302 (`.scratch/hydra/build/302-the-suite-is-green-inside-a-run.md:89-118`) resolved test fixture failures across 12 integration files by having test subprocesses explicitly strip both pointers.

### Recommendation
No, this is an unintended implementation side-effect rather than the design: Map 74 was created solely to redirect writes from the run's primary checkout away from the parent repository's object store, and sub-repository objects landing in the parent repo as unreferenced dangling objects violates repository isolation. A run's newly created repository should be fully self-contained with its own `.git/objects/` and travel in the worktree as regular files, while reap should only import objects belonging to the run's primary checkout.

### What would change my mind
Evidence that an existing Hydra workflow relies on `reap` importing unreferenced commits from nested repositories into the parent repository, or proof that permitting sub-repositories to retain their own object databases within the worktree introduces a mechanism to bypass seatbelt integrity guarantees on the parent repository.

---

## 2. Where is the fix?

### What the code does today
Ticket 123 posits three potential locations for a fix:
1. Git config in the checkout:
   Git provides no configuration key for the primary object directory. `extensions.worktreeConfig` only scopes supported config keys to specific worktrees; `extensions.objectFormat` sets the hash algorithm (sha1 or sha256); `core.alternateRefsCommand` configures alternate ref advertisement. Git's `includeIf.gitdir:` allows conditional config inclusion, but cannot set a nonexistent key. Throughout Git's design, `GIT_OBJECT_DIRECTORY` is an environment variable; the object database path cannot be overridden via repository configuration without patching Git itself.
2. A `git` shim on PATH:
   Hydra could place a `git` wrapper binary or script in a directory prepended to `PATH` (e.g. `spawn/<run-id>/bin/git`). The shim would check `--git-common-dir`: if it matches the parent checkout's common dir, it would set the pointers and exec the real Git; otherwise, it would leave them unset.
   However:
   - In `src/env.rs:21` and `src/env.rs:157-164`, `PATH` is strictly allowlisted from host environment. Adding a dynamic binary directory to every run modifies `ChildEnv`, alters the spawn directory layout, and requires seatbelt review (`src/sandbox.rs:105-215`).
   - Latency: If the shim forks `git rev-parse --git-common-dir` before every git command, it doubles the process execution overhead of every git invocation.
   - The bootstrapping failure: When `git init` runs in a new, empty directory, no `.git` exists yet. Running `git rev-parse` in that directory walks up and discovers the parent worktree! The shim would therefore mistakenly set `GIT_OBJECT_DIRECTORY` during `git init`, reproducing the exact bug it was designed to fix.
3. Product unchanged + documentation:
   `IMPLEMENTATION.md:5069-5070` already states: `"Git inside a run, in any repository, uses that store."` `IMPLEMENTATION.md:6035-6045` details how build 302 isolated fixture subprocesses via `tests/common/git.rs`. `config/preamble.md:1-12` is protected config #4, dedicated strictly to instruction provenance and policy boundaries under Map 28.

### What was measured and verified
1. Bootstrapping test with `git rev-parse`:
   In an empty directory `.scratch/test_empty`, executing `git rev-parse --git-common-dir` returned `/Users/USER/hydra/.git`. A shim relying on git directory discovery would falsely treat `git init .scratch/test_empty` as targeting the parent checkout.
2. Environment allowlist:
   `src/env.rs:21` restricts `BASE` to `HOME`, `USER`, `PATH`, and `TERM`. Adding a shim directory requires mutating `ChildEnv::build` and ensuring child scripts or nested subagents do not bypass or tamper with the shim.
3. Test suite isolation in practice:
   In `tests/common/git.rs:11-17`, `isolated_command` simply calls `env_remove("GIT_OBJECT_DIRECTORY")` and `env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")`. This approach cleanly resolved the issue for all 12 integration test files in build 302 without needing a system-wide shim.

### Recommendation
The fix should not be an invasive `git` shim or a git config wrapper, because Git lacks an object-directory config key and a shim introduces double-fork latency while failing on `git init` due to upward repository traversal. Leave the product runtime unchanged, keep `config/preamble.md` unpolluted by operational git trivia, and document the `env -u` requirement in `IMPLEMENTATION.md` and repository skills for tasks that construct nested repositories.

### What would change my mind
Upstream Git adding a native `core.objectDirectory` configuration option that respects `extensions.worktreeConfig` or `includeIf.gitdir:`, or empirical evidence from production run records showing that autonomous agents frequently create sub-repositories that break upon reap without a runtime shim.

---

## 3. Product change vs. documentation

### What the code does today
1. Build ticket 302 (`.scratch/hydra/build/302-the-suite-is-green-inside-a-run.md:89-118`) made the entire test suite independent of the inherited environment pointers.
2. In `tests/common/git.rs:1-18`, `command()` and `isolated_command()` construct `std::process::Command` with `env_remove` for both `GIT_OBJECT_DIRECTORY` and `GIT_ALTERNATE_OBJECT_DIRECTORIES`.
3. Twelve integration test files use this helper: `tests/cleanup.rs`, `tests/copilot_repo_and_tools.rs`, `tests/curate.rs`, `tests/drift.rs`, `tests/evolve.rs`, `tests/git_config.rs`, `tests/launch_phases.rs`, `tests/outcome.rs`, `tests/record_contract.rs`, `tests/skills.rs`, `tests/spawn.rs`, and `tests/worktree.rs`.
4. `src/run/mod.rs:1332-1337`: the 3 tests in `src/run/mod.rs` re-execute themselves in an isolated child binary with `env_remove` for both pointers.
5. `src/worktree.rs:129-136`: `GitLocation::configured` strips both pointers from all internal git calls Hydra executes against checkouts.
6. `crates/grader/tests/originals.rs`, `crates/puppets/tests/differential.rs`, `tests/export.rs:331-332`, and `tests/sandbox_copilot.rs:152-153` explicitly isolate their subprocess environments.
7. `IMPLEMENTATION.md:6034-6045` documents the subprocess inheritance mechanism and the build 302 resolution.
8. `config/preamble.md:1-12` is protected config #4 under `docs/harness-spec.md` section 15.1. It contains 12 lines governing instruction provenance, data classification, and policy enforcement.

### What was measured and verified
1. Test suite execution in this active run:
   Executed `cargo nextest run --release --workspace` inside this worktree with `GIT_OBJECT_DIRECTORY` and `GIT_ALTERNATE_OBJECT_DIRECTORIES` intact in the environment:
   - Exactly **1,388 of 1,388 tests passed** (4 skipped) in **47.28 s**.
   - Zero test failures occurred despite running inside a gated Hydra worktree with both git pointers exported.
2. Single-test verification:
   Executed `cargo test --release --lib run::tests::a_run_with_a_repo_is_handed_its_own_object_directory`: passed in 0.07 s.
3. Review of `config/preamble.md`:
   The preamble is exactly 12 lines (631 bytes) and establishes security and provenance rules. It does not carry operational advice, command-line syntax tips, or provider quirks. Injecting git plumbing instructions into the preamble would dilute its core security mandate for every run across all providers.

### Recommendation
The product does not need to change because the test suite is verified green inside a run (1,388 of 1,388 tests passing) and typical agent workflows operate within the existing checkout. Document the behavior in `IMPLEMENTATION.md` and repository skills (such as `hydra-self-edit`), while keeping `config/preamble.md` strictly focused on instruction provenance and security boundaries.

### What would change my mind
Evidence that real coding tasks routinely initialize sub-repositories that become corrupted upon `reap`, or if CI or developer environments report flakiness in test fixtures that cannot be addressed through `tests/common/git.rs`.

---

## Recommendation summary
1. A run's new repository should travel in the worktree as self-contained files with its own objects, not as unreferenced objects imported into the parent by reap.
2. Reject the `git` shim and config workarounds; keep the product runtime unchanged and document the `env -u` requirement in `IMPLEMENTATION.md` and repo skills.
3. No product change is needed: the test suite is verified green inside a run (1,388 tests passed), and `config/preamble.md` should remain focused on instruction provenance rather than git plumbing.
