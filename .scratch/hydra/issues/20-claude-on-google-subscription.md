# 20 - Claude models on the Google subscription

Type: grilling
Status: resolved
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

## Answer, 2026-09-04

The ticket asked five things. Four are settled below and the fifth, whether this changes the
delegation policy, is settled by keeping something off a face rather than adding to it.

The route exists, it is reachable by name, and Hydra never takes it on your behalf. That is a
narrower answer than the grilling started with, and the path to it is recorded here because two
rounds of it were reversed by a fact nobody had checked.

Versions re-read at the top of the session: `claude` 2.1.257, `cursor-agent` 2026.09.02-c22c1a3,
`agy` 1.1.25.

### The model list today, and what moved

Re-read on 1.1.25 because the evidence pass ran on 1.1.23 and this list is the ticket's load-bearing
fact. Fourteen ids, up from eleven.

```
gemini-3.8-flash-high / -medium / -low      <- new since 1.1.23
gemini-3.7-flash-high / -medium / -low
gemini-3.6-flash-high / -medium / -low
gemini-3.1-pro-high / -low
claude-sonnet-4-6           Claude Sonnet 4.6 (Thinking)
claude-opus-4-6-thinking    Claude Opus 4.6 (Thinking)
gpt-oss-120b-medium         GPT-OSS 120B (Medium)
```

**The Gemini ids advanced a generation and the Claude ids did not.** Google moved its own models 3.7
to 3.8 in three days and left Claude at 4.6. So the Google side is not tracking Anthropic's releases
at the same cadence, and the generation gap this ticket is about is more likely to widen than close.
Anyone re-reading this in a month should re-run `agy models` before trusting the gap's size.

Checked against the Claude model reference rather than from memory: **Opus 4.6 and Opus 5 are three
releases apart**, 4.6 to 4.7 to 4.8 to 5. Both carry a 1M context window and the same list price, so
the gap is generation alone. Neither context nor money is the reason to prefer one.

### It is a model choice, not a provider

`hydra run --provider agy --model claude-opus-4-6-thinking`. Nothing new in the router.

Every provider-specific fact this map has settled is a property of the binary rather than the model:
ticket 26's hook format, ticket 23's MCP path, ticket 10's `--add-dir` and skill root, ticket 21's
environment allowlist, ticket 18's permission mode. None of them change when a Claude id is passed,
so a distinct provider row would duplicate eleven settled facts to carry one field.

The cost is that `provider` stops answering which vendor's model replied. That is what the
attribution field below is for, and a field is cheaper than a provider.

### The substitution question, and the two rounds it cost

The evidence pass had already corrected the ticket's premise. This is not one model reachable two
ways. It is Opus 4.6 on Antigravity against Opus 5 on Claude Code, and the harness gap is wider than
the model gap: 56 tools against 25, browser and IDE heavy against file and shell, no reasoning
content on the stream, no cost or attribution fields, and 16,338 uncached input tokens for a two-word
answer against 2 billed after caching.

The grilling worked it in this order, and the order matters because the last step reversed the
middle two.

**First, the only case where the harness gap cannot bite is `judge`.** Ticket 27 made `judge` allow
nothing at all, so the tool count, the permission model and the browser surface are all irrelevant to
it, and the work is to read a diff and return one JSON answer. Everywhere else a three-generation
model gap plus a different tool set changes what a run produces in ways the record cannot explain
afterwards.

**Then, the trigger looked easy.** Ticket 15 already defines the failure condition, since it requires
all three judges to return a parseable verdict or there is no review. A trigger keyed on that needs
no rate-limit parsing, which is worth a lot: the `status: "rejected"` shape is a third-party report
nobody here has observed, and the research caught `utilization` going backwards, 0.24 to 0.05 on
`seven_day` against an unchanged `resetsAt`.

**Then ticket 15's panel killed it.** The panel is three judges, one per routable provider, and its
independence rests on one sentence: model diversity comes from provider diversity. Substituting the
`claude` judge onto `agy` produces `agy` running Claude 4.6, `cursor-agent`, and `agy` running
Gemini. Two judges on one binary, one login, one quota and one system-prompt harness, each paying
`agy`'s 16,338-token floor. Model diversity survives, since the vendors are still Anthropic, Cursor
and Google. Process independence does not, and if the Google subscription is the limited one then two
judges die together and the panel is at one responder, far under quorum.

**So there is no automatic substitution anywhere, and the class named in the first round turned out
to have no members.** `agy --model claude-opus-4-6-thinking` is a thing you type. It is never
something Hydra reaches for.

The deciding argument is ticket 15's own. It met this problem in the auth case and answered stop and
tell the human, on the reasoning that nothing was landing unattended anyway. A rate limit is the same
class, a provider temporarily unreachable through no fault of the patch. Building a fallback whose
failure mode is two correlated judges buys uptime for a system where a human merges every patch, and
that uptime is worth less than a gate that degrades quietly costs. Ticket 15 names quiet degradation
as the one property a gate must not have.

### What rung two does instead when `claude` is limited

**A rate-limited judge aborts the review**, in the shape ticket 15 already uses for auth failure: a
remediation string naming the provider and what fixes it, and rung two waits. No shrinking panel, no
substituted seat.

### The rate-limit reading is recorded and never acted on

`rate_limit_event` arrives on a run's stream, so Hydra only ever holds a reading from a previous
`claude` run. D10 puts those on disk, so the reading is free to consult and it goes in the record.

**It changes no routing decision.** With no substitution anywhere, there is nowhere for a pre-check
to route, and a reading from a past run can be stale in the direction that hurts. Hydra launches, and
the provider refuses if it refuses. A provider refusing its own limit is a fact; Hydra predicting one
is a guess.

**It keeps one narrow job.** Rung two can read the last `claude` run's `status` before starting a
review and abort early, rather than spending two other judges' quota on a review that cannot reach
quorum. That is an optimisation on an abort that happens either way, not a decision, and it fires on
the vendor's own three-value enum rather than on a threshold Hydra invents. Ticket 14 ruled out
thresholds on the grounds that Hydra has run zero times and has nothing to calibrate against, and
`utilization` is the field the research caught moving backwards. `resetsAt` gives the reading a free
staleness test: a reading whose window has already reset says nothing.

### Attribution: one field, set at spawn

**Hydra writes which subscription paid, from what it launched**, rather than parsing it off a stream.

`claude` answers the question three independent ways, through `apiKeySource`, through
`modelUsage[*].provider`, and through the presence of `rate_limit_event` at all, which the binary's
own schema says appears only on claude.ai subscription sessions and never on API-key, Bedrock or
Vertex ones. `agy` answers nowhere on the stream. Hydra knows which binary it spawned and which
subscription that binary is logged into, so authoring the field is the only version that answers
identically on all three providers. This is the map's standing pattern rather than a new idea: ticket
09 made Hydra own run ids, timestamps and delegation lineage for the same reason, and ticket 30 added
a decider field Hydra sets because a provider's own reason text could not identify which gate fired.

**One field, naming the payer.** The model's vendor stays derivable from the model id, since every id
on all three providers names its vendor in the first token, `claude-`, `gemini-`, `composer-` or
`gpt-oss-`. Ticket 09 carries a no-stored-derived-fields rule with one written exception, and a
vendor field would be a second exception bought for nothing.

Reading `agy`'s CLI log for the account email was considered and rejected. It is worse than absent:
it pulls an address into the record that ticket 19's redaction would then have to scrub, and Hydra
would have chosen to put it there.

**A plane field cannot do this job.** D4 says the plane names the auth mechanism and nothing else,
and both surfaces here are the agent-CLI plane. Telling the two subscriptions apart is the whole
point of this ticket, so the field names the subscription rather than the plane.

### No `model` on the delegate face

Ticket 11's MCP face stays `delegate(provider, task, files?, policy?)`. The model comes from the
policy and Hydra's config.

Ticket 11 kept three things off that face because they are how a delegation escapes its worktree,
outlives its budget or spawns a tree, and it called them a human decision rather than a per-call
choice a model makes unattended. `model` belongs in that group for the same reason: it is the lever
that lets an unattended orchestrator send a refactor to a three-generation-old model to dodge a limit
it noticed. The CLI keeps the full surface, which is ticket 11's own split.

### One record trap

A rate-limited `claude` run reports `subtype: "success"` with `is_error: true` and
`api_error_status: 429`, per the third-party report in the research file. **A recorder keying off
`subtype` files an exhausted run as a clean one.** Ticket 14 already named this class, since three of
four providers file a failed run as clean, and it is the reason rung one gained a fifth failure
class.

So a rate-limited run is its own terminal outcome, in the shape ticket 12 used for auth failure and
ticket 27 used for a gated run. It is a run that did no work through no fault of the task, and rung
one must not read it as either a clean run or a task failure. This follows the map's pattern rather
than a grilling answer, and it is the one item here decided by precedent.

### Observations, no tickets opened

**`claude-sonnet-4-6` needs no separate decision.** It is a second model id inside the `agy` provider
and everything above applies to it unchanged.

**`gpt-oss-120b-medium` is on the same subscription**, so the overlap this ticket is about is not
limited to Anthropic. Under the model-choice answer it is already reachable with no new decision, and
nothing on the route to the destination waits on it.

**Three things stay unverified from the evidence pass** and belong in the harness spec as caveats
rather than claims: whether `--effort` does anything to a Claude id on `agy`, whether
`thinking_tokens: 0` means the field is unwired for Claude ids, and how much of `agy`'s 16,338-token
floor a resumed `--conversation` run recovers from cache. None of them change an answer above,
because none of them are on a path Hydra takes automatically.

### What this amends

- **Ticket 15** says model diversity comes from provider diversity, "the only kind three
  subscriptions buy". That is now false as written, since `agy --model claude-*` buys model
  diversity without provider diversity. It also gains a rate-limited judge as an abort in the auth
  case's shape, and an early abort that reads the last `claude` run's `status`.
- **Ticket 11** keeps `model` off the delegate face, for the reason it kept three other things off.
- **Ticket 09** gains one attribution field Hydra sets at spawn, the recorded rate-limit reading, and
  one terminal outcome for a rate-limited run.
- **Ticket 18** settled `agy` as the Google provider. Its Claude ids are a model choice inside it and
  not a fourth route.
- **D4** on the map holds, and gains a consequence: inside the agent-CLI plane the paying
  subscription and the model's vendor can be different parties, which is what the attribution field
  exists to record.
