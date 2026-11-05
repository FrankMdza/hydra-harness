GPT seat: **1A, 2A, 3A; rank A > C > B > D.** These are recommendations for the human’s decision.

**1. Cloud: A—training only first.** Keep Hydra, agents and vendor logins on the Mac; export only approved material to `<gcp-project>`, then delete cloud inputs, checkpoints, disks and other copies after retrieving the round’s outputs. Treat $10 as a total spending ceiling: bound runtime and retries against remaining funds, reserve cleanup/storage costs, and reconcile actual billing; the reported $0.25 round is an estimate, not a verified bill.

**Vote-changing risk:** If measured batch work cannot finish locally, move toward B after proving remote authentication, sandboxing, cost bounds and record deletion; bubblewrap alone does not establish those.

**2. Experts: A—one small dense base, task adapters, external routing.** This offers independently testable specialisation without constructing and training a sparse router; start below 1B parameters and load one adapter at a time, subject to map 133’s human decisions. It is the plausible fit beside Kev on 8 GB, not a demonstrated fit: the digest’s unverified 4.2 GB OLMoE footprint plus 3 GB Kev already leaves inadequate space for macOS, agents and inference overhead.

**Vote-changing risk:** If a tiny base cannot improve held-out task success, defer local expert deployment rather than assume adapter count creates capability; shared-base adapters are not equivalent to a sparse MoE.

**3. Post-quantum: A > C > B > D; first crate: `ml-dsa`.** Start with signed manifests binding artifact hashes, base-model identity, training-input digest and round identity, verified against a locally pinned trust key before loading; RustCrypto’s [`ml-dsa`](https://docs.rs/ml-dsa/latest/ml_dsa/) provides signing and verification. C follows where Hydra controls both connection configuration and endpoint support ([rustls supports X25519MLKEM768](https://rustls.dev/docs/rustls/manual/_05_defaults/index.html)); B needs a specified hybrid combiner/KDF and cannot keep training data inaccessible to the cloud while ordinary cloud training consumes it, while D first needs a deterministic inventory baseline.

**Vote-changing risk:** A signer available to compromised training code authenticates compromised output; without a separately controlled signing boundary, I would put C first. Signatures establish origin and integrity, not model quality.

**4. Ideas entering the map**

- **Shell parsing—map now:** differential `tree-sitter-bash` experiment over gate fixtures; malformed or unsupported syntax must never widen permissions.
- **Kev Guardian—fog:** finish 294 first; completion prediction does not establish action-risk accuracy, and a model must not override the gate.
- **Verify-on-stop—map now:** bounded, task-specific verification with recorded failure and timeout, preserving the human’s stop control.
- **Short instruction index—map now:** audit existing AGENTS.md links and duplication before replacing useful detail in CLAUDE.md.
- **Role/cost routing—map now:** compare measured cost per accepted task; discard document 3’s unverified model rankings as evidence.
- **Novelty reward—fog:** establish task coverage and verifier reliability before rewarding novelty.
- **Rust compute/Go control—drop:** no demonstrated orchestration requirement justifies a second language and service boundary.

**Vote-changing risk:** Evidence that an existing mechanism already meets a proposed ticket’s acceptance criteria would make me drop that duplicate.

**5. First three tickets, smallest first**

1. **Evidence and budget preflight:** assemble the existing round’s billing, approved export scope, deletion evidence and missing measurements; succeeds when every claim has evidence or an explicit unknown, with no new GPU spend.
2. **Artifact admission prototype:** implement a signed-manifest verifier with a pinned key and base/adapter binding; succeeds when valid fixtures pass and tampered, substituted, unknown-key and incompatible artifacts fail before loading.
3. **One-adapter round-trip experiment:** after preflight and 294’s outcome, train one narrow adapter within the remaining budget; succeeds only with a preregistered held-out improvement over the base and trivial prior, no material regression, and measured Mac memory/latency within an agreed envelope.

**Vote-changing risk:** Insufficient eligible, independent evaluation data blocks ticket 3; use verifier-backed fixtures first. Leave claimed build 294 with its current owner.

## Pending from me

No implementation or cloud action. The five documents’ benchmark, price, licence and memory claims remain independently unchecked; I did not verify live cloud state or rerun measurements.

## Changed

Nothing; this review was read-only.

## Found

- **Linux containment checked:** `src/sandbox.rs:137` refuses standard/permissive launches without bubblewrap readiness; `src/sandbox.rs:686` adds PID isolation. This is code inspection, not a cloud-host test.
- **Training-pair restriction checked:** `crates/apprentice/src/pairs.rs:161` rejects either answer unless recorded for that task; this supports build 351’s claim without reopening map 133’s settled training-source decision.
- **Historical correction:** `config/export-allowlist.json:2` now contains `/Users/USER/hydra`; map 77’s “does not exist” statement is stale.
- **Evidence gap:** `.scratch/hydra/build/294-kev-beside-system-one.md` remains claimed with an empty Outcome; its file does not substantiate the digest’s Kev measurements.
