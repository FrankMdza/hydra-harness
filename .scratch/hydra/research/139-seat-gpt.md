**GPT seat: keep all four items. Choose credential option C, encryption option A, and `aws-lc-rs` for signing.**

**1. Version-one order.** Deliver SSH preference first because it is the smallest change, then credential protection, `hydra seal`, and the signed manifest. Credential protection provides the greatest practical protection for modest effort; SSH provides the cheapest incremental improvement. Encryption must precede sensitive uploads; manifest verification must precede loading returned weights. Neither should block implementing the other.

**2. Credential conflict: C, both, with an explicit residual exposure.**

- **A alone:** deny unrelated credentials; allow only the launched provider’s required credential. Its shell retains exactly that exception. Avoid exempting an entire provider directory when one file suffices.
- **B alone:** migration removes file copies, but a shell may still retrieve Keychain items or invoke authenticated tools, depending on access controls. Unmigrated credentials remain exposed without additional denies.
- **C:** migrate supported credentials, confirm authentication works, remove stale copies, and deny those paths plus unrelated credentials. Permit narrowly documented exceptions only where authentication still requires files. Provider exceptions and accessible Keychain services remain usable by the shell.

Test with synthetic credential files and disposable Keychain items, never production secrets. Exercise each provider and its child shell under each supported policy. Confirm denied reads, successful provider authentication, and whether the child can retrieve the disposable Keychain item. A successful retrieval disproves isolation even if file tests pass. Keychain storage alone does not create a provider/shell boundary.

**3. Encryption: A, a Rust wrapper invoking Go `age` directly.** Keep orchestration, recipient policy, and failure handling in Rust; reuse the existing envelope. A Rust KEM composition introduces protocol, encoding, and key-management obligations that SP 800-227 guidance does not discharge. Waiting for native `rage` support delays protection unnecessarily.

Require parsed `age1pq1` recipients, reject classical mixtures, pin the executable, and publish output atomically only after success. Use separate VM and Mac identities; protect the Mac identity in Keychain and the VM identity outside the training account where practical. Once training receives plaintext, encryption cannot protect it from compromised training code.

I checked the [age documentation](https://pkg.go.dev/filippo.io/age): native PQ support and mixing restrictions match the research. Its “only Go” wording is too broad: the documented PQ plugin also supports compatible age implementations.

**4. Signing: `aws-lc-rs`, with `libcrux-ml-dsa` verification in tests.** The current [AWS-LC Rust API](https://docs.rs/aws-lc-rs/latest/aws_lc_rs/signature/index.html) exposes ML-DSA-65 signing and verification. Choose it subject to NIST vector and interoperability gates; those gates test correctness, not side-channel safety or certification. Do not infer ML-DSA FIPS validation from ML-KEM validation.

Carry **ML-DSA-65 and Ed25519, requiring both signatures**. An either-signature rule loses the intended protection. Sign identical domain-separated, canonical manifest bytes binding schema, algorithm suite, key identifiers, round, base-model digest, and artifact paths, lengths, and hashes. Pin trusted public keys independently of downloaded artifacts. Reject unexpected rounds, unsafe paths, duplicates, missing files, and unsigned loadable files; ensure the bytes loaded are those verified.

On the VM, keep signing keys under a separate signer account, outside training mounts, with restrictive permissions. On the Mac, keep verification keys in a protected trust store; any local signing private keys belong in Keychain. Defer Secure Enclave integration until separately demonstrated.

A reachable signing service can still be a signing oracle. If training code can access the key or authorize arbitrary signatures, the manifest proves possession and artifact integrity—not honest training, provenance, or safe weights.

**5. Tickets, smallest first:**

1. **SSH preference and measurement.** Human applies the host-scoped `KexAlgorithms` preference. Record negotiated algorithms for GitHub and each VM, before/after, without credentials. Configuration output alone is insufficient.
2. **Credential deny plus migration.** Measure the provider/policy exposure matrix using synthetic fixtures. Each deny must fail with enforcement and succeed after reverting that specific rule in an isolated test profile. Include provider authentication and disposable Keychain retrieval tests.
3. **`hydra seal`.** Measure interoperability with pinned `age`: both authorized recipients decrypt; wrong identities, corruption, truncation, and mixed recipients fail; failures leave no published plaintext or partial ciphertext.
4. **Build 382.** Require NIST vectors, independent verification, rejection of flipped bytes, wrong keys, missing either signature, and stale rounds. Replay round one through the actual verified loading path successfully.

## Pending from me

None for this panel. Human decision and implementation remain.

## Changed

Nothing. No repository writes, commits, or credential contents accessed.

## Found

Two code checks support the research: `render_sbpl` starts with `allow default`, while `rules_of` adds specific read denies; `copilot_block` restricts home writes, not reads, with explicit write exceptions. `denied_write_paths` does not establish credential read protection. The research’s signature recommendation also needs an explicit **both-signatures-required** rule.
