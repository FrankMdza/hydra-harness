I've read the prompt, the research, ticket 382 and the sandbox code, and I'm writing the seat's answer now.

# Seat Opus, map 139

## What I checked against the code

1. **Run records are not readable from a gated shell.** The research's section 1 table says they are, under `standard`, "by shell". That is wrong. `rules_of` pushes `ReadDenySubpath(spawn.corpus)` for every policy (`src/sandbox.rs:465`), and `spawn.corpus` is `layout.runs()` (`src/spawn_config.rs:162`). Section 5 of the research has it right. Only the table row is wrong.
2. **Section 5's account of the sandbox is accurate.** It starts from `(allow default)` (`src/sandbox.rs:590`). Bounded shells get `file-read-data` denied on Hydra's root (`:941`). `copilot_block` denies writes under home and leaves reads open (`:854`). The write list is at `src/policy/mod.rs:794`. No rule names a credential path.
3. **The research misses a bubblewrap problem.** It renders `ReadDenySubpath` as `--tmpfs <path>` (`src/sandbox.rs:794`). A tmpfs mounts over a directory, not a file, so a deny on `~/hydra/.env` or `~/.codex/auth.json` needs a separate file rule on Linux, such as `--ro-bind /dev/null <file>`. The GCP VM runs Linux.
4. **`aws-lc-rs` would be a new dependency.** `Cargo.lock` holds 178 packages and none of them is `aws-lc-rs`, `ring` or `rustls`, so choosing it brings in a C toolchain.
5. **`age` is not installed.** Neither `age` nor `rage` is on `PATH`. `ls ~/.ssh` also shows a third key directory that the ticket's list leaves out.

## 1. Order of version one

Reorder, and cut `hydra seal` from version one.

- **The `KexAlgorithms` line comes first.** It costs one line and Frank writes it.
- **The read deny comes second, and it protects the most.** Version one has to deal with stolen bearer tokens and stolen SSH keys.
- **The signed manifest comes third.** Build 382 waits on it.
- **Seal is cut.** Traffic to Google already negotiates `X25519MLKEM768`, which the research measured. GCP encrypts disks with symmetric keys, and those are not the post-quantum exposure. So a seal guards against a leaked account or a misconfigured bucket, and that threat is classical and handled by the token work. It goes back on the list when data is stored somewhere other than a GCP disk.

## 2. The credential conflict

**Choose (A), narrowed, and keep (B) as a measurement, not a dependency.**

Under (A), every profile denies `~/.ssh`, `~/.config/gcloud`, `~/.config/gh`, `~/hydra/.env` and `~/.claude/.credentials.json`, plus every provider's token file except the launched provider's own. A gated shell can still read its own provider's token, and it can still get Claude's Keychain item through `/usr/bin/security`.

(B) does not fix the second leak. Claude Code reads its credential by running `security` (`src/env.rs:160` requires `/usr/bin` for that read), so the profile cannot deny that exec without breaking `claude`. The gh maintainer says the same about `gh` [30]. Items `codex` writes through the keyring crate might carry an app-specific access list. Nobody has measured that, and it is the one place (B) could add something.

(C) waits on (B), and (B) is unproven. That makes (D), "move the provider out of the process tree", a redesign of the launch, so it belongs on the map and not in version one.

The test plants dummy files at every path under a scratch `HOME`, so no real credential is touched. A gated shell's `cat` of each one gets EPERM, and the provider's own file reads. Then revert: drop the rule and watch each read succeed. A second assertion records `security find-generic-password -s 'Claude Code-credentials'` (exit code only, no `-w`) succeeding from the gated shell. That makes the known gap a test result and not a sentence.

One cost: under the deny, any `gcloud` or `gh` work moves to the ungated session. Build 380's GCP checks already ran there.

## 3. Encryption at rest

**Choose (A) when a seal is needed, not now.** Composing ML-KEM over `aws-lc-rs` ourselves (B) puts the bugs where the lab cannot see them, in the KEM combiner, the KDF and the nonce handling. The human's wish for Rust is better spent on the manifest, where the Rust is a format around a vetted signer. Waiting for `rage` (C) costs nothing, since seal is cut.

## 4. Signing

- **Crates.** `aws-lc-rs` signs with `signature::ML_DSA_65`, and `libcrux-ml-dsa` verifies, as a dev-dependency used in one test only. Both run against NIST ACVP vectors, which is what 382 asks for. `fips204` is a reasonable pure-Rust alternative if the C build is unwelcome, but it has one vendor and no audit.
- **Ed25519 beside it.** Yes, with `ed25519-dalek`, and verification requires both signatures. That protects against a bug in a young ML-DSA implementation, which is likelier than a quantum forger.
- **The key on the VM.** Make a fresh key pair per round on the VM, under a separate Unix user (`signer`) from the one that trains (`trainer`). It signs only after the training process has exited. The Mac fetches the public key over `gcloud compute ssh` before uploading anything, and pins it in the round's input manifest.
- **The key on the Mac.** The Mac's key signs that input manifest, so the VM can check what it trains on. It lives in a file under a path the read deny covers.
- **What the signature cannot prove.** It proves the bytes did not change between the signer's hash and the Mac's load. It says nothing about whether the training code was honest. If `trainer` escalates to `signer`, or the key were reachable from training, as GPT's limit on map 137 says, the signature covers whatever that code wrote.

## 5. The tickets

1. **The credential read deny.** Add a `Rule` variant read by both composers, with a file-shaped bwrap rendering. The test is the scratch-`HOME` EPERM test from question 2, plus the revert step and the recorded Keychain gap.
2. **Measure what is still reachable, with no code change.** Check whether `gh auth status` reports Keychain storage, which keys (not values) are in `~/.cursor/cli-config.json`, whether a `codex` keyring item prompts when a gated shell reads it, and the VM image's `sshd` key exchange. Write the results to `IMPLEMENTATION.md`.
3. **Build 382, amended.** The manifest gets canonical JSON, ML-DSA-65 plus Ed25519, the `signer`/`trainer` split, the libcrux cross-verify, ACVP vectors, and the three refusals: a flipped byte, a wrong key and a missing signature. Revert: skip the verify call and watch the flipped-byte test load.
4. **Seal goes back to the map as fog**, to be reopened when data is stored outside GCP disks.

## Pending from me

- Nothing. The panel's recommendation goes to the human.

## Changed

- No file changed and nothing committed. I only read files and ran `ls`, `grep` and `which`.

## Found

- The research's asset table says run records are shell-readable under `standard`. They are not (`src/sandbox.rs:465`).
- bwrap's `--tmpfs` rendering cannot deny a single file, so the credential deny needs a new rendering on Linux.
- `aws-lc-rs` is not in `Cargo.lock`, so picking it adds a C build to Hydra.
- a third key directory under `~/.ssh` exists and is not in the ticket's list. `~/.cursor/cli-config.json` is mode 644.
- `claude` reads its Keychain item through `/usr/bin/security`, so a sandbox cannot keep that token from the claude run's own shell.
