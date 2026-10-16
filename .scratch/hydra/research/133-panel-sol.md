# Adversarial review of plan 133

## Publication blockers

Recommendation 6.1 is wrong. Documentation and green security CI cannot substitute for closing whatever 134 designates fix before. My recommendation to the human is to block publication on persistent user-configuration writes, fail-open execution without effective independent confinement, and false claims of record completeness. These are recommendations, not answers entered on the grilling.

Verified by reading, `src/sandbox.rs:351` explicitly permits writes matching `.claude.json` and its siblings. Issue 99 at `.scratch/hydra/issues/99-the-gated-copilot-can-still-write-claude-json.md:15` describes planting a user-scope MCP server for a later ungated session. That is worse than an ordinary current-run path escape: it potentially transfers execution into a later session with the human's permissions. The code permission is verified; that execution chain remains inferred and untested end to end, as the ticket itself says. Deferring it until after publication is the wrong priority.

Verified, `tests/drift.rs:434` contains the Codex dead-hook fail-open test, but `tests/drift.rs:435` skips it unless paid probing is enabled. Therefore green routine CI does not establish that current provider hooks fail closed. I did not execute it.

The unknown web name is availability noise beside those risks. `src/policy/table.rs:395` lists Codex's tools without `webrun`; `src/policy/gate.py:1438` starts the unknown-capability refusal. That is safe refusal, not unauthorized execution. Its missing evidence matters separately.

Bench outcomes must not support public security claims until corrected. Verified, `src/bench.rs:212` treats every completed run as attack success without inspecting a breach artifact. That invalidates the measurement, rather than proving every completion breached containment.

## Contract size

A directory is reasonable for a shared runner, schema and explanatory prose. The proposed contract is incomplete. Define schema dialect, error envelope, process timeout, output limit, exit status and whether each case starts a fresh process. Distinguish startup latency from inference latency, with warmups, repetitions and a machine identifier. Otherwise an echo executable can hang the runner or pass correctness with meaningless percentiles. A handwritten checker should support an explicitly restricted schema subset rather than claim JSON Schema compatibility.

Cases are examples, not an implementation-independent specification. Include invariants and hidden cases. Replace “make the runner green” with satisfying both the contract and its checks.

## Tickets and surviving inputs

338 confuses login inspection with inference. Verified, `src/provider/codex.rs:472` invokes `login status`; `tests/drift.rs:429` identifies an actual Codex turn as quota-consuming. The ticket's free fetch probe is unsubstantiated. An unhooked classification also cannot be justified solely by one missing callback. Require an observed tool execution and a positive hook control.

After 338, the input `webrun` with a fetch URL spelling `file:///Users/USER/hydra/ru%6es/x.jsonl` would still pass the proposed raw path guard under permissive. This is a gate-level inference from `src/policy/gate.py:1555` and `src/policy/gate.py:467`. The equivalent encoded WebFetch case is explicitly allowed in `tests/gate.rs:2419`. I have not verified that the hosted tool dereferences local files. Also test actual fetch operations against the Slack floor: `src/policy/gate.py:1361` has no `webrun` mapping.

339 needs an explicit validator edit. Verified, `src/record/validate.rs:180` requires command fields, and `src/watch.rs:380` copies only rows accepted by that validator. Otherwise the new non-shell rows disappear again. Specify canonicalization and preserve historical absence as unknown; replay cannot recover arguments never recorded.

After 339, a forged shell-shaped sidecar row with a real failed call's id but an invented command still gets through. Verified, `tests/detached.rs:663` demonstrates accepted forged evidence. `src/walk.rs:947` joins ids and failure statuses without comparing tool, command or argument digest. My inference is that reusing a real failed id can even obtain corroboration. Hashing new arguments adds no authenticity. Separate missing-frame evidence from missing-id evidence.

340 and 341 are useful, but neither closes an exposure. Keep 342 gated on the human's decision. Do not open 343 now: five runs per arm cannot establish broad effectiveness, and ten identical tickets measures repetition rather than task coverage. Open 344 only with the runner limits above. Defer 345 as a build until model, runtime, memory budget and kill threshold are selected; map 133 and build 345 currently depend on each other. Keep 346 experimental, adding seed, sampling configuration and resume identity. Verified, `crates/apprentice/src/attempt.rs:89` fixes temperature at zero and `crates/apprentice/src/attempt.rs:303` keys existing attempts by task, prompt hash and model. Fold 347 into 344.

## Cheapest useful experiment

Recommendation 6.2 mistakes routing visibility for useful specialization. First measure one narrow task's held-out accuracy, peak resident memory, startup and warm latency with a dense baseline on the intended serving hardware. Predeclare acceptable accuracy and latency. Failure within 4 GB kills that deployment candidate cheaply, not the whole tiny-expert idea. Only then compare MoE and causal expert ablation. Domain hit counts alone cannot establish that reinforcing those experts improves performance.

The plan trusts the Gemini memo too far. Its hardware figures at `133-seat-gemini-moe.md:67` to line 74 are estimates without a firsthand legacy-machine run. Recommendation 6.5 turns them into a model selection. Likewise “one afternoon” is an estimate without build evidence. HTTP 200 establishes neither model support nor licence content.

The evaluation exemption also needs restraint. Verified, `crates/apprentice/src/pairs.rs:125` reads both local and better answers and `crates/apprentice/src/pairs.rs:138` places both into preference training fields. Those pairs are not merely labels. I did not verify their authorship or vendor terms, and infer no legal permission from the seats.

## Comparison

Use Claude Code and Codex CLI directly as two baselines against Hydra-wrapped runs. Measure observed forbidden side effects, denial recall, record completeness and overhead on identical stub tasks. One day is enough to design a pilot. A week is unjustified before trustworthy breach oracles. The plan's uniqueness claim needs evidence.

## Pending from me

No pending tool work. External sources, recorded runs, current CI, provider behavior and exploit execution were not independently verified. No tests or launches ran. No gate refusal occurred.

## Changed

Created only `.scratch/hydra/research/133-panel-sol.md`. Committed nothing.

## Found

The plan prioritizes safe refusal over persistence risk and treats broader logging as stronger provenance. Both priorities need correction before approval.
