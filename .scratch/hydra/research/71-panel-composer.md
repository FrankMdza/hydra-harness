# Panel 71 (composer)

## Recommendation

Copy admitted skills into the worktree for `codex` only, keeping symlinks for the other three providers.

## Mechanism

Today `skills::inject` always symlinks each skill from the pinned snapshot into `<worktree>/<provider skill root>/` (`src/skills/mod.rs:413-447`). For `codex` that root is `.codex/skills` (`src/provider/mod.rs:100-106`). A shell read resolves the link with `os.path.realpath` (`src/policy/gate.py:127-140`) to `~/hydra/skills-snapshots/<sha>/`, which sits under `denied_write_paths` (`src/policy/mod.rs:574-590`, `src/paths.rs:235-237`). `protected_path_named_by` refuses the whole command before scope is consulted (`src/policy/gate.py:259-328`, `866-885`). `file_read` on `skill_snapshot` in `policies/standard.json:6` never applies because `codex` has no read row in `CODEX` (`src/policy/table.rs:349-355`).

Branch `inject` on `Provider::Codex`: replace the `symlink` call (`src/skills/mod.rs:445-446`) with a recursive copy of each skill directory from the snapshot into the worktree root. Add a small `copy_tree` helper in the same file (~20 lines). Extend `tests/skills.rs` (~25 lines) to assert copied bytes, no symlink, and shell-readable paths. Amend harness spec 6.4 (~3 lines) to say `codex` gets copies. No `gate.py` or `reap.rs` change: copies stay under the worktree scope, and the existing `.gitignore` with `*` (`src/skills/mod.rs:422-424`) still keeps them off the branch `hydra reap` commits (`src/reap.rs:201-215`).

Rough total: ~50 lines of Rust, ~25 of test, ~3 of spec.

## Cost per provider

**codex:** Skills become readable by `Bash`, the only read path this provider has (`src/policy/table.rs:349-350`, `src/provider/codex.rs:300-301`). `skill_invoke` still maps to nothing (`src/policy/table.rs:294-296`). Copies are ordinary worktree files, so `read-only` routing stays blocked for the same reason.

**claude** and **cursor-agent:** Unchanged. They keep symlinks and reach skills through `file_read`, scoped to `skill_snapshot` after `realpath` (`src/policy/gate.py:895-919`).

**agy:** Unchanged for the common case (`view_file` at `src/policy/table.rs:241`). Shell reads of linked skills were already broken the same way as `codex`; this ticket does not need to fix them unless product asks.

## What a hostile run gains

A `codex` run could `cat` or `sed` files under `.codex/skills/` that it cannot open today. Shell does not consult `write_denied_worktree_paths` (`src/policy/gate.py:813-825`, `src/policy/mod.rs:736-742`), so a redirect or `sed -i` against a copied skill mutates this run's local copy mid-session. That is new for `codex` and weaker than symlink semantics on the other providers, where the same shell write resolves into the shared snapshot and is refused by `protected_path_named_by` (`src/policy/gate.py:325-327`). Copies do not widen reads of `~/hydra/runs/`, sibling worktrees, or the store: absolute paths naming those prefixes are still refused (`src/policy/gate.py:311-314`, `src/policy/mod.rs:554-604`). The seatbelt still denies kernel writes to Hydra directories on macOS (`src/sandbox.rs:75-88`).

## Strongest argument against

Harness spec 6.4 chose links so a run cannot edit its own instructions mid-session (`docs/harness-spec.md:293-303`, `src/skills/mod.rs:13-17`). Copies traded for shell access reintroduce that guarantee gap on `codex` only, and fixing it properly means teaching the shell check to distinguish reads from writes on skill-root paths, which is option 2's unsolved problem.

## The four ticket options

**1. Copy on codex (recommended, scoped).** Smallest fix that matches how this provider actually reads; isolates mutability to one worktree instead of the shared snapshot.

**2. Read-only shell carve-out.** Would need read/write discrimination the gate deliberately avoids (`src/policy/gate.py:270-276`, `docs/harness-spec.md:889-893`). A carve-out on `skill_snapshot` also reopens direct shell access to snapshot bytes under `~/hydra`.

**3. Do not inject on codex.** Honest but wastes the injection path Hydra already runs; `skill_invoke` is already empty (`src/policy/table.rs:294-296`), so the run gets listing metadata without bodies.

**4. Leave it and tell the run.** Documents a refusal the preamble already implies; run `01a0cc2a-f20b` shows the model stopping rather than proceeding without the skill.
