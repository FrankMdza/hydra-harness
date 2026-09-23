# Panel 73, composer

## 1. Recommendation

**Option 1: an allowlist.** Rung two may edit only named subtrees (at minimum `src/provider/`, `src/normalize/`, `src/preamble.rs`, and `config/preamble.md` per the ticket). Everything else, including all of `tests/`, is refused at `protected_path_hit` (`evolve.rs:166-186`) before judges run. The denylist at `PROTECTED_CODE_PATHS` (`evolve.rs:448-465`) has already missed three times (`evolve.rs:459-460`); widening it is the same failure mode on a longer timer.

## 2. Mechanism

**`src/rungs/evolve.rs` (~50 lines).** Replace `PROTECTED_CODE_PATHS` with `EDITABLE_PATHS` (or invert the check). Rewrite `protected_path_hit` (`evolve.rs:469-492`): for each path from `changed_paths` (`evolve.rs:1140-1158`, fed by `read_proposer_patch` at `evolve.rs:400-428`), refuse unless it lies inside an editable root; keep the five config rows (`evolve.rs:470-477`) always protected. Admission still runs at `evolve.rs:166` before `admission_gate` (`evolve.rs:507-528`).

**`docs/harness-spec.md` section 15.2 (~20 lines prose).** Record that admission is allowlist-shaped for source, not denylist-shaped; map rows 1-2, 4, 6-7, plus `sandbox.rs` and `containment.rs` (`harness-spec.md:1462-1468`), to "not editable by rung two."

**`tests/evolve.rs` (~30 lines).** Replace `a_diff_outside_the_protected_rows_passes_admission` (`evolve.rs:291-308`), which today asserts `src/run.rs` is editable, with allowlist positives and negatives. Extend `a_diff_naming_a_code_row_of_harness_15_2_is_refused_before_any_judge` (`evolve.rs:227-288`) to walk `EDITABLE_PATHS` boundaries.

**Test proof:** `tests/evolve.rs::a_diff_naming_a_code_row_of_harness_15_2_is_refused_before_any_judge` already drives `protected_path_hit`; new cases for `src/run.rs`, `src/pty.rs`, `tests/gate.rs` must fail before the change and pass after. Would fail today: `evolve.rs:295-308` expects `src/run.rs` and `tests/evolve.rs` to pass.

## 3. What it breaks or costs

Proposers **cannot add or edit any test** under Option 1; patches arrive without test coverage of their own change, so the human merge must add tests or accept judge-only evidence. **Every new editable area needs a human edit** to `evolve.rs`, which is already protected (`evolve.rs:450`). Past passes that an allowlist would have refused: **`src/paths.rs`** (pass `58fcb71`, 122 lines added), **`src/record/validate.rs`** (`ca5f54b`), and the large rename in `dd0ab72` touching `src/run.rs`, `src/cli.rs`, docs, and `tests/cleanup.rs`. Passes that would still pass: **`src/normalize/*.rs`** (`51357f8`), **`src/provider/codex.rs`** (`5967077`), mostly provider-facing work. Measure whether that split matches what rung two should do before freezing the list.

## 4. What a hostile run or proposer still gains

Under an allowlist, a proposer can still **edit everything inside `src/provider/` and `src/normalize/`**, including resume argv, hook wiring, and record normalization. A patch could **weaken provider behavior** (for example soften a refusal path in `src/normalize/claude.rs`) while leaving gates untouched; admission runs `cargo test` (`evolve.rs:527`) on the patched tree but cannot catch a change that keeps tests green while widening runtime behavior. **`docs/` and `integrations/`** stay editable unless added to the allowlist; poisoned public docs are not blocked. The proposer runs under `standard` with real `HOME` (`rungs-spec.md:405-416`); network and shell exfiltration remain as documented in `SECURITY.md:40-51`.

## 5. The strongest argument against your choice

Rung two's legitimate wins have been cross-cutting: layout extraction into `src/paths.rs`, evolve fixes in `src/rungs/evolve.rs` before it was protected, and harness hardening outside provider/normalize. A short allowlist freezes those paths behind human-only edits and may stall the loop the spec describes (`rungs-spec.md:31-34`). The denylist failed from omission, not from the idea of naming dangerous rows; a generated allowlist derived from harness 15.2 roles might be wider but still closed. Option 1 trades repeated security misses for slower iteration on core infrastructure.

## 6. The other options

**Option 2** adds four files and moves eight probes into `tests/corpus/`; it does not protect `tests/gate.rs`, `tests/spawn.rs`, or `src/face.rs` where `tests/corpus/README.md:10-22` says the probes live, so a proposer can still weaken the probe and the gate in one patch. **Option 3** protects `tests/` but forbids proposer-written tests entirely without giving judges a replacement mechanism; same test gap as Option 1, with a wider denylist to maintain.

## 7. Something the ticket missed

**Yes, there is a better combination than the three options alone:** Option 1 plus **build ticket 80** (`80-baseline-test-provenance.md`), which refuses edits to tests present at `base_tree` (`rungs-spec.md:468-475`). Option 1 closes the mechanism boundary; ticket 80 closes the "weaken an existing test" hole if the allowlist is ever widened to include `tests/`. The ticket understates that `tests/corpus` protection (`evolve.rs:464`) does not protect the probes: eight of fourteen README rows point at `tests/gate.rs`, `tests/spawn.rs`, `tests/mcp.rs`, `tests/delegation.rs`, and `src/face.rs` (`tests/corpus/README.md:10-22`), while `tests/evolve.rs:291-308` explicitly treats `src/run.rs` as fair game. `harness-spec.md:1429-1432` notes `src/run.rs` held the spawn writer until ticket 66; it is still the launch sequence and remains unprotected in code.
