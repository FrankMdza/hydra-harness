# 139 - A post-quantum security system of our own, and what it protects

Type: research
Status: open
Blocked by:
Opened: 2026-11-04, by the session, from the human's answer to map 137 question 3. Build 382, the
signed round manifest, waits on this answer.

## The human's words

Translated from Spanish: "could we develop our own post-quantum security system for ourselves,
so that we are very well protected? Launch research and a panel to find our best route, and also
to protect our things. I think we do not suffer from key theft, because we do everything with
OAuth."

## The session's correction, measured 2026-11-04

OAuth does not remove keys. It replaces a password with tokens, and a refresh token kept on disk
is a bearer key: whoever reads the file acts as the account until it is revoked. On this Mac, by
`ls -l` only, contents not read:

- `~/.codex/auth.json`, mode 600.
- `~/.config/gcloud/credentials.db` and `application_default_credentials.json`, mode 600, the
  GCP project's credentials.
- `~/.config/gh/hosts.yml`, mode 600, the GitHub token.
- `~/hydra/.env`, mode 600, the Jev key (also in the Keychain).
- `claude`'s credentials in the login Keychain, item `Claude Code-credentials`.
- `~/.cursor/cli-config.json`, mode 644, contents not checked.
- `~/.ssh/id_ed25519`, a second ed25519 key and `google_compute_engine`, private keys.

Mode 600 stops other users, not other processes of the same user. The repository's CLAUDE.md
already says that under `standard` a gated run's shell can reach `~/.ssh` and the rest of the home
directory. So the lab's first exposure is a stolen token or key file, and that is a classical
problem. Post-quantum cryptography protects against a different one: data recorded today and
decrypted later, and signatures forged later.

The local OpenSSH is 10.0p2, whose default key exchange is `mlkem768x25519-sha256`. Whether GCP's
VMs and GitHub negotiate it is not checked.

## Research questions

1. **The threat model, per asset.** Run records, exports, training data on a VM, weights coming
   back, the skill store, the tokens and keys above. For each: classical threat, post-quantum
   threat, and which one matters on this machine.
2. **What "our own system" should mean.** Composing vetted primitives (ML-KEM, ML-DSA, X25519,
   HKDF, ChaCha20-Poly1305, Argon2id) into a Hydra-specific format and tool, against writing any
   primitive. Cite what NIST, the IETF and the RustCrypto maintainers say about it.
3. **What exists to reuse.** `age` and its post-quantum recipients, if they exist; X-Wing and the
   IETF hybrid KEM drafts; ML-DSA signing tools; OpenSSH's post-quantum key exchange; TLS
   X25519MLKEM768 in `rustls` and in Google's front ends and `gcloud`. Versions and dates.
4. **The Rust crates.** `ml-kem`, `ml-dsa`, `fips204`, `pqcrypto-*`, `oqs`, `aws-lc-rs`, `libcrux`:
   version, audit or formal verification, known-answer test coverage, constant-time claims, and
   maintenance. Which one the lab should build on.
5. **Protecting the tokens.** What reduces the risk of the files above: Keychain or Secure Enclave
   storage, shorter-lived tokens, a deny on those paths in Hydra's sandbox profile for every
   policy, and what each CLI allows. Say what Hydra's own `src/sandbox.rs` denies today.
6. **The route.** A first version the lab can build and test in two weeks, and what comes after.

Defensive only: no attack technique, no cryptanalysis.

## Output

`research/139-post-quantum-route.md`, at most 2,500 words, every claim with its source and the
date read. Then a panel under `research/139-panel-prompt.md`.

## Panel

Sat 2026-11-04, three seats (Grok not seated, Cursor login). Synthesis in
`research/139-synthesis.md`. Waiting on the human.

## The SSH line, done by the human 2026-11-04

`~/.ssh/config` did not exist. The human created it with `Host *` and `KexAlgorithms
^mlkem768x25519-sha256,sntrup761x25519-sha512`, mode 600 (map 142, question 3). Negotiated with
GitHub, from `ssh -v git@github.com 2>&1 | grep 'kex: algorithm:'`: before `ecdh-sha2-nistp256`,
after `mlkem768x25519-sha256`. A GCP VM is not measured yet; check it with `gcloud compute ssh VM
--ssh-flag=-v` at round two.
