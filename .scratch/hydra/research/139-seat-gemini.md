Seat: Gemini.

## 1. Order of version one

**Recommendation:** Reorder and keep all four: (1) SSH `KexAlgorithms` line, (2) Credential read deny, (3) Signed round manifest (build 382), (4) `hydra seal`.

The **credential read deny protects the most for the least**. On-disk bearer tokens and SSH keys in `~/.ssh`, `~/.config/gcloud`, `~/.config/gh`, and `~/hydra/.env` represent the lab’s primary active risk today: any standard run can read them by shell. Closing this requires minimal LOC in [`src/sandbox.rs`](file:///Users/USER/hydra/worktrees/01a1a86d-70c7-7623-b8ff-d9b27ec32986/src/sandbox.rs). The SSH config change is placed first only because it is a zero-risk, one-line user edit by Frank that enables immediate post-quantum transport security. The signed manifest (build 382) comes next to unblock pipeline artifact loading. `hydra seal` is fourth because Google API traffic already negotiates post-quantum TLS (`X25519MLKEM768`), making cloud data at rest a lower immediate urgency.

## 2. The credential conflict

**Recommendation: (C) Both.**

**Why:** (A) alone leaves the active provider’s shell capable of reading its own on-disk token (e.g. a `codex` shell can `cat ~/.codex/auth.json`). (B) alone causes runtime failures if an unmigrated or third-party CLI still requires local files (such as `gcloud` ADC). Combining both isolates credentials completely: migrate `codex` and `gh` to system Keychain storage, delete on-disk plain-text token files, and deny all credential file paths in the Seatbelt profile. If an unmigrated provider still requires a file, deny all paths except that specific provider’s file.

- **What each leaves readable:** (A) leaves the launched provider’s own token file readable to its subprocess tree. (B) leaves zero on-disk credential files readable (`EPERM`), but leaves Keychain items exposed if the shell invokes `/usr/bin/security`. (C) eliminates plain-text on-disk credential reads entirely for migrated tools (`EPERM`) and restricts residual exposure to Keychain IPC.
- **What a test would show:** Executing `cat` on `~/.ssh/id_ed25519`, `~/.config/gcloud/credentials.db`, `~/.config/gh/hosts.yml`, and `~/hydra/.env` fails with `EPERM`. For migrated Codex, `cat ~/.codex/auth.json` also fails with `EPERM`. Reverting the SBPL rules in [`src/sandbox.rs`](file:///Users/USER/hydra/worktrees/01a1a86d-70c7-7623-b8ff-d9b27ec32986/src/sandbox.rs) allows reads to succeed, proving the sandbox enforces the block.

## 3. Encryption at rest

**Recommendation: (A) Shell out to the Go `age` binary with `age1pq1` recipients.**

**Why:** The human’s preference for Rust must yield to cryptographic safety: writing a bespoke Rust composition over `aws-lc-rs` (B) constitutes rolling a custom cryptographic envelope (framing, streaming AEAD, recipient wrapping), violating the core principle of composing vetted formats. `age` 1.3+ offers a battle-tested, standardized hybrid ML-KEM-768/X25519 format that strictly prevents mixing with classical-only recipients. Shelling out takes under 100 LOC in Rust. Once `rage` (Rust) ships `age1pq` support, Hydra can switch to native crate bindings with zero format changes.

## 4. Signing

- **Crate:** **`aws-lc-rs`** signs. It provides stable, constant-time, FIPS-validated ML-DSA-65 primitives maintained by AWS. `libcrux` remains pre-1.0, and RustCrypto `ml-dsa` is unaudited.
- **Cross-verification:** **Yes**. A test must verify signatures using **`libcrux-ml-dsa`** to catch serialization bugs or non-standard encoding quirks in either crate.
- **Dual signing:** **Yes**. The manifest must carry both ML-DSA-65 and Ed25519 signatures. Ed25519 provides proven classical security today, while ML-DSA-65 guarantees post-quantum integrity.
- **Key custody:** On the VM, the private signing key resides in an isolated service directory (e.g. `/etc/hydra/signing.key`, mode 0400, owned by an isolated daemon user, unreadable by the training job). On the Mac, verification public keys are pinned in `config/roots/`, and the Mac’s private keys live in the macOS Keychain/Secure Enclave.
- **Signature limitation:** The signature guarantees authenticity and transit integrity, but **cannot prove artifact correctness or prevent weight poisoning if compromised training code can access the signing key**.

## 5. The tickets

1. **Ticket 1: Credential read denial in sandbox ([`src/sandbox.rs`](file:///Users/USER/hydra/worktrees/01a1a86d-70c7-7623-b8ff-d9b27ec32986/src/sandbox.rs))**
   - *Measures:* `EPERM` when reading credential paths (`~/.ssh`, `~/.config/gcloud`, `~/.config/gh`, `~/hydra/.env`, `~/.claude/.credentials.json`, `~/.cursor`).
   - *Test:* Gated shell test confirms `cat` exits non-zero with `EPERM`. Verify-by-reverting: remove SBPL deny lines, confirm `cat` succeeds.
2. **Ticket 2: Provider Keychain migration (`codex`, `gh`)**
   - *Measures:* Codex configured with `cli_auth_credentials_store = "keyring"`, `gh` on system Keychain, removal of on-disk token files.
   - *Test:* Asserts `~/.codex/auth.json` does not exist; verifies CLI auth passes via OS Keychain.
3. **Ticket 3: Signed round manifest (Build 382)**
   - *Measures:* Generation and verification of hybrid ML-DSA-65 (`aws-lc-rs`) + Ed25519 signatures over canonical JSON SHA-256 manifests.
   - *Test:* Unit tests reject corrupted weight bytes, altered manifests, or untrusted keys. Cross-verifies `aws-lc-rs` signatures against `libcrux-ml-dsa`. Confirms round 1 adapter loads.
4. **Ticket 4: `hydra seal` CLI wrapper over `age`**
   - *Measures:* Encryption and decryption round-trip of export archives to `age1pq1` recipients.
   - *Test:* Asserts output starts with age PQ header, decrypts with private key to bitwise identical plaintext, and rejects classical-only recipients.

## Verified research claims

1. **Hydra sandbox read rules ([`src/sandbox.rs:465-475, 590, 941`](file:///Users/USER/hydra/worktrees/01a1a86d-70c7-7623-b8ff-d9b27ec32986/src/sandbox.rs#L465-L475) and [`src/policy/mod.rs:794`](file:///Users/USER/hydra/worktrees/01a1a86d-70c7-7623-b8ff-d9b27ec32986/src/policy/mod.rs#L794)):** Confirmed profile starts with `(allow default)` and lacks read denies for `~/.ssh`, `~/.config/gcloud`, `~/.config/gh`, or `~/hydra/.env` under `standard`.
2. **OpenSSH default key exchange:** Confirmed via `ssh -V` (OpenSSH_10.0p2) and `ssh -G github.com` that Apple OpenSSH places `ecdh-sha2-nistp256` ahead of `mlkem768x25519-sha256`.
3. **Dependencies:** Confirmed neither `aws-lc-rs` nor `rustls` is present in `Cargo.lock`.

## Pending from me

- None.

## Changed

- None. Read-only seat; no files created, modified, or committed.

## Found

- `age`, `age-keygen`, and `rage` are not currently installed on the host machine (`which` returns not found).
- Hydra currently has no cryptographic dependencies beyond `sha2 = "0.11"`.

