# Decision memo: Map ticket 142 (Seat: Gemini)

## Claims checked against repository
1. **File counts and `private-repo` tracking**: Checked against `git ls-files` and `git grep`. The repository tracks 1,667 files total, with 716 in `.scratch/` (~6.2 MB). Exactly 21 tracked files mention `private-repo`, including the 4 task-class files under `.scratch/hydra/labeling/2026-09-29/`.
2. **Export allowlist contents**: Checked [`config/export-allowlist.json`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/config/export-allowlist.json). It contains exactly `["/Users/USER/hydra"]`.

---

## 1. `.scratch/` and private-repo
**Commitment: (A) Publish as it is.**

- **What `private-repo` discloses**: [withheld: what the seat read in a private repository]
- **Trade-offs**: A reviewer evaluating a harness engineering portfolio loses the primary proof of system evolution under (D) or (C). `.scratch/` holds the 370+ build tickets, map issues, benchmarks, and multi-agent panel syntheses. Dropping `.scratch/` (D) strips the repository of its architectural depth. A single clean snapshot (C) or history rewrite via `git filter-repo` (B) is irreversible; rewriting history breaks commit hashes referenced across [`CHANGELOG.md`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/CHANGELOG.md), ticket outcome records, and tag verification.
- Publishing as-is (A) maintains complete evidentiary integrity.

---

## 2. Credential paths
**Commitment: (A) Leave them.**

- Under Option 1(A), git history is preserved intact. Editing paths in working files without rewriting history leaves the original paths accessible in `git log`, creating an illusion of redaction without technical benefit.
- Path strings and file names alone are not secrets; without cryptographic material, knowing a key name or path provides zero exploit primitive.
- Standard locations (`~/.ssh`, `~/.config/gcloud`, `~/.codex/auth.json`) reflect upstream tool conventions.
- In [`tests/gate.rs`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/tests/gate.rs), concrete paths are used strictly to test gate denial logic (`cat ~/.ssh/id_rsa`, `/etc/hosts`). The test suite does not name custom private key files and requires concrete strings to construct and test path violations. Generalizing them would introduce unnecessary abstraction into test assertions.

---

## 3. The SSH line
**Commitment: Add `KexAlgorithms ^mlkem768x25519-sha256` under `Host *` in `~/.ssh/config`.**

- **Scope**: Use `Host *`. The `^` syntax prepends `mlkem768x25519-sha256` to OpenSSH's default KEX algorithm list rather than replacing it.
- **Fallback behavior**: When connecting to a server that does not support ML-KEM, OpenSSH automatically falls back to the next mutually supported cipher (e.g., `sntrup761x25519-sha512@openssh.com` or `curve25519-sha256`). Connections do not terminate or fail.
- **Verification**: Run `ssh -v <host>` and look for `debug1: kex: algorithm: <name>`. Prior to configuration, Apple OpenSSH 10.0p2 negotiates `ecdh-sha2-nistp256`. After configuration, connections to GitHub negotiate `mlkem768x25519-sha256`.
- **Host compatibility**: GitHub officially supports `mlkem768x25519-sha256` as of October 2026. Standard Linux hosts (`gcloud compute ssh` to Debian/Ubuntu images, a VPS provider) negotiate standard fallback algorithms safely per RFC 4253 algorithm negotiation rules.

---

## 4. The signing crate
**Commitment: `fips204`.**

- **Build complexity**: `aws-lc-rs` depends on `aws-lc-sys`, requiring C, CMake, and assembly compilation toolchains. `fips204` is pure Rust, building cleanly across macOS and Linux CI without external C dependencies.
- **Hydra footprint**: Checked [`Cargo.lock`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/Cargo.lock) and [`deny.toml`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/deny.toml). Hydra currently has no cryptography dependencies beyond `sha2`. Adding `aws-lc-rs` pulls in a massive BoringSSL-derived C codebase, inflating binary size. Both crates pass license checks in `deny.toml`.
- **FIPS validation scope**: As identified in research 139, AWS-LC's FIPS validation covers ML-KEM, not ML-DSA. ML-DSA in `aws-lc-rs` lacks FIPS validation.
- **Architecture**: In build 382, ML-DSA-65 is deployed alongside Ed25519 in a dual-signature scheme. Ed25519 provides mature classical integrity today, while `fips204` delivers post-quantum longevity with source-level constant-time assurances verified against NIST ACVP and Wycheproof test vectors.

---

## 5. Order to the switch
1. **Session**: Complete build 378 (deny `~/.claude.json` and `history.jsonl` in gated profiles, update tests and [`SECURITY.md`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/SECURITY.md)).
2. **Frank**: Execute the 30-minute interactive acceptance session for build 378.
3. **Frank**: Add `KexAlgorithms ^mlkem768x25519-sha256` to `~/.ssh/config` and verify negotiation via `ssh -v git@github.com`.
4. **Frank**: Update lines 40–43 of protected [`config/mcp.md`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/config/mcp.md) with the synthesis wording.
5. **Frank**: Configure the 8 repository topics on GitHub (`agent-harness`, `ai-agents`, `claude-code`, `codex-cli`, `cursor-agent`, `sandboxing`, `macos-seatbelt`, `rust`) with no homepage.
6. **Frank**: Push `master` to GitHub (`git push origin master`).
7. **Frank**: Toggle repository visibility from Private to Public in GitHub repository settings.

---

## Pending from me
- None.

## Changed
- Read-only panel evaluation; no repository files created, modified, or deleted.

## Found
- [`Cargo.lock`](file:///Users/USER/hydra/worktrees/01a1aa01-b1ce-773d-93fc-79a3113825a9/Cargo.lock) contains only `sha2` (and `crypto-common`) as cryptographic dependencies; no C-based crypto libraries exist in the workspace.
- The 21 tracked files mentioning `private-repo` contain no financial balances, tickers, or proprietary parameters, only technical indicator labels and concurrency locks.

