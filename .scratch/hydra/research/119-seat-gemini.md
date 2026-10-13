# Panel Memo: Ticket 119 — Linux VM Seatbelt & Sandbox Architecture

Seat: Gemini
Date: 2026-10-13
Ticket: .scratch/hydra/issues/119-hydra-on-linux-has-no-seatbelt.md

## Question 1

1. **What the code does today:**
   `src/sandbox.rs` lines 46–48 defines `wraps_launches()` as strictly `cfg!(target_os = "macos")`. In `src/spawn_config.rs` lines 122–128, non-macOS platforms emit `(None, None)` for `(sandbox_profile, sandbox_sha)`. Consequently, `src/run/mod.rs` lines 612–618 and 638 records `sandbox: null` in `run_start`. While macOS halts bare execution in `src/run/mod.rs` lines 864–870 (`SandboxFailed("macos requires sandbox-exec")`), Linux falls straight through to `pty::spawn_with_pty` (`src/pty.rs` line 100), running completely unconfined. Furthermore, `src/sandbox.rs` line 83 returns `false` for `inside_seatbelt()`, bypassing child launch guardrails in `src/run/mod.rs` line 295. `policy_sha` (`src/policy/table.rs` lines 77–104, `src/doctor.rs` lines 68–80) is the cryptographic hash of `policies/*.json` (protected config #1) and cannot encode operating system or sandbox execution state.

2. **What was measured or verified:**
   In `tests/spawn.rs` (lines 103, 137, 169, 192, 219, 239, and 260), seven sandbox enforcement tests explicitly skip when `!sandbox::wraps_launches()`. Under `policies/standard.json` line 8, the `shell` tool is granted with unrestricted path parameters (`"paths": []`). On macOS, `src/sandbox.rs` lines 118–124 enforces kernel-level denials against modifying `~/hydra/runs/`, `policies/`, and credentials. On Linux, hook-only gating leaves shell subprocesses completely unconfined: an agent can directly mutate run records, alter protected policies, or read cloud tokens.

3. **Recommendation:**
   Recorded runs must remain on macOS until Linux implements an OS-level sandbox wrapper, because hook-only execution forfeits the tamper-proof guarantees required of run records. Creating a bifurcated "hook-only" population with an adulterated `policy_sha` would erode the evidentiary integrity of the entire corpus.

4. **What would change your mind:**
   If runs were executed inside ephemeral, single-use throwaway containers (or unprivileged microVMs) stripped of any host credentials or write access to `~/hydra/runs/`, and where the policy strictly prohibited shell tool execution.

## Question 2

1. **What the code does today:**
   `src/sandbox.rs` lines 105–277 generates an Apple Seatbelt profile using `(version 1)` and `(allow default)`, followed by negative deny rules (`deny file-write*`) for `~/hydra/runs`, `policies/`, `config/`, the seed repository checkout, `~/.ssh`, and `~/.aws`, re-allowing writes only to the run worktree and `/tmp`. It executes via the CLI wrapper `/usr/bin/sandbox-exec -f profile binary args` (`src/sandbox.rs` lines 417–425). In `src/spawn_config.rs` lines 122–128, the profile is written alongside `gate.py`. Row 4 of `docs/harness-spec.md` table 15.2 designates `src/spawn_config.rs` as the protected "hook-config writer" (`PROTECTED_CODE_PATHS` in `src/rungs/evolve.rs` line 594).

2. **What was measured or verified:**
   Landlock (Linux 5.13+) is strictly an in-process, whitelist-based LSM API (`landlock_create_ruleset`, `landlock_restrict_self`): once enforced, handled actions default to deny, requiring an explicit whitelist of every readable and writable path across the entire filesystem root. It lacks negative exclusion rules and cannot restrict abstract Unix domain sockets (`seats.sock`) or PTY ioctls (`TIOCSTI`). In contrast, Bubblewrap (`bwrap`) is an unprivileged user-namespace CLI wrapper executing external binaries identically to `sandbox-exec`. It directly maps Hydra's default-allow and negative deny model using mount flags: `--ro-bind / /`, `--bind <worktree> <worktree>`, `--tmpfs ~/hydra/runs`, and masked config overlays.

3. **Recommendation:**
   Bubblewrap matches the default-allow, negative-deny composition of `sandbox.rs` and CLI-wrapper model far closer than Landlock. `src/spawn_config.rs` (row 4) will write Linux `bwrap.args` alongside macOS `sandbox.sb` and `gate.py`, and its admission protection status in table 15.2 remains protected (`yes`) unchanged.

4. **What would change your mind:**
   If target Linux cloud VM environments routinely disable unprivileged user namespaces (`kernel.unprivileged_userns_clone = 0`), which would prevent non-root `bwrap` execution and force the adoption of an in-process LSM like Landlock.

## Question 3

1. **What the code does today:**
   `src/provider/mod.rs` lines 29–34 defines a single global table `pub const MEASURED_VERSIONS: [(Provider, &str); 4]` pinning version strings across all providers. `src/doctor.rs` lines 46–55 checks installed binaries against this static array, emitting `DoctorCheck::ProviderDrift` on any mismatch. `src/run/mod.rs` lines 612–618 records whatever version string the provider CLI emits into `run_start.provider_version`.

2. **What was measured or verified:**
   Executing `cargo test --test doctor` confirmed that version checking compares strictly against `MEASURED_VERSIONS`. Across macOS (Darwin arm64/x86_64) and Linux (Linux arm64), vendor CLIs (`claude`, `cursor-agent`, `agy`, `codex`) are distributed as separate binary packages with independent update cadences and distinct platform version metadata. Pinned versions on macOS cause false `ProviderDrift` alerts on Linux and vice versa.

3. **Recommendation:**
   The provider drift table must have a per-OS row (or an OS-keyed mapping `(OS, Provider) -> Version`) to accommodate platform-specific release cadences and avoid false drift alarms.

4. **What would change your mind:**
   If all four vendor CLIs unified their cross-platform release pipelines and semantic version tags such that macOS and Linux binaries were released simultaneously with identical version strings at all times.

## Question 4

1. **What the code does today:**
   `docs/run-record-contract.md` line 196 specifies `base_tree` as the 40-character Git tree SHA representing worktree state at launch (`git write-tree`), verified in `tests/record_contract.rs` lines 163–164. It contains no host, OS, or machine metadata. `hydra doctor --walk` (`src/doctor.rs` lines 63–66) calls `src/walk.rs` lines 356–382 to execute `replay_run()` on historical records using `src/policy/replay.rs` and `gate.py`'s `inside()` scope check (`gate.py` lines 157–167). `src/rungs/corpus.rs` lines 108–115 filters candidates and checks for gate anomalies, while `src/live.rs` line 194 detects abandoned runs by inspecting local kernel PIDs.

2. **What was measured or verified:**
   Python's `os.path.realpath` resolves `/home/ubuntu/...` on macOS by prepending `/System/Volumes/Data/home/ubuntu/...`. When replaying a Linux record on macOS, `gate.py`'s `inside(p, scope)` check fails because the realpath does not start with the recorded Linux worktree path prefix, generating spurious `PolicyDecision::Deny` verdicts and false `Finding::GateLeak` errors (`src/normalize/claude.rs` line 702). Furthermore, syncing active runs across hosts causes `src/live.rs` to misidentify remote PIDs as `Abandoned`.

3. **Recommendation:**
   `base_tree` is completely insufficient because it only tracks Git tree state, whereas multi-machine corpora require host/platform tags in `run_start` and relative-path normalization in event logs to prevent cross-OS path canonicalization failures during replay and Rungs analysis.

4. **What would change your mind:**
   If all run record paths were strictly normalized to worktree-relative paths before storage and the replay harness stubbed host filesystem roots identically across all host environments.

## Recommendation summary
Question 1: Keep recorded runs on macOS until Linux has an OS-level sandbox wrapper to preserve run-record integrity.
Question 2: Adopt Bubblewrap for Linux launches, and keep spawn_config.rs (row 4) protected while adding bwrap argument rendering.
Question 3: Expand MEASURED_VERSIONS with a per-OS dimension to prevent false provider drift warnings across platforms.
Question 4: Add host environment metadata and worktree-relative path normalization to records, as base_tree alone cannot support cross-machine replay.
