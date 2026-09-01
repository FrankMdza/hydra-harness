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
