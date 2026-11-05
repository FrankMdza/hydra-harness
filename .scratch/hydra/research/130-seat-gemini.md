# 130, seat Gemini: the docket checked against the tests

Read on 2026-10-15 at `c5c00f5`. Role: the seat that checks the tests. For each item, this memo
evaluates whether the test named in `.scratch/hydra/research/130-panel-readings.md` would actually fail
if the problem were not solved, verifies the source lines cited, and specifies the exact tests
required before closing each item. Nothing was executed via `cargo` or the test runner; every finding
is drawn directly from code and file inspection inside the repository.

---

## 1. The Linux job

**Recommendation: Narrow the CI job immediately to what is true today, but fix Option 1's command
first; open targeted tickets for the eight groups to widen CI before public release; for the three
decisions: (c) commit with a fixed fallback literal identity (`Hydra <hydra@localhost.invalid>`) only
when unset, (d) refuse an embedded NUL in `cwd` explicitly with `hydra/bad-payload` before any
filesystem call, and (g) maintain bit-exact matching per platform with `cfg` gates rather than
introducing a 1-ulp tolerance.**

### Analysis of claims and line citations:

- **a. `tests/seats.rs:47`.**
  - *Repo check:* Line 47 is `Layout::at(&root).ensure().unwrap();`. Lines 44-46 format
    `root` as `/private/tmp/hs{}-{}`, hardcoding `/private/tmp`. The line says exactly that.
  - *Would the readings' test fail?* Yes. On Linux `/private` does not exist, so directory creation
    panics with `Permission denied` or `No such file or directory`.
  - *Demanded test:* A cross-platform root path helper (using `/tmp` on non-macOS or `std::env::temp_dir()`)
    and a test asserting that `root()` creates a path adhering to UNIX domain socket length limits
    (104 bytes on macOS, 108 bytes on Linux) and that socket binding succeeds.

- **b. `tests/corpus/mod.rs:675`.**
  - *Repo check:* Line 675 is `panic!("sandbox-exec is required on CI (macOS)");` inside
    `seatbelt_exec_or_skip`. It checks `std::env::var_os("CI").is_some()` without checking `target_os`.
    The line says exactly that.
  - *Would the readings' test fail?* Yes. GitHub Actions sets `CI=true` on Ubuntu runners, and since
    `sandbox-exec` does not exist on Linux, every test calling this guard panics.
  - *Demanded test:* A test verifying that `seatbelt_exec_or_skip` on Linux with `CI=true` logs the
    skip and returns `false` without panicking, while on macOS with `CI=true` it panics if `sandbox-exec`
    is absent.

- **c. `tests/git_config.rs:353` and `src/worktree.rs:266`.**
  - *Repo check:* `tests/git_config.rs:353` is `let commit = committed.unwrap().expect("the checkout held work");`.
    `src/worktree.rs:256-279` is `identity()`, which returns empty args when neither `user.name` nor
    `user.email` is set.
  - *Critical test finding:* The readings claim: *"For group c the test is `tests/git_config.rs` run in
    a child with no `HOME` git config, which is what the runner already does."* **That is FALSE.**
    In `tests/git_config.rs:22-45`, `isolated_test` redirects `XDG_CONFIG_HOME` to a scratch directory,
    but **inherits `HOME` and the system git configuration.** On the developer's Mac, the test passes
    because it reads the human's global `~/.gitconfig`! On a fresh Linux CI runner without `~/.gitconfig`,
    `identity()` finds nothing, git's guess fails, and `commit_everything` fails. On macOS, the existing
    test gives a **FALSE GREEN**.
  - *Decision c (Identity):* When `user.name` or `user.email` is unset in git configuration, reap and
    curate must supply fixed fallback literals: `user.name=Hydra` and `user.email=hydra@localhost.invalid`.
    Never use the OS username or machine hostname (which leak local identity, as ticket 316 noted).
  - *Demanded test:* An isolated child test in `tests/git_config.rs` that explicitly sets `HOME` to an
    empty scratch directory, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_SYSTEM=/dev/null`, and
    `GIT_CONFIG_NOSYSTEM=1`, asserting that `commit_everything` succeeds and `git log -1 --format='%an <%ae>'`
    outputs `Hydra <hydra@localhost.invalid>`. A second case where only `user.name` is configured must
    verify that the configured name is preserved alongside `hydra@localhost.invalid`.

- **d. `tests/gate.rs:3464`.**
  - *Repo check:* Line 3464 is `assert!(decision.allowed && decision.reason.starts_with("hydra/allow"), ...);`
    in `a_single_call_whose_cwd_holds_a_nul_is_answered_as_before_the_batch_entry`. In `src/policy/gate.py:1425-1428`,
    `cwd = payload.get("cwd")` is passed to `resolve_shown`, which calls `os.path.realpath(path)`.
    On Python 3.12 (Ubuntu), `os.path.realpath` raises `ValueError: embedded null byte`. `decide_payload`
    catches `Exception` and returns `hydra/gate-error: the gate raised ValueError...`.
  - *Would the readings' test fail?* Yes. On Ubuntu Python 3.12, the test fails because it asserted `allowed`.
  - *Decision d (NUL cwd):* POSIX paths cannot contain NUL bytes. The gate must refuse up front with
    `hydra/bad-payload` whenever `payload.get("cwd")` contains `\0`, before calling `os.path`.
  - *Demanded test:* A gate test asserting that any payload with `\0` in `cwd` is denied with
    `decision.allowed == false` and `decision.reason.starts_with("hydra/bad-payload")` across both Python 3.9
    and Python 3.12+.

- **e. `tests/pty_tee.rs:174`.**
  - *Repo check:* Line 174 is `assert!(serde_json::from_str::<serde_json::Value>(line).is_ok(), "a frame was cut at a read boundary");`.
  - *Would the readings' test fail?* Yes. Linux ptys deliver in 4 KiB chunks and default canonical line
    discipline can split or buffer long frames differently than macOS.
  - *Demanded test:* A test verifying that the pty slave is set to raw mode, that a 200,000-byte JSON
    payload sent through the pty arrives in exactly 1 frame (`seen == 1`), and that the parsed JSON payload
    matches the input byte-for-byte.

- **f. `src/record/validate.rs:31` and `tests/detached.rs:388-448`.**
  - *Repo check:* Line 31 is `assert!(self.ok(), ...)` inside `Report::assert_ok`.
  - *Critical test finding (The Unmeasured Root Cause Discovered):* Both the readings and Opus noted
    `line 0: a Hydra line of type ""` as "Unmeasured". Reading `tests/detached.rs:396-405` reveals the
    exact bug: the test manually constructed line 0 as `serde_json::json!({ "kind": "run_start", ... })`.
    In `src/record/lines.rs:18`, `RunStart` has `#[serde(rename = "type")] pub kind: &'static str`.
    The validator in `src/record/validate.rs:284` extracts `body.get("type").and_then(Value::as_str).unwrap_or("")`.
    Because `"type"` was missing, it extracted `""` and reported `line 0: a Hydra line of type ""`.
    Why did it pass on macOS? In `tests/detached.rs:426`, macOS hits `if !spawned.pty_still_held { return; }`
    and **exits early before line 448 (`check(&text).assert_ok()`) is ever reached!** On Linux,
    `pty_still_held` is true, so it executed line 448 and failed on the test's own malformed JSON!
  - *Demanded test:* Fix line 0 in `tests/detached.rs` to write `"type": "run_start"` (or use `minimal_run_start`),
    and ensure `check(&text).assert_ok()` is exercised unconditionally on all platforms.

- **g. `crates/puppets/src/sheet.rs:644`.**
  - *Repo check:* Line 644 is `(4.0, 1.791759469228055)`. In `crates/puppets/src/sheet.rs:70-82`,
    `c_mul_add` branches on `cfg!(target_arch = "aarch64")`. On x86_64, `a * b + c` is computed without
    fused multiply-add, producing `1.7917594692280554` (1 ulp difference).
  - *Critical test finding:* The readings claim Option 1 (`cargo test --release --lib --workspace`) would
    be green. **That is FALSE.** `crates/puppets/src/sheet.rs` is part of `crates/puppets` library, and
    `sheet::tests::lgamma_is_cpythons_and_not_the_c_librarys` is a lib test (`#[test]` inside `src/sheet.rs`).
    Running `--lib` still runs it and fails on Linux!
    Furthermore, the readings claimed the differential runs against the runner's live CPython. **FALSE:**
    `crates/grader/tests/differential.rs:7` explicitly states *"No test starts `python3`"*; it compares
    against static recorded fixtures in `tests/fixtures/python/`.
  - *Decision g (CPython match vs 1-ulp tolerance):* Match CPython per platform; do NOT use a 1-ulp
    tolerance. As build ticket 207 showed, a single last bit swapped the rankings of two agents.
  - *Demanded test:* Gate the bit-exact tests to `cfg(all(target_os = "macos", target_arch = "aarch64"))`,
    and assert platform-specific behavior with dedicated fixtures when Linux ranking is supported.

- **h. `crates/puppets/tests/differential.rs:3863` and `crates/grader/tests/differential.rs:147`.**
  - *Repo check:* Both lines are assertion checks (`assert!(mismatches.is_empty(), ...)` and
    `assert!(self.differing.is_empty(), ...)`).
  - *Would the readings' test fail?* Yes. macOS-recorded fixtures contain `__CF_USER_TEXT_ENCODING` and
    differing shell stub error prefixes (`/bin/cat` under dash vs `cat` under zsh).
  - *Demanded test:* Environment sanitization in the test harness (stripping `__CF_*` macOS-only vars)
    and path normalization for shell binary names in stderr stubs before comparing against fixtures.

---

## 2. Build 316

**Recommendation: Additive fields only (`run_start.host: {os, arch, machine}` and additive `sandbox_kind: "seatbelt"`).
Do NOT bump to schema version 2; keep `schema_version = 1`.**

### Evidence and Test Analysis:
- *Repo check:* Section 15 of `docs/run-record-contract.md` (lines 1416-1420) states:
  *"Adding a `run_start` field is not a version change, and removing or re-typing one is."*
  `src/record/mod.rs:57` sets `pub const SCHEMA_VERSION: u32 = 1;`.
  No normalizer, validator, or CLI command in `src/normalize/`, `src/walk.rs`, or `src/show.rs` branches
  on `schema_version` or reads `run_start.sandbox` today.
- *Would the readings' tests fail if not solved?*
  The readings' tests (`tests/record_contract.rs` accepting records with and without `host`, `fixture_corpus.rs`
  guarding fixtures, `doctor --walk`) only check the **happy path**. They would **NOT catch malformed `host` objects!**
  Per Contract Section 15: *"A group's leaves are checked too, not only the group... All four leaves of 5.4
  are required when auth is present..."*. The validator must check `host` leaves.
  Moreover, the readings' test suite does not guard against `fixture_corpus.rs` writing the developer's
  actual hostname into public fixtures.
- *Demanded tests:*
  1. **Leaf validation tests in `tests/record_contract.rs`:**
     - Validator accepts valid `host: {os, arch, machine}` and accepts absence of `host` under schema v1.
     - Negative test: Validator explicitly rejects `host: null`.
     - Negative test: Validator explicitly rejects `host` missing any leaf (`os`, `arch`, or `machine`).
     - Negative test: Validator rejects non-string leaf values.
  2. **Fixture hygiene test:** A test asserting that all fixtures under `tests/fixtures/records/` contain
     synthetic hostname values (e.g., `"stub-host"`) and none match `gethostname()`.
  3. **Backward compatibility test:** `hydra doctor --walk` over the existing 952 records reports zero findings.

---

## 3. Build 326

**Recommendation: Treatment one (commit nested files as plain files, drop the gitlink, clean the checkout),
but amend the implementation to move the nested `.git` OUT of the checkout before committing rather
than renaming to `.git.nested`.**

### Evidence and Test Analysis:
- *Repo check:* In `tests/cleanup.rs:1934-1940`, the staged case fails because `git status --porcelain`
  exits non-zero with `failed in submodule sub`. In `tests/cleanup.rs:2013-2016`, the unstaged case fails
  because `git worktree remove` exits 128 (`'sub/.git' not recognized as a git repository`), leaving a
  gitlink `160000 commit` on the branch.
- *Readings error:* The readings claim both cases are *"red tests in tests/cleanup.rs today"*. **FALSE.**
  They are green regression tests that currently pin the failed reap behavior. The readings also transposed
  the error messages between staged and unstaged cases.
- *Would the readings' tests fail if not solved?*
  The readings propose: flipping the two 325 cases, adding a case with two nested repos, and checking
  `git fsck --unreachable` empty.
  **This test misses a catastrophic failure mode:** If reap renames `.git` to `.git.nested` inside the
  checkout, the subsequent `git add -A` will commit `.git.nested/` (all HEAD, config, index, logs, sample
  hooks) as plain tracked files into the branch! The readings' test does not verify branch tree cleanliness.
  Additionally, if the nested repo is a submodule or worktree, `.git` is a **file** (`gitdir: ...`), not
  a directory; a directory-only scanner misses it.
- *Demanded tests:*
  1. **Branch cleanliness test:** Assert that `git ls-tree -r --name-only` on `hydra/<run-id>` contains the
     nested files (e.g. `sub/note.txt`) and contains **NO path matching `**/.git/**` or `**/.git.nested/**`**.
  2. **Gitlink absence test:** Assert that `git ls-tree -r` on the branch contains **zero entries of mode `160000`**.
  3. **Nested `.git` file test:** A test case where `.git` is a file pointing to a gitdir, ensuring it reaps
     cleanly as plain files.
  4. **Double nested repo test:** A checkout with two nested repos (`sub1/` and `sub2/`).
  5. **Clean reap outcome:** Assert that reap returns `Reaped::Clean` (exit 0) and the checkout is completely removed.

---

## 4. Build 321

**Recommendation: Accept "written by the hook, not tamper-proof" as the evidence's standing in the
contract. Do NOT sign the sidecar lines. Implement reader-side corroboration checks in `hydra doctor --walk`.**

### Evidence and Test Analysis:
- *Repo check:* In `src/sandbox.rs:183-185`, the seatbelt profile permits reading the run's own spawn directory
  (`policy::own_read_paths(spawn)`), because the provider reads its hook config, MCP config, and gate script.
  `src/sandbox.rs:235-240` grants a kernel write allowance for `gate-refusals.jsonl`.
  Because the gate hook runs as a subprocess of the provider under the same sandbox profile, any signing
  secret readable by the hook is readable by the provider and shell. Cryptographic signatures within the
  same sandbox cannot establish authenticity against an adversarial agent in that sandbox.
- *Would the readings' tests fail if not solved?*
  The readings' tests (`tests/gate.rs:40-105`) verify happy-path recording and 64KB truncation. They would
  **NOT fail if an adversary appended forged lines to the sidecar.**
- *Demanded tests:*
  1. **Sidecar corruption recovery test:** A test where `gate-refusals.jsonl` contains truncated, non-JSON,
     or garbage lines mid-file. Assert that `copy_gate_refusals` does not panic, notes the error on
     `run_end.error`, and closes the record cleanly.
  2. **Corroboration audit test in `tests/walk.rs`:** A test where a synthetic record carries a `gate_refusal`
     event whose `call_id` has no matching tool call in the provider stream, or matches a tool call that
     succeeded. Assert that `hydra doctor --walk` flags this as an uncorroborated refusal.

---

## 5. Build 320

**Recommendation: Leave the three facts in text. Do not spend four drift probes to add JSON fields to
hook replies. If structured fields are desired, write them to the Build 321 sidecar (`gate-refusals.jsonl`),
which Hydra controls directly.**

### Evidence and Test Analysis:
- *Repo check:* `docs/harness-spec.md:503` notes that `claude` can fail open if a hook reply dialect is
  unrecognized. No provider inspects custom JSON fields in hook denials.
  In `tests/gate.rs:3086-3160`, three unit tests verify the facts in text:
  `a_refusal_names_the_longest_deny_entry_that_matched`, `a_refusal_quotes_the_token_as_the_command_wrote_it`,
  and `a_refusal_names_the_link_a_path_went_through`.
- *Would the readings' tests fail if not solved?*
  Yes, reverting `longest` to first-match fails `a_refusal_names_the_longest_deny_entry_that_matched`.
- *Demanded tests:*
  1. **Single-line formatting test:** Assert that multiline tokens or special characters in refusal reasons
     are escaped to a single line (preventing `codex` stderr line-splitting).
  2. **Reason prefix stability test:** Assert that all refusals retain the canonical `hydra/protected-path:`
     or `hydra/out-of-scope:` prefix so normalizers and `doctor --walk` extract the correct `reason_code`.
  3. **Token length cap test:** Assert that tokens exceeding 200 characters are capped in the refusal text.

---

## 6. Map 126

**Recommendation: For Question 1, mark `Foreign` and count under foreign host; do NOT replay with the
record's layout. For Question 2, refuse mixed-host corpora at load in Rungs until Build 318 lands.**

### Evidence and Test Analysis:
- *Repo check:* `src/rungs/evolve.rs:591-636` defines `PROTECTED_CODE_PATHS`, which includes `src/policy`,
  `policies`, `src/sandbox.rs`, and `src/containment.rs`. Any evolve patch touching these is rejected
  before review. `src/rungs/curate.rs` only modifies skills (`.claude/skills/...`).
- *Readings error:* The readings' claim for Question 2—*"exclude them from proposals that touch the gate"*—describes
  an **empty set.** No Rungs proposal ever touches the gate.
  In `src/normalize/mod.rs:737-758`, `GateMark::Foreign` already exists for gates compiled under differing layouts.
- *Would the readings' tests fail if not solved?*
  The readings' test—*"a `tests/curate.rs` case where a hook-only record is present in the corpus and absent
  from the evidence of a proposal that touches the gate"*—is **impossible and invalid** because curate
  never touches the gate.
- *Demanded tests:*
  1. **Foreign host walk test in `tests/walk.rs`:** A fixture record with `host.os = "linux"` walked on
     macOS asserts that `doctor --walk` does not panic, does not run invalid local `realpath` checks,
     and reports the record under a dedicated `foreign_host` count rather than a verification failure.
  2. **Multi-host corpus rejection test in `tests/curate.rs`:** Assert that passing a corpus containing
     records from multiple distinct `host.os` values is rejected at load with an explicit error naming both hosts.

---

## 7. Map 127

**Recommendation: Yes to Question 1: exempt the run's own `skill_snapshot` directory from the shell
token pass, ONLY when `run_start.sandbox` is non-null. Question 2: Own snapshot only (not `spawn/`).
Question 3: Moot.**

### Evidence and Test Analysis:
- *Repo check:* In `tests/gate.rs:1328-1345`, native file tools (`gate.read_from(&link.join("reference.md"))`)
  are **already allowed** to read skill links into `skill_snapshot`. The restriction is purely on the shell
  string floor.
  `tests/gate.rs:2990-3040` currently asserts that `cat .claude/skills/unslop/SKILL.md` is denied.
- *Would the readings' tests fail if not solved?*
  The readings propose flipping `tests/gate.rs:2990-3040` to assert allowed, plus a live measurement.
  **This test checks the happy path but completely misses security containment!**
- *Demanded tests:*
  1. **Sandboxed shell read allowed:** Assert `cat .claude/skills/unslop/SKILL.md` is allowed when `sandbox` is active.
  2. **Unsandboxed refusal test (CRITICAL):** Assert that if `sandbox` is null, the exemption is **NOT applied**
     and the shell command remains refused (`hydra/protected-path`).
  3. **Foreign snapshot refusal test:** Assert that a shell command referencing another run's snapshot
     (`~/hydra/skills-snapshots/<other-sha>/...`) remains refused.
  4. **Spawn directory refusal test:** Assert that shell commands referencing `~/hydra/spawn/<run-id>/` remain refused.
  5. **Kernel write denial test:** A sandbox test asserting that an attempted write through the link
     (`echo bad > .claude/skills/unslop/SKILL.md`) fails with `EPERM` at the kernel level.

---

## 8. The publication list from research 129

**Recommendation: Fix the release version to `v0.3.0` (NOT `v0.1.0`). Immediately add the two untracked
labeling files and `pendintes_hydra.md` to `.gitignore`. Order: Secret scan -> `.gitignore` -> CI green ->
Release `v0.3.0` -> Metadata -> Personal paths -> Keep commit email -> `git gc`.**

### Evidence and Test Analysis:
- *Repo check:*
  - `Cargo.toml:6` is `version = "0.2.0"`.
  - `CHANGELOG.md:103` already has `## 0.2.0, 2026-09-08`. Releasing `v0.1.0` would release an older
    version than what is already shipped in the changelog! The next release is `v0.3.0`.
  - `git status --porcelain` reveals:
    ```
    ?? .scratch/hydra/labeling/2026-09-29/classes-c-agy-discarded.jsonl
    ?? .scratch/hydra/labeling/2026-09-29/shard-c.jsonl
    ?? pendintes_hydra.md
    ```
    The readings mentioned only `pendintes_hydra.md`, completely missing the two untracked labeling files!
  - `.github/workflows/security.yml` is **not protected ground** (it does not appear in `PROTECTED_CODE_PATHS`
    in `src/rungs/evolve.rs:591-636`).
  - `sha2 = "0.10"` is pinned in four `Cargo.toml` files (`.`, `crates/pyjson`, `crates/records`, `crates/grader`).
    `format!("{:x}", Sha256::digest(..))` is used across nine sites.
- *Would the readings' tests fail if not solved?*
  The readings test `git add -A --dry-run` only against `pendintes_hydra.md`. It would **allow the two
  labeling files (198 KB) to be committed to the public repo!**
- *Demanded tests:*
  1. **Clean staging test:** `git add -A --dry-run` must produce NO output for `pendintes_hydra.md` AND
     NO output for `.scratch/hydra/labeling/`.
  2. **Version consistency test:** Assert that git tag matches `Cargo.toml` (`0.3.0`) and the top section
     of `CHANGELOG.md`.
  3. **Fixed-vector `sha2` regression test:** A test asserting that `sha2` formatting produces exact expected
     64-character lowercase hex strings across known test vectors, ensuring `gate_sha` and `sandbox` sha
     never drift.

---

## 9. Unticketed items

**Recommendation: Rank the items as follows:**
1. **Rank 1 (Security finding): `Ungated { tool: "Edit" }` on build 327 (`01a13e24-243b`).**
   Open immediately. `src/normalize/claude.rs:615` flags a tool call that executed without a gate
   decision. This is a potential hook evasion and must be audited before release.
2. **Rank 2 (CI reliability): Wall-clock timing in `tests/cockpit.rs:3865-3875`.**
   Open immediately. `assert!(waited >= Duration::from_secs(5) && waited < Duration::from_secs(7))`
   causes flaky CI failures under runner load, identical to the pattern build 330 removed.
3. **Rank 3 (Feature): `hydra show --verdict`.**
   Open. Clean CLI enhancement allowing quick inspection of provider vs gate verdicts.
4. **Do not open: JSON refusal fields behind four probes.**
   Facts are already in text, and structured data belongs in the 321 sidecar.
5. **Do not open: OpenCode as fifth provider.**
   Premature before public release; existing four providers require Linux containment first.

### Demanded tests:
- For Ungated Edit: A normalizer regression test ensuring that any tool call missing both gate decisions
  and input-level refusals is flagged as a fatal finding.
- For cockpit test: Replace the wall-clock sleep assertion with an event-driven timer / channel assertion.

---

## Vote

Narrow the Linux CI job immediately with `sheet::tests::lgamma_is_cpythons_and_not_the_c_librarys`
explicitly skipped or gated, since the readings' `--lib` command fails on x86_64, and resolve the
eight groups with exact tests before public release.
Adopt additive designs on all record questions—keeping schema version 1 with `host` leaves validated,
accepting sidecar evidence without signatures, leaving refusal facts in text, and having reap commit
nested files while moving `.git` out of the tree.
Prioritize public readiness by tagging `v0.3.0` rather than `v0.1.0`, ignoring the untracked labeling
files, and investigating the `Ungated { tool: "Edit" }` security finding before any release.

---

## What I could not verify

- The raw GitHub Actions log for run 38695261071 (external network was not accessed).
- The 27 unapplied files of Build 316 in worktree `01a13dff-4745-7361-a3b5-d3dc98cf6d74` (sits outside this worktree).
- The full record of run `01a13e24-243b` (reading `~/hydra/runs/` is denied to this run).
- The live startup failure message of the `security` workflow on GitHub Actions.
- Neither `cargo` nor the test suite was run, in strict adherence to panel instructions.
