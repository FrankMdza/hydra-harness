# Panel review of the plan of 2026-10-15

## Verification of repository claims

I verified the following claims in the repository using file inspection and code search.

First, relative path traversal in MCP arguments. In src/policy/gate.py lines 457 to 469, protected_path_named_in matches raw strings against denied prefixes without path canonicalization. Its docstring at lines 463 to 465 admits that a relative path in an MCP argument walks through. Lines 1541 to 1543 call this check on mcp_call without resolving paths against worktree root. In tests/gate.rs lines 3330 to 3364, an_mcp_call_naming_a_protected_path_is_refused_under_permissive checks only absolute paths. IMPLEMENTATION.md lines 253 to 259 prematurely claims this door was closed.

Second, unrecorded non-shell gate refusals. In src/policy/gate.py lines 1806 to 1807, record_refusal records evidence only when capability is shell and command is a string. When any other tool is denied, no row is written to gate-refusals.jsonl. This confirms the plan diagnosis of run 01a14290-c0a1.

Third, codex capability table drift. In src/policy/table.rs lines 395 to 401, CODEX lists Bash, apply_patch, update_plan, spawn_agent, and unhooked WebSearch. Tool webrun from codex 0.160.0 is absent, causing the gate to refuse it as hydra/unknown-tool.

Fourth, divergence in secret redaction lists. In src/export.rs lines 625 to 638, redact_secrets matches twelve pattern entries, plus OpenAI keys at line 676. In contrast, src/record/secrets.rs lines 11 to 17 scans only five classes.

Fifth, protected config counts. CLAUDE.md line 203 previously stated five config files. Commit 4fe258f updated it to six, aligning with SECURITY.md line 68 and harness spec 15.1.

Sixth, stale admission documentation. Commit f1a4787 amended admission comments in src/mcp.rs. However, protected config config/mcp.md lines 41 to 42 still asserts that mcp_call is granted by permissive alone.

Seventh, apprentice attempt diversity. In crates/apprentice/src/attempt.rs line 89, temperature is hardcoded to 0, confirming that diverse attempts per task cannot currently be sampled.

Unverified from source: I did not run the test suite or launch binaries, following instructions. Runtime behavior of codex 0.160.0 hook payloads and live memory on legacy hardware remain unverified locally.

## Answers to the plan questions

Question 1 on exposures to fix before public release.
Three items must be fixed before release rather than stated. First, build 348 must resolve relative paths in MCP arguments in gate.py. Second, build 339 must write gate_refusal rows for all denied calls to maintain transcript integrity. Third, build 338 must add webrun to the codex capability table. Other items, including the shell token residual, unconfined Linux, and admission environment reach, should be stated in SECURITY.md under known exposures.

Question 2 on contract format sizing.
A directory per contract suits standalone components and multi-language verification. For micro-tasks built by small models, separate directories add friction. A unified JSONL task bank following crates/apprentice/src/bank.rs is preferable for high volume tasks.

Question 3 on evaluation of tickets 338 to 347.
Ticket 343 for ponytail trial should not be opened. Ponytail npm package ships executable code, adding unvetted supply chain risk without improving harness mechanics. Ticket 339 lacks a schema update in docs/run-record-contract.md for arguments_sha256. Ticket 345 is blocked until map 133 selects a model. Tickets 338, 339, 340, 341, 344, 346, and 348 are sound.

Question 4 on initial measurements and fatal results.
Measure CPU routing entropy and latency for a sub-1B dense model and a small MoE using Transformers output_router_logits on apprentice tasks without a GPU. The cheapest fatal result is finding that routing lacks domain clustering across code and math, or that CPU throughput falls below two tokens per second due to memory thrashing across total parameters.

Question 5 on project comparison and evaluation value.
Compare Hydra against Aider and OpenHands. The sole metric should be audit containment and replay fidelity, measured by percentage of tool calls verifiable against immutable sidecars without undetected side effects. Spending a week on broad feature comparisons is wasteful because Hydra avoids feature sprawl by design. A one-day focused benchmark suffices.

## Outside claims fact check

Claim 1 on Xiaomi MiMo-V2.6 Pro parameters and architecture.
Confirmed. MiMo-V2.6 Pro is an open-weight mixture of experts model with 1.02 trillion total parameters, 42 billion active parameters per token, 70 layers, 384 routed experts with 8 active, and 1M context.
Primary source at https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Pro-RL and https://arxiv.org/abs/2610.11959

Claim 2 on Xiaomi MiMo-V2.6 Flash parameters and architecture.
Confirmed. MiMo-V2.6 Flash has 309 billion total parameters, 15 billion active parameters per token, and 256 routed experts.
Primary source at https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Flash-RL and https://arxiv.org/abs/2610.11959

Claim 3 on Xiaomi MiMo model licensing.
Confirmed. Model cards for MiMo-V2.6 Pro and Flash specify the MIT license.
Primary source at https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Pro-RL

Claim 4 on MobileMoE parameter sizes and licensing.
Confirmed. MobileMoE-S has 1.3B total and 0.272B active parameters. MobileMoE-M has 2.8B total and 0.528B active parameters. MobileMoE-L has 5.3B total and 0.922B active parameters. Meta released them under the FAIR Noncommercial Research License.
Primary source at https://huggingface.co/facebook/MobileMoE-S-Base and https://arxiv.org/abs/2408.06202

Claim 5 on OLMoE-1B-7B parameters and license.
Confirmed. OLMoE-1B-7B features 6.9 billion total parameters, activates 1.3 billion parameters per token across top 8 of 64 routed experts, and is licensed under Apache 2.0.
Primary source at https://arxiv.org/abs/2409.02060 and https://huggingface.co/allenai/OLMoE-1B-7B-0125

Claim 6 on Anthropic Terms of Service on competing model training.
Confirmed. Anthropic Consumer Terms prohibit using outputs to develop competing products or services, including training AI or machine learning models. Commercial Terms prohibit accessing services to build competing products or train competing AI models.
Primary source at https://www.anthropic.com/legal/consumer-terms and https://www.anthropic.com/legal/commercial-terms

Claim 7 on OpenAI Terms of Use on model training.
Confirmed. OpenAI Terms of Use Section 2 states users may not use output to develop models that compete with OpenAI. Business terms prohibit developing competing artificial intelligence models.
Primary source at https://openai.com/policies/terms-of-use/ and https://openai.com/policies/business-terms/

Claim 8 on Google Gemini API Additional Terms.
Confirmed. Google Gemini API Additional Terms state users may not use Services to develop models that compete with Services, nor attempt to reverse engineer, extract, or replicate components including underlying data or models.
Primary source at https://ai.google.dev/gemini-api/terms

Claim 9 on DietrichGebert/ponytail repository metrics and license.
Wrong with the correct values. The repository has roughly 164,000 stars, 8,866 forks, 26 releases reaching v5.2.0 on 2026-10-13, MIT license, and origin date 2026-06-12. However, the repository lists 8 open issues, whereas the plan asserts 16 open issues.
Primary source at https://github.com/DietrichGebert/ponytail

Claim 10 on legacy 2010 desktop CPU instruction sets and memory bandwidth.
Confirmed. Desktop CPUs from 2010 including Intel Core i5-750 and Core i7-920 support SSE4.2, while AMD Phenom II X4 supports SSE4a but lacks Intel SSE4.1 and SSE4.2. All lack AVX and AVX2. Real-world dual-channel DDR3-1333 memory throughput achieves 10 to 14 GB per second.
Primary source at https://www.techpowerup.com/cpu-specs/core-i5-750.html and https://www.anandtech.com

## Pending from me

No further tool calls or background investigations are pending.

## Changed

Created .scratch/hydra/research/133-panel-gemini.md. No repository source code or protected configuration was modified. Nothing was committed.

## Found

The plan correctly identifies omissions in gate refusal logging and path resolution in MCP arguments. Outside claims regarding model architectures, sizes, vendor legal terms, and hardware instruction sets are confirmed, with a minor discrepancy noted in ponytail open issue counts.
