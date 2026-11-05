# Seat Opus, map 137

## 1. What goes to the cloud: A

Train only on ephemeral spot GPUs, and keep Hydra and every agent on the Mac. Round one already showed the shape: one spot T4 for 65 minutes, about 0.25 USD, VM and disk deleted by hand, no bucket. B and C both put the vendor CLIs' OAuth logins on a machine that stays up. They both also leave records on Google's disk longer than one round, which breaks the 2026-10-03 rule. And they both run an always-on cost against a budget that only warns and never cuts. C also gives up Seatbelt, which is the measured layer on this Mac.

What would change my vote: the nightly bank needing amd64 task images, like MiMo's images of up to 6 GB, that the Mac can only run under emulation. Then I would pick a B with no vendor logins, local models only, one spot VM per night, deleted at the end.

## 2. How the experts come home: A

One dense base plus a LoRA adapter per task, with `system-one` as the router until build 294 gives Kev a number. This is the only option that fits. Kev uses 3.0 GB steady and 4.5 GB at peak (`.scratch/hydra/research/109b-kev.md:88-89`), and swap already stood at 6.9 of 8 GB. OLMoE is 4.2 GB at Q4, so C would put 7.2 GB of weights on an 8 GB machine before the OS or an agent runs. B grows total parameters with every expert, and total parameters set the RAM. Merging experts trained separately into a sparse MoE is also research, not a recipe. A dense base under about 1.5B plus adapters of a few MB each keeps the total flat.

What would change my vote: map 133's `expert_profile` crossing its pre-stated threshold on OLMoE. Even then I would study C in the cloud and still not serve it on the Mac.

## 3. Where post-quantum enters: A, then B, then D, then C

My crate for A is `ml-dsa` from RustCrypto. I did not check its version or audit status.

- **A, provenance.** This defends against a real threat on this machine today: a file from a VM being loaded on the Mac. It fits the house pattern of refusing before acting. The signing key should be generated on the VM per round, and its public half read on the Mac from the serial console before training starts. SHA-256 alone gives integrity (`sha2` is already in `Cargo.lock`). ML-DSA adds proof of origin and is the lab's first real use of FIPS 204.
- **B, at rest.** Harvest-now-decrypt-later is the actual post-quantum threat model. But the GPU needs plaintext to train, so "encrypted to a key that stays on the Mac" protects a bucket or an export bundle in storage, not the training step. Round one used no bucket. B fits a later stored-export case.
- **D, a narrow expert that checks a repo for non-PQ crypto.** It matches the human's access goal and the verifiable-rewards answer to 133's question 3. It waits on the training path, though, and a grep does most of the job today.
- **C, in transit.** There is no Hydra connection to change. Hydra has no TLS stack at all: its only HTTP client is plain HTTP limited to 127.0.0.1, localhost or [::1] (`crates/records/src/loopback_http.rs:54`, `:101`). Traffic to the cloud goes through `gcloud`, which is not Hydra's code.

What would change my ranking: a stored export bundle becoming part of the round. That would move B to first.

## 4. Ideas from documents 1 to 4

- `tree-sitter-bash` in the gate: map now. The shell check is the weak layer the 2026-09-14 review measured. I did not check whether the shell check is Rust or the compiled Python gate, and that decides the cost.
- Kev as Guardian: fog until build 294 step 3. 258 ms on every tool call is slow, and a model that rates risk can be prompt-injected.
- Verify-on-stop: map now, as a question about which of the four providers have a stop hook.
- Short AGENTS.md-style index: map now. CLAUDE.md has grown to a changelog, and it is the human's file.
- Routing by role and cost: drop. Document 3's benchmarks and prices are unverified claims, and the routing table already lives in the human's provider decisions.
- Novelty reward: fog. The noisy-TV trap applies, and the dynamic sampler (drop task groups that all pass or all fail) is the verifiable version for bank selection.
- Rust compute and Go control split: drop. Hydra has no control plane to split off.

## 5. The first three tickets, smallest first

1. **Measure base plus adapters beside Kev.** Load a dense base under 1.5B with two adapters swapped in a loopback server, next to Kev's server and one agent. Record steady and peak footprint, swap, and time per adapter swap. It worked if the table exists and meets thresholds stated before the run. If it does not fit, Q2's A dies here.
2. **Signed round manifest.** The VM writes a SHA-256 manifest and an ML-DSA-65 signature. The Mac verifies against the key it read at boot and refuses on a mismatch. It worked if tests that flip a byte, use the wrong key or omit the signature are all refused, and the round-one adapter replayed through it loads.
3. **Round two, one adapter.** One narrow task with a verifiable reward, on a spot VM, signed by ticket 2, loaded through ticket 1's setup and routed by `system-one`. It worked if a per-tool held-out score beats an always-Bash baseline, which round one could not show, and the bill reads under 1 USD.

## Claims checked

- The export allowlist is `~/hydra` only: `config/export-allowlist.json:1-3`. Map 77 said that file did not exist, so it now does.
- bubblewrap with `--unshare-pid`: `src/sandbox.rs:686`.
- No post-quantum or TLS crate in `Cargo.lock` (grep for `ml-kem`, `pqcrypto`, `oqs`, `rustls`, `ring`, `chacha20`: none found).
- Kev's footprint: `109b-kev.md:88-95`.

Not checked: every number in documents 1, 3 and 4, the maturity of the `ml-dsa` crate, the gate's shell-check language, and which TLS groups `gcloud` negotiates.

## Pending from me
Nothing. The human decides on the panel's recommendation.

## Changed
Nothing. This seat only read files.

## Found
- `config/export-allowlist.json` exists now, while map 77's Answer still says it does not.
- Option C of question 3 has no target in the code, because Hydra opens no TLS connection anywhere.
- Option B of question 3 cannot cover the training step as written, because the GPU needs plaintext.
