**GPT seat: choose A, stable input IDs first. Keep map 138’s main votes; add one bounded human-written weight-edit experiment.**

1. **Weight editing changes the rationale, not the training plan.** Keep Burn plus the PyTorch reference and independent float64 check, frozen layers before LoRA, and the gate classifier first. Keep frontier-written weight diffs with the human for decision; technical feasibility does not settle permission to use those outputs.

   I checked the primary source: it reports successful small-network construction and repair, while Gemma 3 270M’s math improvement remained unestablished. The earlier blanket dismissal of small-model editing was wrong. These results still do not establish a replacement for training the lab’s classifier. [MIT CSAIL report](https://rikiyat.github.io/blog/weight-space-programming/)

   Add a **human-authored edit after 385, before 386**, using the lab’s own checkpoint and synthetic examples. Limit it to one diagnosed failure, with a fixed development budget and untouched evaluation set. Compare original, edit, and sham edit; measure false allows, target-error reduction, and forgetting. No frontier-generated tensors or copied experiment code. This avoids introducing an editor-model licence question. Budget: $0 incremental cloud spend, local inference only; failure does not block 386.

2. **Infinity: A.** Borrow the ideas incrementally. Immediate infrastructure/API spend is $0. RAM cost is **small but nonzero**: event metadata, duplicate tracking, and eventually a dispatcher. IDs alone save no provider RAM. Wake-on-message saves a waiting provider process only when that process actually exits; tools, histories, and the dispatcher still consume resources. Measure the saving rather than adopting the ticket’s approximate 200 MB.

   Reject C under map 137’s existing answer: durable cloud conversations exceed training-only temporary storage. Defer B until a nano model reliably calls tools; the first gate classifier does not meet that prerequisite.

3. **Stable input IDs first, before event delivery.** Define an envelope binding issuer, destination session, source run/tool call, event kind, sequence, payload digest, and stable ID. Retries preserve the ID; reuse with different content is rejected.

   This needs a **map decision before implementation**: durable acceptance and duplicate state touch D10’s “anything not in a run record did not happen,” plus the record schema. Specify crash recovery and ownership before adding a queue.

   Later, change `face.rs` to support an explicit deferred response, and route accepted events through a launcher calling `resume.rs` after the parent exits. Leave `watch.rs` as executor. Hydra cannot force an arbitrary vendor CLI to checkpoint at a tool boundary.

   **Wake-on-message does not break PID liveness.** The old run is ended; a pending continuation is a separate scheduling fact; the new run gets a fresh PID/start-time pair. Preserve section 10’s single-writer rule and fresh linked run sentence. Extend its observation/channel rules explicitly; never append an event to the ended parent or call it alive.

   Reserved synthetic calls come last. They require deliberate provider/export representation, not fabricated vendor frames.

4. **If B is later approved: after tool competence, reuse Infinity’s interfaces/core behind a Hydra adapter.** Avoid writing a second general event loop initially. I verified that `infinity-agent-core` supplies in-memory conversation/state stores; these do **not** provide durable restart recovery by themselves. [Store implementation](https://raw.githubusercontent.com/hydro-project/infinity/main/crates/infinity-agent-core/src/stores.rs)

   `ModelProvider` requires asynchronous model listing and streamed completions, with `Send + Sync`; it does not require an HTTP API. A local adapter is therefore plausible, but Burn integration, scheduling, and memory remain unmeasured. [Provider trait](https://raw.githubusercontent.com/hydro-project/infinity/main/crates/infinity-provider-protocol/src/lib.rs)

   Linked run files alone also do not resolve B’s conflict with “Hydra never reconstructs conversation state itself.” That needs an explicit nano-runtime exception.

5. **Security:** credential deny remains necessary but cannot authenticate events. Add issuer authorization, destination/call binding, replay protection, size/rate limits, and rejection before waking a model. A reserved suffix identifies representation, not authority; reject model attempts to invoke it.

   Keep any authentication key outside agent reach. Reuse map 139’s canonicalization and domain-separation discipline, with a distinct event domain. An artifact signature does not authorize a callback, and an authenticated payload remains untrusted data. No additional post-quantum machinery is necessary for a local authenticated launcher channel.

6. **Three tickets, smallest first:**

   - **Map event identity and recovery:** measure duplicate/conflict handling; fixtures prove retries deduplicate, altered payloads fail, and reconstructed acceptance agrees after restart.
   - **Bounded weight-edit experiment:** after 385, before 386; held-out comparisons prove improvement or record failure without weakening the deterministic gate.
   - **Local wake prototype:** after the map decision; stub providers test duplicate delivery and crashes around launch, live-parent refusal, unchanged parent files, correct lineage, RAM savings, and wake latency. Ambiguous recovery must not blindly launch again.

## Pending from me

None. Human decisions remain on these recommendations and frontier-written diffs.

## Changed

No files changed. My recommendation withdraws the small-model rationale for rejecting all weight-edit experiments.

## Found

Checked the MIT result, Infinity’s stores/provider interface, and Hydra’s actual resume path: it refuses live sources and launches a fresh linked continuation.
