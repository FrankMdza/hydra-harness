# 140 research: two videos, a frontier model editing weights and AWS's sliced agent runtime

Every source was read on 2026-11-04. "Derived" marks my own arithmetic on a source's numbers.

## Short answer

The short is Better Stack's 60-second retelling of the MIT CSAIL blog post that research 138 had already found. The video description names that post and its logs repository, so the match is now confirmed rather than inferred. Research 138 got the Qwen result right. It was wrong to summarise the method as failing on small models: on networks under 10M parameters it read, fixed and wrote weights successfully. The failure was improving Gemma 3 270M at math.

Infinity is real and released under Apache 2.0, at version 0.2.0. The 75,000 figure is an extrapolation from about 100 KB per idle agent, measured with a simulated model and in-memory stores on an AMD EPYC server, not on a Raspberry Pi. Hydra cannot slice `claude`, `codex`, `agy` or `cursor-agent`, because each of them holds its conversation inside its own process. `hydra resume` is already a coarse, human-typed slice. Of the three ways it could fit, the ideas cost nothing. A sliced runtime for the lab's own nano agents fits on the Mac. Infinity in AWS breaks map 137's answer as it stands and adds new credentials to map 139's threat model.

## 1. The short, from its source

YouTube now works from this machine. Research 138's TLS failure did not recur in curl. The page for `jNuDdqEwI58` gives these facts:

- Title "GPT-6 Just Fixed Another AI by Editing Its Weights! #ai #openai #llm", channel Better Stack, published 2026-11-03.
- Description: "Source: https://rikiyat.github.io/blog/weight-space-programming/ Logs Repo: https://github.com/RikiyaT/lm_understand-logs".
- I fetched the auto-generated English transcript through YouTube's Android player endpoint.

The primary source is Takehi, Teoh, Zhang and Khattab (MIT CSAIL), "Sparks of Weight-Space Programming", <https://rikiyat.github.io/blog/weight-space-programming/>, published 2026-11-02. The short cites no OpenAI post, and I found none. The editor model was GPT-6 Astra. The short's claims match the blog:

- **What was edited and on which model.** Qwen3.5-0.8B. Astra "traced the looping to one direction written by the MLPs of layers 15 and 19, and subtracted that direction from 64 output columns in each (131,072 weights)". That is 0.016% of the model. Before the edit, "85 of its first 96 practice answers ran to the 32,768-token limit". After it, MATH-500 went from 7.9% to 48.1% and IFEval tripled. Research 138 has the full table from `exp8_math/README.md`.
- **How it was checked.** By benchmark scores under the same sampling settings, with gradients, training loops and edits to the embeddings, output layer and final norm all forbidden (research 138 section 1, from the logs README).
- **What failed.**
  - Gemma 3 270M at math: "Astra could not improve it ... none moved MATH-500 by more than about a point, a gain Astra itself reported as not established". The blog's reason is that Gemma made "ordinary reasoning errors ... with no single component behind them to switch off".
  - Experiment 7: given two copies of Gemma 3 270M, one fine-tuned to be sycophantic, Astra could not tell them apart "from the weights alone" in four attempts. It picked the sycophant in all three pairs once it could see attention patterns on texts it wrote itself.
- **What else the blog shows, which research 138 left out.**
  - Experiment 1: Astra named the game for four small grid-game networks from their raw weights, without being given the list of games.
  - Experiment 2: it named the shape each diffusion model draws from its weights.
  - Experiment 3: on 7.2M-parameter character models with the input and output layers removed, it named the training language in all three and found every copying (induction) head.
  - Experiment 4: it fixed a failing control network in one try with no tools, changing 3.5% of the weights so the car succeeds in all 1,024 episodes.
  - Experiment 5: it wrote MinAtar game-playing networks from scratch. The blog says they "outplay networks trained with PPO for 100 million steps".

**Corrections to research 138 section 1.**

1. "I could not open the short ... the match below is an inference". The match is now confirmed by the video's own description.
2. "That method failed on the one model under 300M it was tried on" (Short answer). This is wrong as worded. Gemma 3 270M was the one small *language model* it tried to improve at math. On networks under 10M parameters, experiments 1 to 5 succeeded at reading, fixing and writing.
3. The summary "it repairs a failure the model already shows, and nothing in it trains" still holds for the LLM case. The blog says so itself: "Editing weights could remove an obstacle that hid an ability, but it could not add an ability the model lacked."

For map 138, writing small networks from scratch without training (experiment 5) touches the nano-model question more than the Qwen edit does. The blog gives no cost in dollars or tokens for it.

## 2. Infinity, from its source

The primary source is Shadaj Laddad, "Introducing Infinity: a runtime for extreme agent concurrency", AWS Open Source Blog, 23 Oct 2026, <https://aws.amazon.com/blogs/opensource/introducing-infinity-a-runtime-for-extreme-agent-concurrency/>. The repository is <https://github.com/hydro-project/infinity>, which I read through `gh api`.

- **Licence, owner, status.** Apache 2.0, part of the Hydro project, "a collaborative effort between AWS and UC Berkeley". The post calls it "an early research release". The repository API reports Apache-2.0, language Rust, created 2026-04-13, last push 2026-11-04 and 692 stars.
- **Version.** Every crate tag is at v0.2.0, for example `rap-protocol-v0.2.0` and `infinity-provider-rig-v0.2.0`, and `infinity-agent-core` declares `version = "0.2.0"`, edition 2024. There are 21 crates. Among them are `infinity-agent-core`, `infinity-agent-lambda`, `infinity-daemon`, `infinity-agent-cli`, `infinity-mcp-bridge`, `rap-protocol`, `rap-client`, two provider crates and three `sandbox-*` crates. The AWS deployment under `agent/` is TypeScript CDK.
- **The slice.** "In a slice, the runtime loads the agent's history from a pluggable store, runs one model completion, persists the output, and dispatches at most one tool call. The slice then ends." Tool calls are the await points. Messages for one thread are processed "in order, one slice at a time". Different threads run in parallel. A local tool's `execute` starts the work and returns at once, and the result comes back through `send_to_input_queue`.
- **RAP's shape** (repository `docs/docs/rap/`). The AWS post does not name RAP. The repository does.
  - The runtime POSTs JSON to a tool's endpoint with `operation`, `arguments`, `id`, `callback_url`, `group_id`, and optionally `thread_ancestors` and `user_id`.
  - The tool "MUST return HTTP 200 immediately" and later POSTs one of five message types to the callback URL: `tool_result`, `subscription_event`, `oauth`, `user_choice` or `view_update`.
  - Discovery is at `/.well-known/rap-toolset`. Best-effort `/close_thread` and `/cancel_tool_call` endpoints are optional.
  - "The authentication mechanism between runtime and tool is implementation-specific." SigV4, bearer tokens and mTLS are listed as examples, and none is required.
  - Existing MCP servers run through a proxy.
- **Events.** A synthetic `receive_event__injected` call followed by a tool result, which cites the original call's name, id and arguments. The runtime "never offers this name to the model as a tool, and it rejects the call if the model tries to invoke it". By default each event goes to a temporary child thread. An "associative" event is appended to the parent instead.
- **Storage.** Two traits, a conversation store and a state store. The state store records processed message ids, so a redelivered message is skipped. In-memory versions ship in `infinity-agent-core/src/stores.rs`. The reference AWS deployment uses an SQS FIFO input queue with one `MessageGroupId` per thread, Lambda for one step per delivery, Aurora DSQL for histories, DynamoDB for deduplication state, and SQS delays plus EventBridge Scheduler for timers. The local daemon keeps a session store (`infinity-daemon/src/session_store.rs`).
- **Model APIs.** The README example uses `global.anthropic.claude-sonnet-4-6`, which is a Bedrock id. `infinity-provider-bedrock` is described as a "Native AWS Bedrock (Converse API) model provider". `infinity-provider-rig` is an "optional bridge" to the rig crate's backends, and its doc comment names "OpenAI, Anthropic, Gemini, ...". Every path is a raw model API with a key or AWS credentials. Nothing in the tree drives a vendor CLI.
- **The 75,000 figure.** "We ran the benchmark with 80,000 agents, for a total of 3.2 million completions, on a single thread of an AMD EPYC 9R14 with in-memory stores. Resident memory grew linearly, at about 100 KB per idle agent." The model was "a simulated model". Each agent ran twenty turns with completions of about 1.2 KB. The README's "75k agents in the memory of a Raspberry Pi" is 8 GB divided by 100 KB, not a run on a Pi. That works out to about 80,000 in decimal units (derived), and the post rounds it to 75,000. Most of the memory is conversation history, so the figure grows with history length. A real agent with a 100k-token context holds far more than 1.2 KB per turn.
- **The post's own open questions.** Interleaved events, scheduling, compaction in agent trees, and "Event authenticity ... agents exposed to the internet may receive inauthentic payloads."

Corrections to `140-video-notes.md`: a slice dispatches at most one tool call, and tools may be in-process, so not every tool call is a POST. The Raspberry Pi figure is extrapolated, and the measurement used a simulated model.

## 3. Infinity against Hydra

**What already resembles a slice.**

- *Process exit as the end of a unit, with state on disk.* "Process exit is the authoritative end of a run", and liveness is derived from the pid and start time in the envelope (`docs/harness-spec.md` section 10).
- *Resume as a new slice.* `hydra resume <run-id>` "reads that field and hands the provider its own resume flag ... A resumed run gets a fresh run id linked to the old one" (section 10, line 1223; `src/resume.rs`). Structurally, that is a slice boundary: nothing runs between runs, and a later process continues. It differs in three ways. A human types it, nothing wakes it on a message, and the state lives in the provider's own store rather than Hydra's.
- *A detached executor that knows nothing about the provider.* `src/watch.rs` calls itself "a dumb executor": "run this argv on a pty, tee it into this file, close it when the process exits".
- *Blocking only as a view.* "Every child is spawned detached, and blocking is a view over it ... Calls block with a timeout, then degrade to a handle" (section 9). The `delegate` tool in `src/face.rs` waits on the child, then hands back a handle. That is the shape of a subscription that has not delivered its event yet.
- *A deferred human answer.* A seat request no shape covers waits for `hydra approve <id>` (section 10, map 82). That is close to RAP's `user_choice` message.
- *Telling events apart from the user.* Infinity uses a dedicated injected name so the model can tell an event from the human. Hydra's face is "the one place a child's text is marked on the way back" (CLAUDE.md layout, harness spec 8). Both answer the same question, and Infinity lists it as open.

**What Hydra cannot do because it wraps CLIs.** It cannot end a provider's process at a tool call and continue later from a history Hydra owns. The spec says "Hydra never reconstructs conversation state itself", and the tool loop runs inside the vendor binary. A waiting `claude` keeps its process, its pty and the about 200 MB the ticket cites (ticket 140, "Why each one touches the plan"). Hydra also cannot deliver an event mid-run: "No attach, no input ... two of three providers cannot be asked anything mid-run" (section 10). Liveness is defined by a live pid, so a sliced run would read as dead between slices.

**Could the record serve as a slice's durable history?** It holds the material: "A provider line wraps one provider frame verbatim, in a `raw` field. Nothing is discarded" (`docs/run-record-contract.md` line 92), and section 7.6 derives `tool_call` and `tool_result` from those frames. Three rules stand against using it as a live store.

- "Nothing appends to a run file except the process that owns it" (harness spec 10), and a slice runtime appends from a new process each time.
- One file per run. Many slices to one file breaks it, while one slice per run would make every completion a run file.
- The frames are in each vendor's vocabulary, not a model API's message list, so replaying them into a completion call needs a converter per provider.

The shape that fits the rules is the one `hydra resume` already has: each slice is a run, linked to the one before it, with the history rebuilt at read time the way a session tree is (contract line 66). That holds for a model Hydra calls directly. For the four CLIs it does not hold, because the CLI, not Hydra, is the one that continues.

## 4. Where it could fit, without deciding

These are options for the panel. Ticket 140 rules out AWS spend, so no dollar figure below was measured, and I did not read AWS pricing pages.

**A. Borrow ideas only.** This adds no RAM or dollars.

- Use synthetic tool calls with a reserved suffix for events: a child finished, CI turned green, a seat was approved.
- Wake on a message: a parent that ends and is resumed with the event, instead of holding its process inside `delegate`.
- Add a stable id per input so a redelivery is skipped.

Nothing goes to the cloud, so map 137's answer stands. The one thing it does to map 139 is that an injected event is a new channel to authenticate. Map 139 inherits Infinity's open "event authenticity" question. The resume half only works for providers with a resume flag, and the spec says resume "fails loudly" where one is missing.

**B. A sliced runtime for the lab's own nano agents.** On the 8 GB Mac, the model's weights are loaded once and shared. Each waiting agent then costs its history, the 100 KB order of magnitude Infinity measured, instead of a provider process. Derived: 8 GB at 200 MB gives about 40 Hydra runs at most. The RAM-ceiling memory note says swap is already at 3.2 of 4 GB, so fewer in practice. Local dollars are zero beyond electricity. The state could be Hydra's own record if each slice is a linked run. Infinity's in-memory or daemon store is the other choice, but then the record stops being the single contract D5 requires. Nothing goes to the cloud, and map 139 is unchanged except for the event channel in A. It waits on map 138 producing a nano model that can call tools, which does not exist yet. Infinity's crates could be used as a library here, since `infinity-provider-protocol` lets any `ModelProvider` be plugged in. I did not check whether a local in-process model fits that trait.

**C. Infinity itself in AWS.** Compute scales to zero. Running cost is per message: SQS, Lambda and DSQL charges, plus a Bedrock or API bill per completion, because Infinity calls raw model APIs and cannot use the subscriptions the four CLIs run on. Lambda has no CLI pty, so the four routed providers would not run on it.

- Map 137: conversation histories live in Aurora DSQL for as long as an agent lives, possibly months. Map 137 answered "none of the agents' records stay in the cloud", with the training disk deleted at the end of the round. C contradicts that answer and needs the human.
- Map 139: it adds AWS credentials, a model API key or Bedrock access, and RAP endpoints whose authentication "is implementation-specific". Each of those is a bearer secret of the kind map 139 is about.

## Gaps

- No AWS or Bedrock prices were read, so options B and C have no dollar figures.
- I did not run Infinity, and I found no benchmark file by name in the tree, so the 100 KB is the post's number, not mine.
- Not checked: whether `ModelProvider` fits a local in-process model.
- Not opened: the critics R&D World cites (research 138), the logs repository's other experiments, and the Let's Get Rusty video again. Its notes came from the session.
- The short's transcript is auto-generated.

## Pending from me

Nothing. The panel prompt `research/140-panel-prompt.md` is for the session to write, since this run is read-only.

## Changed

Nothing in the repository. The only files written were downloads, in `/tmp/r140/`.

## Found

- Research 138 section 1 needs two corrections. The match is confirmed by the video's description. "Failed on the one model under 300M" should be limited to improving Gemma's math: small non-LLM networks were read, fixed and written successfully, and the sycophancy pair was read only with attention patterns.
- The 75,000 figure was extrapolated from a server run with a simulated model, not measured on a Pi.
- `hydra resume` already has the shape of a slice, but a human types it and the vendor holds the state.
- Option C contradicts map 137's answer and widens map 139. Options A and B leave both as they are, apart from a new event channel to authenticate.
