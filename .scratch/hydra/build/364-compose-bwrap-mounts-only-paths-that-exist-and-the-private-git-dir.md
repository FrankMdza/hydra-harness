# 364 - `compose_bwrap` mounts only paths that exist, and the private git directory whole

Type: build
Status: resolved
Blocked by: 318b

## Objective

Close the critical Linux finding of the review panel on `37525dc`, Architect seat `01a17e09-b543-77e3-990c-5f0f95d696aa`. `compose_bwrap` in `src/sandbox.rs` turns every `rules_of` rule into `--bind SRC SRC` or `--ro-bind SRC SRC`. Bubblewrap aborts with `Can't find source path` when SRC does not exist at launch, and SBPL on macOS never cared, because it matches a path when the process opens it. On a clean launch several sources do not exist yet, so the first bwrap launch on a real Linux host fails before the provider starts.

## Problem

Sources `rules_of` names that may be missing at launch:

1. `<spawn>/gate-refusals.jsonl`, `Rule::WriteAllowLiteral`, `src/sandbox.rs` around line 468. Nothing creates it before the first refusal.
2. `<spawn>/seats.sock`, the `allow_seat_socket` of `Rule::UnixSocketConnectDeny`, around line 400. Only a gated interactive launcher listens on it, so on a plain `hydra run` it never exists.
3. `own_read_paths` in `src/policy/mod.rs` line 1076: `<spawn>/cursor-data` on `cursor-agent` and `~/.claude/projects/<slug>` on `claude`. The vendor binary creates them after launch.
4. `<git>/worktrees/<run-id>/index.lock` from `own_write_paths`, line 888. It exists only while git holds the lock.

There is a second defect in the same place. Under bwrap, `index` and `index.lock` mounted as two single-file binds break `git add`: git creates `index.lock` with `O_EXCL` and then `rename`s it over `index`, and a rename onto or off a mount point fails with `EBUSY` or `EXDEV`.

## Cases

1. A unit test builds a real `Spawn` in a scratch layout, the way `scratch_layout` in `src/sandbox.rs` tests does, with a real `git worktree add` so `<git>/worktrees/<run-id>/` exists, runs the same spawn-time preparation `hydra run` runs, then calls `compose_bwrap`. Every source after `--bind`, `--ro-bind` and `--dev-bind` exists on disk. Run it for `claude`, `cursor-agent` and `codex` under `standard`.
2. In the same test the args carry `--bind <git>/worktrees/<run-id> <git>/worktrees/<run-id>` and, after it, `--ro-bind` on each of `HEAD`, `commondir`, `gitdir` and `logs` under that directory that exists. They carry no single-file mount of `index` or `index.lock`.
3. A non-interactive spawn's args carry no `--bind` of `seats.sock`. A gated interactive spawn whose socket exists still carries it.
4. `compose()` on macOS renders the SBPL profile byte for byte as it does today. `compose_and_compose_bwrap_share_one_rule_table_across_policies_and_providers` and the SBPL tests stay green unchanged. If the git directory change cannot keep SBPL identical, keep SBPL's `index` and `index.lock` rules and diverge only in `translate_rule_to_bwrap` or `compose_bwrap`, and say so in the Outcome.

## Do

1. Tests first, in `src/sandbox.rs` `mod tests`, for the four cases.
2. Pre-create at spawn time, in the code that already writes the spawn directory for a launch (find it from `write_bwrap_profile` callers and `src/watch.rs` line 321), only on the bubblewrap path: an empty `gate-refusals.jsonl`, and each `own_read_paths` directory with `create_dir_all`. Do not pre-create them on macOS; nothing there needs it and the SBPL tests pin the current layout.
3. Make the seat socket allow conditional on the launch being a gated interactive one. If `Spawn` has no field that says so, add the narrowest one and set it from `src/interactive.rs` only.
4. For the private git directory under bwrap: bind `<git>/worktrees/<run-id>` whole, then `--ro-bind` `HEAD`, `commondir`, `gitdir` and `logs` over it, skipping any that do not exist. A comment says why: rename across a mount point.
5. Write denials whose path does not exist, from `write_denies`, `GitAdminWriteDeny`, `CodexSkillsWriteDeny` and `PluginWriteDenySubpath`: do not decide what bwrap does with them. Skipping one lets the run create the path; pre-creating one changes what an optional config file's absence means. Leave them as they are, list every such path the Case 1 test sees under `## Outcome`, and say in `## Found` that this is a decision for the map. The Case 1 assertion covers allow sources only, and names this ticket for why.
6. Amend `IMPLEMENTATION.md` at the bubblewrap paragraph near line 48: what 364 fixed, and that a deny path missing at launch is still open.
7. Run `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, `cargo nextest run --release -p hydra --lib sandbox`, and `cargo nextest run --release -p hydra --test sandbox_read_only --test spawn`.

No Linux host is available. Say so in the Outcome; do not claim a launch was measured.

## Outcome

Bubblewrap profile writing now creates the refusal log without truncating it and creates each provider data directory from `own_read_paths`. SBPL profile writing creates neither. The shared rule table and SBPL renderer are unchanged.

Bubblewrap mounts the private git directory whole. It then mounts each existing `HEAD`, `commondir`, `gitdir` and `logs` read-only. It mounts neither `index` nor `index.lock` separately. Git can rename its lock over its index within one mount. SBPL keeps its original index and lock rules.

The gated interactive flag travels through the spawn writer and profile writer as a launch argument. Only the interactive writer sets it. It does not add a field to the shared policy `Spawn`, so gate compilation and existing SBPL call sites keep their shape. A headless profile never binds the seat socket, even if one exists. An interactive profile binds it only when it exists. The existing refusal of gated interactive launches under bubblewrap remains.

The new unit test uses real git worktrees for claude, cursor-agent and codex under standard. It checks all allow mount sources after bubblewrap preparation. It also checks metadata mount order, socket selection, refusal evidence preservation and the absence of bubblewrap preparation on the SBPL path. Existing tests were not changed.

Missing deny sources remain unchanged. The test printed the following paths. `<root>` is its unique scratch layout. `<checkout>` is the Hydra checkout. `<provider>` expands to each of `claude`, `cursor-agent` and `codex`, so each path containing it occurred three times.

- `<checkout>/config/env-allowlist.json`
- `<checkout>/config/export-signoff.json`
- `<root>/pruned`
- `<root>/puppets`
- `<root>/approvals`
- `<root>/skills`
- `<root>/skills-snapshots`
- `<root>/skill-worktrees`
- `<root>/<provider>/.claude/skills`
- `<root>/<provider>/.claude/settings.json`
- `<root>/<provider>/.claude/settings.local.json`
- `<root>/<provider>/.agents/plugins`
- `<root>/codex/.codex/skills`

No missing private git metadata denial occurred in this fixture. Missing deny sources need a map decision. Skipping them allows creation. Creating them can change the meaning of absent optional files.

All five requested checks passed: `cargo fmt --all --check`, `cargo clippy --release --workspace --all-targets -- -D warnings`, `cargo check --target x86_64-unknown-linux-gnu`, `cargo nextest run --release -p hydra --lib sandbox` with 13 tests, and `cargo nextest run --release -p hydra --test sandbox_read_only --test spawn` with 39 tests.

No Linux host was available. No bubblewrap launch was measured. The Linux cross-check passed on macOS. The regression first reproduced the missing socket source. Its first socket fixture also exceeded the macOS Unix socket path limit, so the fixture now uses a shorter scratch root under `/tmp`.
