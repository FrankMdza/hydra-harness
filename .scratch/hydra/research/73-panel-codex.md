# Panel 73, codex

## 1. Recommendation

Choose option 1, but use exact files rather than the ticket's example directory grants. Start with `src/normalize/agy.rs`, `src/normalize/claude.rs`, and `src/normalize/cursor.rs`; keep all other paths closed until a human deliberately widens the compiled list. This permits useful normalization work while acknowledging that normalization itself remains security-sensitive (`src/normalize/agy.rs:4`, `src/normalize/claude.rs:26`).

## 2. Mechanism

In `src/rungs/evolve.rs`, replace `PROTECTED_CODE_PATHS` and invert `protected_path_hit` into refusal of every path not explicitly allowed (`src/rungs/evolve.rs:448`, `src/rungs/evolve.rs:469`). Preserve whole-patch refusal before admission and judges (`src/rungs/evolve.rs:166`, `src/rungs/evolve.rs:230`). Reject absolute paths, parent traversal, unexpected casing, symlinks, and non-regular entries; compare normalized repository-relative Git names, not filesystem aliases. Inspect both sides of renames. Approximately 70–110 lines, including a shared validation helper and a brief generated from the same list.

Keep the recorded seeded tree as the comparison baseline: `read_proposer_patch` reads `base_tree` and derives changed paths against it (`src/rungs/evolve.rs:409`, `src/rungs/evolve.rs:420`). Retain NUL-delimited enumeration (`src/rungs/evolve.rs:1144`). Apply the same check on re-review: `read_blocked_patch` currently uses newline-separated names, and its caller proceeds to judges without this validation (`src/rungs/evolve.rs:739`, `src/rungs/evolve.rs:837`, `src/rungs/evolve.rs:844`). Approximately 30–50 additional lines.

In `tests/evolve.rs`, replace permissive expectations and add synthetic patches naming a new mechanism file, `tests/gate.rs`, `src/face.rs`, `Cargo.toml`, an allowed file, and a mixed patch. Existing tests expressly permit `src/lib.rs` (`tests/evolve.rs:218`). New negative cases fail without the change because those paths are absent from the current denylist (`src/rungs/evolve.rs:448`). Add rename, mode-change, and re-review cases proving refusal before judge dispatch. Approximately 120–180 test lines.

Update `docs/rungs-spec.md:424` and `docs/harness-spec.md:1396` to describe default refusal and the exact admitted surface, approximately 40–60 lines. No assertion relocation is necessary: the protected corpus module supplies helpers, while its inventory points to probes elsewhere (`tests/corpus/mod.rs:1`, `tests/corpus/README.md:10`).

## 3. What it breaks or costs

It prevents adding tests, changing existing tests, adjusting launch behavior, and fixing most core defects through rung two. Existing inline tests inside the three allowed files remain editable; this is not complete baseline-test provenance. Build 80's promise therefore remains separate work (`.scratch/hydra/build/80-baseline-test-provenance.md:9`).

Historical evidence has limits. The requested `git log --oneline --all | grep -i rungs/` returned no matches, but one `rungs/` branch and seven legacy `ouroboros/blocked/` branches remain. Parent-to-pass diffs can include seeded human work, whereas admission compares against the recorded seed (`src/rungs/evolve.rs:320`, `src/rungs/evolve.rs:409`); I did not read prohibited run records.

The accepted pass's actual landed correction is recoverable in commit `e0c782b`: help text and a comment, `src/cli.rs:158` and `src/run.rs:202` at that revision. This allowlist would refuse it. Pass `58fcb71` changed the absent-run test (`58fcb71:tests/cleanup.rs:650`); the current source independently documents that proposer weakening (`src/rungs/evolve.rs:330`). It would be refused. Commit `51357f8` names only the three normalization files in its parent diff, so that visible path set fits; seed-relative attribution remains unconfirmed. Commits `8856a81` and `5967077` include CLI/evolution changes and would fail if those are proposer changes. Do not report a numerical acceptance rate from these snapshots.

## 4. What a hostile run or proposer still gains

An allowed normalizer can suppress or misclassify permission evidence: the agy adapter derives permission decisions, and Claude imports decision and replay machinery (`src/normalize/agy.rs:4`, `src/normalize/claude.rs:26`). The allowlist protects other files, not the truth of this output. A malicious Rust change in an allowed file can also execute during admission; admission runs on the patched worktree (`src/rungs/evolve.rs:230`). Judges and unchanged external tests remain necessary.

## 5. The strongest argument against your choice

This shrinks “core evolution” into three adapters and rejects a demonstrated harmless improvement. Meanwhile, any allowed Rust file can contain dangerous executable behavior. The benefit is a smaller review surface and default protection for newly added mechanisms, not a semantic proof that permitted changes cannot undermine Hydra.

## 6. The other options

Option 2 still requires identifying every mechanism and every probe correctly. Build 79 moved helpers, not the assertions its protection claim depended on (`.scratch/hydra/build/79-the-attack-corpus.md:15`, `.scratch/hydra/build/79-the-attack-corpus.md:31`).

Option 3 closes integration-test editing but leaves new source mechanisms and unlisted inline probes exposed; A12 resides in `src/face.rs` (`tests/corpus/README.md:21`).

## 7. Something the ticket missed

No, I do not recommend a better fourth option now. An allowlist plus separately admitted new tests could follow, but adds execution authority and does not preserve inline baseline assertions automatically.

The example allowlist includes the admission predicate itself through `src/normalize/paths.rs::inside` (`src/rungs/evolve.rs:486`, `src/normalize/paths.rs:219`), hook-configuration refusal through `src/provider/mod.rs::claim` (`src/provider/mod.rs:271`), and an expressly protected preamble (`src/rungs/evolve.rs:473`, `docs/harness-spec.md:1421`). Default denial helps only if its exceptions do not reopen the mechanisms being protected.
