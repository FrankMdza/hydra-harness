# Decision panel for map ticket 139

You are one seat on a decision panel. Read only: do not edit, create or delete a file in the
repository, and do not commit. Never open, print or copy the contents of a credential file; paths
and modes only. Scratch experiments under `/tmp` are fine. Your seat name is in your task line.
The human is present and decides on the panel's recommendation, so commit to one option per
question and say why.

## Read

1. `.scratch/hydra/issues/139-a-post-quantum-security-system-of-our-own.md`, the human's words
   and the session's correction.
2. `.scratch/hydra/research/139-post-quantum-route.md`, the research, read 2026-11-04.
3. `.scratch/hydra/build/382-a-signed-round-manifest.md`, the build this answer unblocks.
4. `src/sandbox.rs` (around lines 465, 590, 854 and 941) and `src/policy/mod.rs` around 794, to
   check what the research says Hydra denies.

## Questions

1. **Order of version one.** The research proposes four items: a read deny on credential paths, a
   `hydra seal` over `age` post-quantum recipients, the signed round manifest, and the SSH
   `KexAlgorithms` line. Keep, cut or reorder, and say which protects the most for the least.
2. **The credential conflict.** A seatbelt profile covers the whole process tree, so a provider
   that must read its own token cannot be separated from its shell. (A) Deny every credential path
   except the launched provider's own. (B) Move what can move to the Keychain (`codex` keyring,
   `gh`) and deny the files outright. (C) Both. (D) Something else. Say what each leaves readable to
   a gated shell, and what a test would show.
3. **Encryption at rest.** (A) Shell out to the Go `age` binary with `age1pq1` recipients. (B) A
   Rust composition over `aws-lc-rs` ML-KEM following SP 800-227. (C) Wait for `rage` to ship
   `age1pq`. The human wants the lab in Rust; weigh that against writing a composition.
4. **Signing.** Which crate signs (`aws-lc-rs`, `libcrux-ml-dsa`, `fips204`, RustCrypto `ml-dsa`),
   whether a second crate verifies in a test, and whether the manifest carries Ed25519 beside
   ML-DSA. Where the key lives on the VM and on the Mac, and what the signature cannot prove when
   the training code can reach the key.
5. **The tickets**, at most four, smallest first, each with what it measures and how a test
   proves it, including a verify-by-reverting step for any deny.

Check at least two claims of the research against the code or its sources and say which.

At most 900 words. End with `## Pending from me`, `## Changed` and `## Found`.
