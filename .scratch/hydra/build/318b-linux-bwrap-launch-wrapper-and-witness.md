# Build 318b - Linux bubblewrap launch wrapper, Codex nesting decision, and mount-namespace witness

Status: claimed
Size: haiku

## Goal

Wire the Linux bubblewrap backend into `src/sandbox.rs` and its callers using the `compose_bwrap` composer from build 318a, settle the three Linux decisions from build 318 (`wraps_launches()` gating and refusal under `standard` and `permissive`, Codex `--sandbox` nesting, and `inside_seatbelt()` mount-namespace witness), guard macOS-only `sandbox-exec` and SBPL tests by `sandbox::kind() == "seatbelt"`, and update `docs/harness-spec.md`, `IMPLEMENTATION.md`, and `.scratch/hydra/build/318-a-bubblewrap-backend-for-the-launch-wrapper.md`.

## Exact edits

1. **In `src/sandbox.rs`:**
   - Add constants:
     - `pub const KIND_SEATBELT: &str = "seatbelt";`
     - `pub const KIND_BUBBLEWRAP: &str = "bubblewrap";`
     - `pub const LAUNCHER_MNT_FILE: &str = "launcher.mnt";`
   - Add `pub fn kind() -> &'static str`:
     - Returns `KIND_BUBBLEWRAP` on `cfg!(target_os = "linux")` and `KIND_SEATBELT` otherwise.
   - Add pure, testable helpers for the three Linux decisions:
     - **Decision 1 (`wraps_launches()` and policy refusal on Linux):**
       - `pub fn bwrap_in_path(path_var: Option<&std::ffi::OsStr>) -> Option<PathBuf>`: checks each directory in `std::env::split_paths` of `path_var` for an executable file named `bwrap` (`mode & 0o111 != 0`).
       - `pub fn linux_userns_enabled_in(proc_sys: &Path) -> bool`: returns `false` if `proc_sys.join("kernel/unprivileged_userns_clone")` reads `"0"` (trimmed), or `proc_sys.join("user/max_user_namespaces")` reads `"0"` (trimmed), or `proc_sys.join("kernel/apparmor_restrict_unprivileged_userns")` reads `"1"` (trimmed); otherwise returns `proc_sys.join("kernel").is_dir()`.
       - `pub fn linux_bwrap_ready() -> bool`: `bwrap_in_path(std::env::var_os("PATH").as_deref()).is_some() && linux_userns_enabled_in(Path::new("/proc/sys"))`.
       - Update `wraps_launches()` and `exec_available()`:
         - On `cfg!(target_os = "macos")`: `wraps_launches()` is `true`, `exec_available()` is `Path::new(SANDBOX_EXEC).is_file()`.
         - On `cfg!(target_os = "linux")`: both return `linux_bwrap_ready()`.
         - Elsewhere: both return `false`.
       - `pub fn launch_refusal_for_policy_on(os: &str, policy: &str, sandbox_exec_present: bool, bwrap_ready: bool) -> Option<String>`:
         - When `os == "macos"` and `!sandbox_exec_present`: returns `Some("/usr/bin/sandbox-exec is missing on this Mac, so Hydra refuses to launch a recorded run without the seatbelt rather than run bare".to_string())`.
         - When `os == "linux"` and `!bwrap_ready` and `matches!(policy, "standard" | "permissive")`: returns `Some(format!("bwrap on PATH with unprivileged user namespaces enabled is unavailable on this Linux host, so Hydra refuses to launch under {policy} rather than fall back to hook-only"))`.
         - Otherwise returns `None`.
       - `pub fn launch_refusal_for_policy(policy: &str) -> Option<String>`: calls `launch_refusal_for_policy_on(std::env::consts::OS, policy, Path::new(SANDBOX_EXEC).is_file(), linux_bwrap_ready())`.
     - **Decision 2 (Codex sandbox nesting):**
       - `pub fn forbids_vendor_sandbox_for(kind: Option<&str>) -> bool`: returns `matches!(kind, Some(KIND_SEATBELT | KIND_BUBBLEWRAP))` because neither macOS `sandbox-exec` nor Linux `bwrap` (whose mount namespace marks parent mounts `MNT_LOCKED` and whose worktree gitdir and object store sit outside the worktree) allows Codex's own `workspace-write` user-namespace sandbox to nest inside Hydra's wrapper.
       - `pub fn forbids_vendor_sandbox() -> bool`: `wraps_launches() && forbids_vendor_sandbox_for(Some(kind()))`.
     - **Decision 3 (`inside_seatbelt()` mount-namespace witness on Linux):**
       - `pub fn linux_inside_bwrap_from(current_mnt_ns: &str, mountinfo: &str) -> bool`:
         - Returns `false` if `current_mnt_ns.trim().is_empty()`.
         - Parses each line of `mountinfo`, taking the mount point (field index 4, the fifth whitespace-separated token, unescaping `\040` to space). If `<mount_point>/launcher.mnt` is a file whose trimmed contents `launcher_ns` is non-empty and `launcher_ns != current_mnt_ns.trim()`, returns `true`.
         - Otherwise returns `false`.
       - Update `inside_seatbelt()`:
         - On `cfg!(target_os = "macos")`: unchanged `sandbox_check(libc::getpid(), std::ptr::null(), 0) == 1`.
         - On `cfg!(target_os = "linux")`: reads `std::fs::read_link("/proc/self/ns/mnt")` and `std::fs::read_to_string("/proc/self/mountinfo")` and calls `linux_inside_bwrap_from`.
         - Elsewhere: `false`.
   - Update profile writing and command wrapping for both backends:
     - `profile_path(spawn_dir: &Path) -> PathBuf`: returns `spawn_dir.join("sandbox.bwrap")` on `cfg!(target_os = "linux")` and `spawn_dir.join("sandbox.sb")` otherwise.
     - `pub fn write_bwrap_profile(set: &PolicySet, spawn: &Spawn<'_>, path: &Path, launcher_mnt_ns: Option<&str>) -> Result<String>`:
       - Composes `let args = compose_bwrap(set, spawn); let text = format!("{}\n", args.join("\n"));`.
       - Creates parent dir if needed, writes `text` to `path`, and if `let Some(ns) = launcher_mnt_ns` writes `format!("{}\n", ns.trim())` to `path.with_file_name(LAUNCHER_MNT_FILE)`.
       - Returns `Ok(hex_lower(&Sha256::digest(text.as_bytes())))`.
     - In `write_profile(set: &PolicySet, spawn: &Spawn<'_>, path: &Path) -> Result<String>`:
       - If `path.extension().and_then(|e| e.to_str()) == Some("bwrap")` (or on `cfg!(target_os = "linux")`), reads `std::fs::read_link("/proc/self/ns/mnt").ok().map(|p| p.to_string_lossy().into_owned())` and calls `write_bwrap_profile(set, spawn, path, launcher_mnt.as_deref())`.
       - Otherwise keeps the SBPL write path.
     - In `narrow_for_copilot(profile: &Path, worktree: &Path) -> Result<String>`:
       - If `profile.extension().and_then(|e| e.to_str()) == Some("bwrap")`, bail with `anyhow::bail!("copilot regex rule (^{home}/.claude.json) has no bubblewrap mount equivalent; gated interactive launch is not supported under bubblewrap")` (after resolving `home = crate::env::real_home()?`).
     - `pub fn wrap_bwrap_command(binary: &str, argv: &[String], profile: &Path) -> (&'static str, Vec<String>)`:
       - Reads `profile` lines (filtering empty lines), appends `"--".to_string()`, `binary.to_string()`, and `argv.iter().cloned()`, and returns `("bwrap", wrapped)`.
     - In `wrap_command(binary: &str, argv: &[String], profile: &Path) -> (&'static str, Vec<String>)`:
       - If `profile.extension().and_then(|e| e.to_str()) == Some("bwrap")`, calls `wrap_bwrap_command(binary, argv, profile)`.
       - Otherwise keeps `SANDBOX_EXEC` (`"-f"`, `profile`, `binary`, `argv...`).
   - Add unit tests in `src/sandbox.rs` (`mod tests`):
     - `linux_userns_check_reads_unprivileged_userns_max_userns_and_apparmor`
     - `launch_refusal_for_policy_refuses_standard_and_permissive_on_linux_without_bwrap_and_allows_judge`
     - `codex_vendor_sandbox_is_forbidden_under_both_seatbelt_and_bubblewrap`
     - `linux_inside_bwrap_witness_compares_launcher_mnt_against_current_mnt`
     - `write_bwrap_profile_and_wrap_bwrap_command_round_trip`
     - `narrow_for_copilot_refuses_a_bwrap_profile_naming_the_regex_rule`

2. **In callers of `sandbox`:**
   - `src/provider/codex.rs` (lines 82, 114, 647): replace `crate::sandbox::wraps_launches()` with `crate::sandbox::forbids_vendor_sandbox()`.
   - `src/run/mod.rs` (line 864): replace `if crate::sandbox::wraps_launches() && !crate::sandbox::exec_available() { ... }` with `if let Some(msg) = crate::sandbox::launch_refusal_for_policy(&machine.request.policy) { return Err(fail_after_handoff(machine, worktree, &handoff, anyhow::anyhow!(msg), true)); }`.
   - `src/interactive.rs` (line 357): replace `if crate::sandbox::wraps_launches() && !crate::sandbox::exec_available() { ... }` with `if let Some(msg) = crate::sandbox::launch_refusal_for_policy(policy) { anyhow::bail!(msg); }`.
   - `src/run/record.rs` (line 307) and `src/interactive.rs` (line 438): replace `.map(|_| "seatbelt")` with `.map(|_| crate::sandbox::kind())`.
   - `src/rungs/evolve.rs` (lines 959, 1223, 1252): guard the SBPL `admission_profile` and `/usr/bin/sandbox-exec` blocks with `crate::sandbox::kind() == crate::sandbox::KIND_SEATBELT && crate::sandbox::wraps_launches()`.
   - `tests/corpus/mod.rs` (line 671): in `seatbelt_exec_or_skip`, check `if hydra::sandbox::kind() == "seatbelt" && hydra::sandbox::exec_available() { return true; }`.
   - `tests/spawn.rs` (lines 1087, 1217, 1290, 1484, 1843, 1875, 2036) and `tests/seats.rs` (lines 665, 1415) and `tests/face.rs` (line 362) and `tests/evolve.rs` (lines 1107, 1180, 1402, 1791): add `hydra::sandbox::kind() == "seatbelt"` to those macOS SBPL guards, and in `tests/delegation.rs` (line 535) assert `assert_eq!(wrapped["sandbox_kind"], hydra::sandbox::kind());`.

3. **In `docs/harness-spec.md`, `IMPLEMENTATION.md`, and `.scratch/hydra/build/318-a-bubblewrap-backend-for-the-launch-wrapper.md`:**
   - Amend `docs/harness-spec.md` section 7.5 (around line 1013) and section 15.2 (around line 1549) so that Linux bubblewrap (`bwrap`, `sandbox.bwrap`, `sandbox_kind: "bubblewrap"`) is stated alongside macOS `sandbox-exec`.
   - Amend `IMPLEMENTATION.md` (around lines 26–29 and 2606–2613) to describe build 318 (`compose_bwrap`, the three Linux decisions, `sandbox_kind: "bubblewrap"`, and that runtime testing on a Linux machine remains the human's).
   - Mark `.scratch/hydra/build/318-a-bubblewrap-backend-for-the-launch-wrapper.md` `Status: resolved` and write its `## Outcome` summarizing the shared rule table, the three decisions, and the verification results.
   - Do NOT use em dashes (`—`) or parentheses in prose in any markdown file.

## Verification

Run:
- `cargo check --target x86_64-unknown-linux-gnu`
- `cargo fmt --all --check`
- `cargo clippy --release --workspace --all-targets -- -D warnings`
- `cargo nextest run --release -E 'test(sandbox) | test(codex) | test(delegation)'`
- `git diff --check`
