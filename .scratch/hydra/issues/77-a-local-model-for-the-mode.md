# 77 - A small model of Hydra's own: what job, where it trains, where it serves, at what price

Type: research
Status: claimed
Blocked by:
Opened: 2026-09-24, by the session, at the human's request after ticket 75's local-model note.

## Question

The human wants a model of Hydra's own, small, at 4 bits, trained on what Hydra records, and the
machine that runs Hydra (an M2 with 8 GB, swap already at 3 GB with the agents alive) is the
bottleneck for both training and serving. Colab is out for serving (ticket 75's note: 12-hour
runtimes, and the FAQ forbids web services and proxies). So: which cheap online services would
train a 4-bit model from scratch or fine-tune one at low bit width on Hydra's exported runs, which
would host the result as an endpoint the mode can call, and which platform's own fine-tuning and
inference (Together, Fireworks, OpenAI, Google, Anthropic, Mistral, or others) would do both without
us owning a GPU? Priced per month at this human's scale: hundreds of runs, a corpus of 380 records
today, one user in Medellín paying in USD.

Two jobs are on the table and they need different answers: first, the mode's classifier and
router (task class and grade, provider from a table), which needs no generation; second, a
generating model that could sit on the mode's planning or judge panel, which does. D12 keeps
training out of Hydra's scope; this ticket asks where it would happen, not builds it.

## Method

One Opus subagent with web search, writing `research/77-a-local-model-for-the-mode.md`: prices
and limits from the vendors' own pages, dated; GPU-hours for a from-scratch low-bit pretrain at
100M, 300M and 1B parameters from published reports; one recommended path per job with a monthly
cost, and what the human gives up by taking it.
