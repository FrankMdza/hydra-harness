# Seat: Gemini — Decision memo for map ticket 140

## 1. The short and map 138

**Vote: No changes to map 138's votes, and no added ticket in the 384–386 order.**

MIT CSAIL's research shows that weight-space programming repaired looping MLPs in Qwen and synthesized toy controllers/games (<10M parameters), but failed completely on Gemma 3 270M at math (MATH-500 moved from 6.8% to 7.0%, "could not add an ability the model lacked"). Because map 138's mission is teaching new capabilities (shell parsing and gate classification), weight surgery cannot substitute for training. Distillation/weight-generation via frontier models also violates provider terms (Anthropic usage policy effective 2026-11-12).

Builds 384–386 must establish the basic training and validation baseline (Burn vs. PyTorch, gate classifier from zero, frozen layers vs. LoRA). Weight editing belongs in the fog after build 386.

## 2. Infinity: A, B, C or none

**Commit: Option A (Borrow ideas only).**

- **Why not C:** C directly violates the human's resolved decision on ticket 137 ("none of the agents' records stay in the cloud"). Infinity persists conversation histories in Aurora DSQL and DynamoDB. Running on raw Bedrock/model APIs bypasses Hydra's fixed CLI subscriptions, incurring variable token and cloud infrastructure charges ($/call), while introducing AWS bearer credentials into map 139's threat model.
- **Why not B:** B is premature. The lab has no tool-calling nano model yet (blocked by builds 384–386).
- **Cost of A:**
  - **RAM:** 0 MB added.
  - **Dollars:** $0.
  - **Hydra contract:** Preserves D5 (single append-only run record) and the deterministic process lifecycle in harness spec section 10.

## 3. If A, which idea first

**First idea: Reserved-suffix synthetic event tool results.**

- **What changes:** `src/face.rs` and child result formatting.
- **Spec sentence touched:** Harness spec section 8 ("Observation and the face") and section 9 ("The MCP delegate face"), formalizing that event payloads entering through synthetic tool results are bounded reports and never executable instructions.
- **Map decision needed:** No; it is a safe formatting and parser refinement inside `src/face.rs`.
- **Does "wake on message" break liveness?** **Yes.** Harness spec section 10 mandates: *"Process exit is the authoritative end of a run. Liveness is derived, never announced... A run is alive when a process matching both still exists."* If a waiting parent exits, it is dead under Hydra's model. Re-waking it requires `hydra resume` (`src/resume.rs`), minting a new linked run id and spawning a fresh CLI process. Treating an exited run as "waiting/suspended" breaks the core contract; adopting true process-free suspension would require a formal map decision.

## 4. If B

- **When:** Strictly **after** map 138 produces a nano model that calls tools (post-build 386).
- **Library vs. loop:** **Write the loop.** Infinity's `ModelProvider` trait (`infinity-provider-protocol`) is designed for remote HTTP/streaming endpoints (Bedrock, Rig), introducing serialization and async network overhead unsuited to in-process Burn inference sharing unified memory on Apple Silicon. Infinity's HTTP RAP protocol and external store model also clash with Hydra's local append-only run files.
- **Checked claim on `infinity-agent-core`:** Research 140 claims that `infinity-agent-core/src/stores.rs` ships in-memory `ConversationStore` and `StateStore` trait implementations that skip processed message ids. Checked against the source on GitHub: `InMemoryConversationStore` and `InMemoryStateStore` exist exactly as described, backed by `HashMap`s for testing and local embedding.

## 5. Security

Injected events introduce an external channel vulnerable to spoofing and prompt injection. Map 139 contributes two essential defenses:

1. **Dual-signed cryptographic provenance (Build 382):** If events cross process or network boundaries, they must carry canonical ML-DSA-65 + Ed25519 signatures. Locally, Hydra must rely on path-scoped UNIX sockets (`seats.sock`, harness spec 10) rather than open HTTP RAP endpoints.
2. **Credential deny (Build 387):** Confining bearer tokens prevents agents from accessing remote event endpoints or leaking cloud credentials. Injected events must remain strictly bounded data inside face markers.

## 6. The tickets

1. **Build 389 — Reserved-suffix event parser and boundary enforcement in `src/face.rs`.**
   - *Measures:* Parser overhead and prompt-injection resistance of marked event payloads.
   - *Test:* Unit test in `tests/face.rs` feeding synthetic events (`__injected`) containing simulated instructions; verifies the model/parser treats them strictly as passive data without tool spoofing.
2. **Build 390 — Stable input ID deduplication for local IPC and seats.**
   - *Measures:* Idempotency and replay suppression overhead.
   - *Test:* Integration test submitting duplicate request IDs to `seats.sock`; asserts the second call returns the cached result without duplicate processing.
3. **Build 391 — Benchmark: Detached CLI holding vs. `hydra resume` continuation.**
   - *Measures:* RSS memory differential and invocation latency between holding a CLI process during delegation vs. terminating and invoking `hydra resume`.
   - *Test:* Automated benchmark executing identical multi-turn tool flows under both models, reporting RSS memory delta and round-trip time.

---

### Research claims checked

1. **Infinity 75,000 agents extrapolation:** Checked against AWS's blog (`/tmp/r140/web/post.html`). Confirmed: 80,000 agents ran on an AMD EPYC 9R14 with in-memory stores and a *simulated model* averaging 100 KB/agent, and the 75,000 figure was an arithmetic extrapolation to an 8 GB Raspberry Pi 5.
2. **Weight-space programming on Gemma 3 270M:** Checked against MIT CSAIL's post (`/tmp/r140/web/blog.html`). Confirmed: Astra failed to improve Gemma on MATH-500 (6.8% base to 7.0% edited), and could not distinguish sycophancy from weights alone without attention patterns.

---

## Pending from me

Nothing. No background tasks or pending writes.

## Changed

Nothing in the repository. No repository files created, modified, or deleted.

## Found

- `infinity-agent-core` version 0.2.0 relies on async HTTP/RAP protocols and external state stores, making it incompatible as an in-process library for local Burn models.
- "Wake on message" directly conflicts with harness spec section 10's liveness derivation unless modeled as clean process termination followed by `hydra resume`.

