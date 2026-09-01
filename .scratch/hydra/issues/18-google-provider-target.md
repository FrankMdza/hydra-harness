# 18 - Which binary is the Google provider?

Type: grilling
Status: open
Blocked by: 01

## Question

Ticket 01 found that the plan's Google side does not work. `gemini` 0.42.0 with `oauth-personal` returns `IneligibleTierError: UNSUPPORTED_CLIENT`: Google retired Gemini Code Assist for individuals on this client and points users at Antigravity.

Three options, none clean. Pick one and say what it costs.

1. **`gemini` with a `GEMINI_API_KEY`.** Keeps the best structured output on the Google side: six event types, real token stats, and a full ACP mode carrying thoughts and answerable permissions. Costs the subscription. Google moves to the raw-model plane and every call is per-token spend.
2. **`agy -p`.** Keeps the subscription, since `agy` authenticates by browser OAuth and has a `useG1Credits` setting for Google One AI Premium quotas. Costs everything structured: no events, no session id on stdout, no token counts, no cost, no tool visibility. The same lossy tier as `hermes -z`.
3. **The Antigravity Python SDK.** Better fields, wrong shape, and it needs an API key anyway, so it buys nothing over option 1 while adding a Python sidecar.

Also settle:

- Whether the harness targets `gemini --acp` rather than `gemini -p`, since ACP is the only Google surface carrying thoughts and answerable permissions
- What D4 says about Google now. If the Google provider needs an API key, it sits on the raw-model plane, which contradicts the intent that interactive work rides subscriptions
- Whether it is worth running both: `agy` for interactive work on the subscription, `gemini` on a key for anything that has to be recorded properly
- The watch condition. If `agy` ever grows an `--output-format`, it becomes the only surface pairing the subscription with a structured agent, and this decision reopens

Nobody is logged into Antigravity on this machine, so option 2 needs an `agy` login before it can even be tried.
