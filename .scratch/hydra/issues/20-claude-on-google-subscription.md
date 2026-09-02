# 20 - Claude models on the Google subscription

Type: grilling
Status: open
Blocked by: 18

## Question

`agy models` lists `claude-opus-4-6-thinking` and `claude-sonnet-4-6` alongside the Gemini models,
addressable by id through `--model`, on the Google AI Pro subscription. So there are two ways to
reach Opus on this machine: `claude`, on the Anthropic subscription, and `agy --model
claude-opus-4-6-thinking`, on the Google one. Ticket 08 found one account per provider, so this is
the only place where two subscriptions overlap on the same model.

That makes routing a real question for the first time. Settle:

- Whether Hydra treats `agy --model claude-*` as a distinct provider, a model choice inside the
  `agy` provider, or a fallback route that only fires on a condition
- What the condition is, if there is one. The obvious candidate is Claude Code hitting its
  five-hour limit, which needs a detectable signal in the run record rather than a guess
- Whether the two Opus surfaces are comparable enough to substitute. Different system prompt,
  different tool set, `agy` cannot be asked for permission, and the harness around the model is
  Antigravity's rather than Anthropic's
- What the run record says about which subscription paid for a run, since `provider` alone stops
  being enough to answer that
- Whether this changes the delegation policy in ticket 11, where the orchestrator picks a provider
  for a child

Not a cost question in the usual sense. Both subscriptions are already paid for, so the currency
here is quota and capability, not money.

## What the evidence pass found, 2026-09-01

Frames, commands and the full side-by-side in
[research/20-claude-on-agy.md](../research/20-claude-on-agy.md).

- **The premise above is wrong, so reword the question before grilling it.** `agy` serves Claude
  Opus 4.6. `claude --model opus` resolves to `claude-opus-5` with a one-million-token context.
  These are not one model reachable two ways, they are two generations reachable one way each, so
  the real question is whether a downgrade is an acceptable fallback.
- **The five-hour signal already exists at rest.** `rate_limit_event` carries `rateLimitType`, a
  `status` of `allowed`, `allowed_warning` or `rejected`, a `resetsAt` epoch and a `utilization`
  fraction per window. Nothing has to break for it to appear. But `utilization` is not monotonic:
  the `seven_day` window read 0.24 and then 0.05 three hours later against the same `resetsAt`, so
  a naive threshold on it will misfire.
- **Attribution is asymmetric.** `claude` answers which plane paid, through `apiKeySource` and
  `modelUsage[*].provider`. `agy` answers nowhere on the stream, though its process knows and writes
  the account email to a log file.
- **The route is not cheap in quota.** `agy` shipped 16,338 uncached input tokens for a two-word
  answer, because its system prompt and 56 tool schemas go out on every call. Claude Code billed 2
  input tokens for the comparable prompt after caching.
