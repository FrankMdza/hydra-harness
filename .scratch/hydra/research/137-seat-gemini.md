# Panel Review: Map Ticket 137 (Seat: Gemini)

## Claims Checked Against the Repository

1. **Linux Bubblewrap Confinement**: Verified in [`src/sandbox.rs`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/src/sandbox.rs#L70) lines 70 and 116–150 ([`linux_bwrap_ready`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/src/sandbox.rs#L117)), and lines 673–775 ([`compose_bwrap`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/src/sandbox.rs#L673)). Hydra implements full Linux bubblewrap sandboxing, translating policy rules to `bwrap` arguments.
2. **Export Allowlist State**: Ticket 77 claimed `config/export-allowlist.json` did not exist. Verified that [`config/export-allowlist.json`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/config/export-allowlist.json#L1-L3) lines 1–3 now exists on disk (`["/Users/USER/hydra"]`), referenced and enforced by [`src/export.rs`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/src/export.rs#L26) line 26.
3. **Apprentice Preference Pairs (Build 351)**: Verified in [`crates/apprentice/src/pairs.rs`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/crates/apprentice/src/pairs.rs#L159-L174) lines 159–174. `pairs` enforces that both `local_answer` and `better_answer` exist among recorded attempts, dropping unrecorded frontier rewrites to avoid training term violations.
4. **Claims Not Checked**: Unverified claims from external documents include the academic benchmark metrics cited in Document 1 (arXiv 2609.00006), the pricing and performance claims for unreleased models in Document 3, and off-machine GCP billing line items for training round 1.

---

## Answers to the Panel Questions

### Question 1: What goes to the cloud, and in which order
- **Option:** (A) Training only, on ephemeral spot GPUs, with Hydra and every agent staying on the Mac.
- **Reason:** Ephemeral spot VMs preserve the 10 USD budget cap through strict launch-train-destroy lifecycles (round 1 cost ~0.25 USD on a spot T4, an unverified estimate from [`round1/README.md`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/.scratch/hydra/training/round1/README.md#L41)), avoid synchronizing vendor CLI tokens (`claude`, `codex`) to remote machines, and guarantee that training records leave the Mac only for the single GCP project and are destroyed when the instance shuts down. In contrast, running persistent headless instances under (B) or (C) continuously drains budget while idling and expands the credential attack surface.
- **Risk that would change my vote:** If local apprentice evaluation runs overload the Mac's thermal and memory limits, requiring an unattended, scheduled headless worker that executes solely under bubblewrap with local weights and zero vendor logins.

### Question 2: How the experts come home
- **Option:** (A) One dense base on the Mac plus one LoRA adapter per narrow task trained in the cloud, routed in Hydra by `system-one` or Kev.
- **Reason:** A sub-1.5B quantized base (or Gemma 3 1B/Qwen 1.5B at ~1 GB RAM) allows narrow task adapters (10–30 MB each) to be dynamically paged into memory, staying well within the 1.5 GB margin remaining alongside Kev-0.8B’s 3.0 GB steady footprint. In contrast, sparse MoEs like OLMoE-1B-7B (4.2 GB at Q4) or multi-expert merged files require all parameters in memory simultaneously, immediately thrashing an 8 GB host whose swap is already at 6.9 GB.
- **Risk that would change my vote:** The release of a permissively licensed (Apache-2.0) sparse MoE with under 2B total parameters that executes in under 1.5 GB quantized on CPU while outperforming discrete LoRA adapters.

### Question 3: Where post-quantum cryptography enters first
- **Ranking:** A > B > C > D.
- **First Choice:** (A) Provenance: an ML-DSA (FIPS 204) signature on every adapter, weight file, and export bundle made in the cloud, checked on the Mac before anything loads.
- **Rust Crate for First Choice:** `fips204` (pure Rust implementation of FIPS 204 ML-DSA).
- **Reason:** Provenance is the primary boundary defense: loading unverified binary tensors from an ephemeral, untrusted cloud instance into the Mac's local runtime creates an unvetted code/weight execution vector unless authenticated at the gate. Transport encryption (C) is already handled by outer TLS layers, while encrypting at rest (B) provides no confidentiality advantage during training because the remote training script must decrypt the records anyway.
- **Risk that would change my vote:** If ephemeral spot training environments cannot establish hardware-isolated signing keys, making cloud-side signatures unverifiable and elevating at-rest encryption of exported bundles (B) to top priority.

### Question 4: Idea triage from Documents 1 to 4
- Parsing shell commands with `tree-sitter-bash` in the gate: **fog**
- Kev as a cheap "Guardian" that rates an action's risk: **map now**
- Verify-on-stop guards: **fog**
- A short AGENTS.md-style index in place of the long CLAUDE.md: **map now**
- Routing by role and cost from document 3: **drop**
- A novelty reward from document 4 for choosing bank tasks or research topics: **drop**
- The Rust compute and Go control split: **drop**

### Question 5: The first three tickets, in order
1. **Sanitize export task bundles (smallest):** Implements task prompt and trajectory validation in [`src/export.rs`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/src/export.rs) to ensure zero data outside [`config/export-allowlist.json`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/config/export-allowlist.json) is bundled; worked when export rejects fixtures referencing non-allowlisted paths and approves clean tasks.
2. **Ephemeral spot training script:** Builds an automated spot T4 launcher script with a strict 2-hour TTL that mounts exported runs, trains a LoRA adapter, copies weights to the Mac, and self-deletes VM and disk; worked when an adapter returns to the Mac and GCP confirms zero running resources under $0.50 USD.
3. **Post-quantum ML-DSA adapter verification:** Builds an artifact verifier in [`crates/apprentice`](file:///Users/USER/hydra/worktrees/01a1a825-55c3-702c-b93b-ac786aa125ae/crates/apprentice) using `fips204` that validates adapter signatures against a pinned public key before inference; worked when authentic adapters load cleanly and a single-byte corrupted adapter is rejected with a gate refusal.

---

## Pending from me
- None. This seat's docket on ticket 137 is complete.

## Changed
- No repository files were modified, created, or deleted. All checks were strictly read-only inspections.

## Found
- `config/export-allowlist.json` exists and is pinned to `["/Users/USER/hydra"]`, resolving the prerequisite stated as missing in ticket 77.
- Build 351's preference pair restriction is fully active in `crates/apprentice/src/pairs.rs`, rejecting unrecorded frontier text.
- Memory constraints (8 GB RAM with 6.9 GB swap used, Kev taking 3.0 GB steady) make dense base + task LoRA the only viable local model architecture. Sparse MoE models cannot fit without severe memory thrashing.

