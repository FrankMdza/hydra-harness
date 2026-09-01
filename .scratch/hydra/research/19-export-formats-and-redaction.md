# 19 - Export formats and redaction, at source

Status: facts only. Ticket 19 is a grilling ticket, so nothing here decides anything. Where the
ecosystem disagrees I show the disagreement and leave it open.

Read first: ticket 19 and ticket 09's `## Answer`. Everything below assumes 09's shape, one
envelope-plus-raw JSONL file per run, eleven normalized event types, `correction` carrying the
rejected and chosen pair inline.

Primary sources fetched live on 2026-09-01. Every format claim below has a URL. Rule counts were
counted by me from the tools' own files, not read off a marketing page, and I say so where the
count I got differs from what a README claims.

---

# Part 1. The formats

## The short version

There are three shapes in play, and they are not variants of one thing. They disagree about
where the prompt ends, how a tool result is spelled, and whether a preference pair may contain
tool traffic at all.

| | SFT | Preference / DPO |
|---|---|---|
| OpenAI API | `{"messages":[...], "tools":[...]}` | `{"input":{...}, "preferred_output":[...], "non_preferred_output":[...]}` |
| TRL (HF) | `{"messages":[...], "tools":[...]}` or `{"prompt":[...], "completion":[...]}` | `{"prompt":[...], "chosen":[...], "rejected":[...]}` |
| ShareGPT (LLaMA-Factory) | `{"conversations":[{"from":..., "value":...}], "tools":"..."}` | same, plus `chosen` and `rejected` as single message objects |

The gap that matters for Hydra: OpenAI's preference format is single-turn only and takes assistant
text, while TRL's takes full message lists and explicitly supports tool calls. A `correction` event
sitting at turn 40 of a tool-heavy trajectory fits one of those and not the other.

## SFT, format one: OpenAI messages JSONL

Source: <https://developers.openai.com/api/docs/guides/supervised-fine-tuning>

One JSON object per line. The only required top-level key is `messages`.

```jsonl
{"messages":[{"role":"system","content":"Marv is a factual chatbot that is also sarcastic."},{"role":"user","content":"What's the capital of France?"},{"role":"assistant","content":"Paris","weight":0},{"role":"user","content":"Can you be more sarcastic?"},{"role":"assistant","content":"Paris, as if everyone doesn't know that already.","weight":1}]}
```

Field by field:

| Field | Where | Required | Notes |
|---|---|---|---|
| `messages` | top level | yes | array of message objects |
| `tools` | top level | no | array of JSON-schema function definitions |
| `parallel_tool_calls` | top level | no | boolean |
| `role` | message | yes | see the role disagreement below |
| `content` | message | yes for text messages | string; may be empty only when a tool call is present |
| `weight` | assistant message | no | `0` or `1`. `0` excludes that assistant turn from the loss |
| `tool_calls` | assistant message | no | replaces `content` |
| `tool_call_id` | tool message | yes on `role: "tool"` | must match the id in the assistant's `tool_calls` |
| `name` | message | no | accepted by the validator |

`weight` is documented at
<https://developers.openai.com/api/docs/guides/fine-tuning-best-practices>: "Use the optional
`weight` key (value set to either 0 or 1) to disable fine-tuning on specific assistant messages."
That one field is the reason this format can carry a whole agent trajectory as a single training
line and still control which assistant turns contribute. Nothing in the TRL side has an equivalent
per-message switch.

Minimum file size is 10 examples, stated on the SFT guide page.

**The roles disagree with themselves.** The guide's prose talks about `user` and `assistant`. The
validator OpenAI publishes in its own cookbook accepts four roles and five message keys:

```python
if message.get("role", None) not in ("system", "user", "assistant", "function")
if any(k not in ("role", "content", "name", "function_call", "weight") for k in message)
```

Source: <https://developers.openai.com/cookbook/examples/chat_finetuning_data_prep>. That validator
is stale relative to the tools API. It still lists the deprecated `function_call` and `function`
and has no `tool_calls` or `tool` in it. Treat it as evidence of what the parser once accepted,
not as the current contract.

### Tool calls in this format

Source: <https://learn.microsoft.com/en-us/azure/foundry/openai/how-to/fine-tuning-functions>
(Microsoft's docs for the same file format, and the only place either vendor spells out a full
call-plus-result cycle in a training file).

```jsonl
{"messages":[{"role":"user","content":"What is the weather in San Francisco?"},{"role":"assistant","tool_calls":[{"id":"call_id","type":"function","function":{"name":"get_current_weather","arguments":"{\"location\": \"San Francisco, USA\", \"format\": \"celsius\"}"}}]},{"role":"tool","tool_call_id":"call_id","content":"21.0"},{"role":"assistant","content":"It is 21 degrees celsius in San Francisco, CA"}],"tools":[]}
```

Two details that bite:

- `function.arguments` is a **JSON string**, not a JSON object. Double-encoded. Every byte inside
  it is invisible to any redactor that walks the JSON tree instead of the string values.
- Tool results are plain `content` strings on a `role: "tool"` message keyed by `tool_call_id`.
  There is no place for a structured result, no exit code field, no truncation marker. A 400 KB
  file read comes back as one string, or it does not come back.

Microsoft's page also documents the deprecated `functions` / `function_call` / `role: "function"`
variant on the same page and says outright: "`function_call` and `functions` have been deprecated
in favor of `tools`." Datasets in the wild still use it.

## SFT, format two: TRL

Source: <https://huggingface.co/docs/trl/en/dataset_formats>

TRL splits format (standard versus conversational) from type (language modeling, prompt-only,
prompt-completion, preference, unpaired preference, stepwise). `SFTTrainer` accepts language
modeling or prompt-completion:

```python
# Language modeling, conversational
{"messages": [{"role": "user", "content": "What color is the sky?"},
              {"role": "assistant", "content": "It is blue."}]}

# Prompt-completion, conversational
{"prompt": [{"role": "user", "content": "What color is the sky?"}],
 "completion": [{"role": "assistant", "content": "It is blue."}]}
```

The `messages` shape is byte-compatible with OpenAI's for plain text. It diverges on tool calls:

```python
messages = [
    {"role": "user", "content": "Turn on the living room lights."},
    {"role": "assistant", "tool_calls": [
        {"type": "function", "function": {
            "name": "control_light",
            "arguments": {"room": "living room", "state": "on"}
        }}]
    },
    {"role": "tool", "name": "control_light", "content": "The lights in the living room are now on."},
    {"role": "assistant", "content": "Done!"}
]
```

Differences from OpenAI, all of them real:

- `arguments` is a **dict**, not a JSON string. Opposite of OpenAI.
- No `id` on the tool call. TRL's example omits it.
- The tool message is keyed by `name`, not `tool_call_id`. With parallel tool calls that
  correspondence is ambiguous, and the docs do not address it.
- A `tools` column is required alongside `messages` when tool calling is in play, holding JSON
  schemas: `{"messages": messages, "tools": [json_schema]}`. TRL's docs say this is needed "for
  Supervised Fine-Tuning (SFT) with tool calling", and the DPO page repeats the requirement for
  preference data.
- No `weight`. Which assistant turns train is decided by the trainer and the chat template, not
  per message in the data.

TRL also documents the Harmony format for GPT-OSS models, which adds a `developer` role, channels
(`analysis` / `commentary` / `final`), and a `"thinking"` key on assistant messages that maps to
the analysis channel. Same page, "Harmony" section, pointing at
<https://cookbook.openai.com/articles/openai-harmony>. This is the only mainstream SFT format with
a documented home for reasoning traces, which is directly relevant to ticket 09's `thinking` event.

## SFT, format three: ShareGPT

Still alive, and the ecosystem is actively split on it.

**LLaMA-Factory treats it as first class**, including tool traffic. Source:
<https://github.com/hiyouga/LLaMA-Factory/blob/main/data/README.md>

```json
{
  "conversations": [
    {"from": "human", "value": "user instruction"},
    {"from": "function_call", "value": "tool arguments"},
    {"from": "observation", "value": "tool result"},
    {"from": "gpt", "value": "model response"}
  ],
  "system": "system prompt (optional)",
  "tools": "tool description (optional)"
}
```

Tool calls are a **role**, not a field. `function_call` and `observation` are turns in the
conversation with string values. The tag names are configurable per dataset in `dataset_info.json`
via `user_tag`, `assistant_tag`, `observation_tag`, `function_tag`, `system_tag`. Note `tools` here
is a string, not an array.

**Axolotl deprecated it.** Source:
<https://docs.axolotl.ai/docs/dataset-formats/conversation.html>, which says "ShareGPT is
deprecated! Please see chat_template section." Axolotl's `chat_template` path wants OpenAI-shaped
messages with `tool_calls`, `tool_call_id`, and a `tools` array.

**Unsloth converts it.** Source:
<https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/datasets-guide> documents the
`from`/`value` shape with `human`/`gpt` alternation, then `standardize_sharegpt(dataset)` to move
it to `role`/`content`, and calls ChatML "probably the most used format".

So: ShareGPT is a legacy format that one major trainer still takes natively with tool support, one
has deprecated, and one auto-converts. Its tool representation is incompatible with both of the
others, and lossy in the same direction as OpenAI's (everything flattened to strings) but worse,
since call and result are not linked by any id at all.

## Preference pairs, shape one: TRL

Source: <https://huggingface.co/docs/trl/en/dataset_formats> and
<https://huggingface.co/docs/trl/en/dpo_trainer>

Two sub-shapes, and TRL has an opinion on which:

```python
# Explicit prompt (recommended)
{"prompt": [{"role": "user", "content": "What color is the sky?"}],
 "chosen": [{"role": "assistant", "content": "It is blue."}],
 "rejected": [{"role": "assistant", "content": "It is green."}]}

# Implicit prompt
{"chosen": [{"role": "user", "content": "What color is the sky?"},
            {"role": "assistant", "content": "It is blue."}],
 "rejected": [{"role": "user", "content": "What color is the sky?"},
              {"role": "assistant", "content": "It is green."}]}
```

"We recommend using explicit prompts whenever possible." The trainer table on that page then
contradicts the recommendation per trainer: `DPOTrainer` wants explicit, `RewardTrainer` wants
implicit. Both are the same file with the prompt moved.

Required: `chosen` and `rejected`. Optional: `prompt` (implicit form), `tools`.

The `prompt` is a full message list, so it can carry arbitrarily many prior turns including tool
calls and tool results. The DPO page states this directly: "each dataset example should include
the conversation messages (prompt, chosen and rejected), including any tool calls (`tool_calls`)
and tool responses (`tool` role messages)" plus "the list of available tools in the `tools` column".
For an agent trajectory this is the only mainstream preference format that admits the thing exists.

## Preference pairs, shape two: OpenAI

Source: <https://developers.openai.com/api/docs/guides/direct-preference-optimization>

```json
{
  "input": {
    "messages": [{"role": "user", "content": "..."}],
    "tools": [],
    "parallel_tool_calls": true
  },
  "preferred_output": [{"role": "assistant", "content": "..."}],
  "non_preferred_output": [{"role": "assistant", "content": "..."}]
}
```

Different key names, different nesting, and three documented restrictions:

- Single-turn conversations only per example.
- "The preferred and non-preferred messages need to be the last assistant message."
- Text only.

That last-message rule is the load-bearing one. A `correction` event that rejects a tool call in
the middle of a 40-step run cannot be expressed as-is. You would have to synthesize an example
whose `input.messages` is the trajectory up to that point and whose two outputs are the rejected
and chosen tool call, which is a legal transformation but is a different thing from what TRL's
format lets you write directly.

## Preference pairs, shape three: ShareGPT

Source: <https://github.com/hiyouga/LLaMA-Factory/blob/main/data/README.md>

```json
{
  "conversations": [
    {"from": "human", "value": "user instruction"},
    {"from": "gpt", "value": "model response"},
    {"from": "human", "value": "user instruction"}
  ],
  "chosen": {"from": "gpt", "value": "chosen answer (required)"},
  "rejected": {"from": "gpt", "value": "rejected answer (required)"}
}
```

`chosen` and `rejected` are **single message objects**, not lists. So a preference pair here is
exactly one assistant turn, and the multi-turn context lives in `conversations`. Same structural
constraint as OpenAI, reached by a different route. LLaMA-Factory also documents an Alpaca-shaped
preference format (`instruction` / `input` / `chosen` / `rejected`) on the same page.

## Where the formats disagree, in one table

| Question | OpenAI | TRL | ShareGPT (LLaMA-Factory) |
|---|---|---|---|
| Message list key | `messages` | `messages`, or `prompt`+`completion` | `conversations` |
| Message keys | `role` / `content` | `role` / `content` | `from` / `value` |
| Assistant role name | `assistant` | `assistant` | `gpt` |
| Tool call | `tool_calls[]` on assistant | `tool_calls[]` on assistant | a turn with `from: "function_call"` |
| Tool call arguments | JSON **string** | **dict** | string |
| Tool call id | `id`, required to link the result | not shown in the docs | none |
| Tool result | `role: "tool"` + `tool_call_id` | `role: "tool"` + `name` | a turn with `from: "observation"` |
| Tool schemas | top-level `tools` array | `tools` **column**, array of schemas | `tools` **string** |
| Per-message loss control | `weight` 0/1 | none | none |
| Reasoning trace | Harmony `thinking` key on GPT-OSS only | Harmony `thinking` key | none |
| Preference keys | `input`/`preferred_output`/`non_preferred_output` | `prompt`/`chosen`/`rejected` | `conversations`+`chosen`/`rejected` |
| Preference multi-turn | no, last assistant message only | yes, full message lists | context yes, pair is one message |
| Preference with tools | text only, explicitly | yes, documented | undocumented |

## What this constrains, without deciding it

Facts that fall out, for the human to weigh:

1. **One pass can produce SFT and DPO only if the pair shape is TRL's.** OpenAI's DPO file needs
   `input.messages` rebuilt as a prefix per correction, which is a second traversal of the same
   trajectory with a different cut point. TRL's explicit-prompt preference needs the same prefix.
   The difference is whether the SFT line and the DPO line can share a serialization of the prefix,
   and with TRL they can, because both sides are the same message list type.
2. **The arguments encoding is a fork in the road, not a detail.** OpenAI wants
   `"arguments": "{\"path\": \"/Users/USER/...\"}"`, TRL wants `"arguments": {"path": "..."}`.
   A redactor that walks JSON values will miss every path, key and secret inside the OpenAI form
   unless it parses the nested string too. This is worth stating in the ticket because it is a
   redaction bug waiting to happen, not just a serialization preference.
3. **Ticket 09's `thinking` event has one documented home.** Harmony's `"thinking"` key, which TRL
   supports for GPT-OSS. Everywhere else it either goes in `content` or it is dropped.
4. **The `tool_result` truncation problem is unrepresentable.** Every format flattens a tool result
   to one string. If Hydra records that a result was truncated, the flag has to be encoded inside
   the string or thrown away.
5. **Cursor's `usage_source: "estimated"` never reaches these files.** None of the three formats
   carries token counts. So 09's provenance guarantee holds at export by construction, provided
   the exporter does not invent a metadata sidecar that leaks estimates as facts.

---

# Part 2. Redaction

## gitleaks

Source: <https://github.com/gitleaks/gitleaks>, config
<https://github.com/gitleaks/gitleaks/blob/master/config/gitleaks.toml>. Latest release v8.30.1
(2026-03-21).

**How rules are expressed.** TOML, one `[[rules]]` table per rule:

```toml
[[rules]]
id = "awesome-rule-1"
description = "awesome rule 1"
regex = '''one-go-style-regex-for-this-rule'''
secretGroup = 3
entropy = 3.5
path = '''a-file-path-regex'''
keywords = ["auth", "password", "token"]
tags = ["tag","another tag"]
```

`regex` is Go's RE2, so no lookaheads. `entropy` is a minimum Shannon entropy on the captured
group. `keywords` are a cheap pre-filter, a literal substring check before the regex runs.
`path` matches file paths and can stand alone as a rule. Allowlists (`[[rules.allowlists]]` and
global `[[allowlists]]`) suppress findings by commit, path, regex against secret / match / line,
or stopword against the extracted secret, with `condition = "OR"` or `"AND"`. Since v8.28.0 there
are composite rules: a primary rule plus `[[rules.required]]` auxiliaries with `withinLines` and
`withinColumns` proximity constraints.

**How many rules.** I counted the default config at `master`: **222 `[[rules]]` entries**, of which
**130 set an `entropy` threshold** and 5 use a `path` pattern. Zero composite rules in the shipped
config so far. Note that agent-sweep's comparison table says gitleaks has "~150" rules; that is
out of date, and the file is the authority.

The generic catch-all rule is worth reading in full because it defines the boundary of what regex
gets you:

```
id = "generic-api-key"
regex = '''(?i)[\w.-]{0,50}?(?:access|auth|(?-i:[Aa]pi|API)|credential|creds|key|passw(?:or)?d|secret|token)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([\w.=-]{10,150}|[a-z0-9][a-z0-9+/]{11,}={0,3})(?:[\x60'"\s;]|\\[nr]|$)'''
entropy = 3.5
```

It requires an assignment-looking context. `KEY = <high entropy blob>`. A bare high-entropy string
in the middle of prose does not match anything.

**Verification.** None. gitleaks never calls out to check whether a key is live.

**Published false-negative behaviour.** gitleaks publishes no recall figure. The engine blog the
README links, <https://lookingatcomputer.substack.com/p/regex-is-almost-all-you-need>, concedes
the limit in the author's own words: "it would be difficult to craft a regular expression that
could correctly label `MyServiceToken="secret123"` as a false positive and
`MyServiceToken="8dyfuiRyq=vVc3RRr_edRk-fK__JItpZ"` as a true positive", and "Entropy by itself
isn't a very good heuristic for determining whether something is a secret or not." The piece ends
on "Okay, so you might need a little more than regular expressions."

**Two flags that matter for an export pipeline.** `gitleaks stdin` reads from a pipe, so JSONL can
be streamed through it without a repo. And `--max-decode-depth` (default `0`, off) recursively
decodes base64 and friends before matching, tagging findings `decoded:<encoding>` and
`decode-depth:<depth>`. Off by default means a base64-wrapped key in a tool result is invisible
unless you turn it on.

**gitleaks does not redact anything.** `--redact` masks secrets **in gitleaks' own report output**.
Read the flag help: "redact secrets from logs and stdout". It never rewrites the scanned content.

## trufflehog

Source: <https://github.com/trufflesecurity/trufflehog>. Latest release v3.97.1 (2026-08-24).

**How detectors are expressed.** Go code, not config. Each detector is a package with a regex plus
a `Verify` method that calls the real service. The README: "For every potential credential that is
detected, we've painstakingly implemented programmatic verification against the API that we think
it belongs to." The AWS detector calls `GetCallerIdentity`.

**How many.** The README says "over 800 secret types" and "over 700 credential detectors that
support active verification". I counted directory entries under `pkg/detectors` at `main`: **884**.
Plus six decoders under `pkg/decoders` (base64, escaped unicode, html, utf16, utf8), which run
before detection.

**Three result states**, quoted from the README:

- **verified**: "Credential confirmed as valid and active by API testing"
- **unverified**: "Credential detected but not confirmed valid (may be invalid, expired, or
  verification disabled)"
- **unknown**: "Verification attempted but failed due to errors, such as a network or API failure"

Default output is `verified,unverified,unknown`. `--results=verified` narrows to live keys.
`--filter-entropy` filters unverified results by Shannon entropy, with the help text suggesting
"Start with 3.0".

**Custom detectors** are YAML, alpha, and need "at least one regular expression and keyword".
Verification is a webhook: trufflehog POSTs the matches, and a `200 OK` marks the secret verified.
The `verify` section is optional, and the README explicitly points at the case Hydra is in: without
it, "matches are still reported as unverified, no webhook required. This makes Custom Detectors
useful for flagging generic hardcoded secrets (e.g. `*.password=`, `*.secret=`) in config files
like `.properties`, `.env`, or `.yaml` that TruffleHog's built-in, verified detectors won't
otherwise catch."

**Published false-negative behaviour.** The README claims "Verification eliminates false positives"
and says nothing about false negatives. Which is the honest shape of the tradeoff: verification is
a precision mechanism, and it costs recall, because every expired, rotated, or offline-service
credential lands in `unverified` and disappears the moment anyone runs `--results=verified`.

**trufflehog does not redact anything either.** Its JSON output has a `Redacted` field, which is
the finding as displayed. Content is never rewritten.

## The one independent measurement I found

Basak, Cox, Reaves and Williams, "A Comparative Study of Software Secrets Reporting by Secret
Detection Tools", ESEM 2023. <https://arxiv.org/abs/2307.00714>

Nine tools against a benchmark dataset. Precision: GitHub Secret Scanner 75%, Gitleaks 46%,
Commercial X 25%. Recall: **Gitleaks 88%**, SpectralOps 67%, **TruffleHog 52%**. The paper's own
diagnosis, verbatim: "false positives are due to employing generic regular expressions and
ineffective entropy calculation. In contrast, false negatives are due to faulty regular
expressions, skipping specific file types, and insufficient rulesets."

Caveats I will not paper over. It is a 2023 measurement against source repositories with
checked-in credentials, not agent transcripts. Both tools have moved since (gitleaks' default
config alone is 222 rules now). And the paper does not state which trufflehog result modes it
counted, which matters a lot given the verified/unverified split. Use the numbers as an order of
magnitude, not a spec: the best open tool misses roughly one secret in eight on the easy case.

## What is reliably catchable by pattern

These have a fixed prefix, a fixed length, or a checksum. Regex plus entropy gets them at high
precision and high recall, and a test suite over them is meaningful.

- Provider API keys with a vendor prefix: `AKIA...`, `ghp_`, `sk-ant-`, `sk-`, `xoxb-`, `glpat-`,
  `p8e-`, `ops_eyJ`, and the other 200-odd in gitleaks' pack.
- PEM blocks. `-----BEGIN ... PRIVATE KEY-----` is unambiguous.
- JWTs. Three base64 segments with a decodable header.
- Connection strings with embedded credentials, `postgres://user:pass@host`.
- BIP-39 seed phrases, which are checksum-validated rather than guessed (agent-sweep does this).
- Hydra's own credentials from its auth config, which is ticket 09's write-time filter. This is the
  easiest class of all, because it is literal-string matching against known values rather than
  pattern matching against unknown ones, and it is the only class where "did the redactor work" has
  a yes-or-no answer.

## What is not catchable by pattern

Blunt, as asked. These are the ticket's real question and none of them is a regex problem.

**Absolute paths carrying a username.** `/Users/USER/hydra/...` is trivially matchable *when you
already know the username*, which you do, since Hydra runs as that user. `$HOME` is knowable at
export time. So this one is genuinely mechanical, and the right tool is literal replacement of the
known home directory prefix, not a rule. What is not mechanical is the tail: `/Users/USER/clients/
acme-corp/...` leaks a client name after the path prefix is gone, and no pattern tells you that
`acme-corp` is a customer rather than an open source dependency.

**Email addresses.** Matchable as a shape, RFC-ish regex, easy. Deciding *which ones must go* is
not. `noreply@anthropic.com` in a commit trailer is fine. A colleague's address in a git blame is
probably not. `franciscomdzah@gmail.com` in a commit message is the user's own and arguably fine
in their own corpus. A pattern cannot rank them. This is a policy question dressed as a detection
question, and the only defensible mechanical answers are "redact all of them" or "redact none".

**Repository and project names.** Not matchable at all. `hydra` is a word. So is `ouroboros`. A
private repo name is a string with no distinguishing shape, and the only reliable signal is an
allowlist of the public repos, which inverts the problem into maintaining a list.

**The contents of a file the agent read.** This is the largest class by volume and the least
tractable. Ticket 09 chose to store raw frames, so `tool_result` events contain whole files. Inside
one file there might be a customer name, a hardcoded internal hostname, an unreleased product
codename, a comment naming a person, or a licence header from vendored proprietary code. None of
these has a pattern. Worse, the aggregate is the leak even when no single line is: a trajectory
that reads twelve files reveals the shape of a private codebase regardless of whether any one
string is sensitive. There is no redactor, model-based or otherwise, that fixes that. The only
mechanisms that work on it are exclusion (do not export runs that touched these paths) and
allowlisting (export only runs whose file reads stayed inside declared-public repos).

**Customer data in a fixture.** A fixture is *designed* to look like real data, which is exactly
why the pattern tools fail on it. A name, an address, an order id, a phone number. Presidio and
similar will flag the *shape* (PERSON, PHONE_NUMBER) with no idea whether the value is synthetic
or real. And in the direction that matters, a fixture full of realistic synthetic customers
produces hundreds of hits, all of them false positives, which trains whoever reviews the report to
stop reading it.

**Secrets split across messages.** agent-sweep names this in its "What's NOT detected" list. An
agent that echoes half a key, then the other half after a tool call, defeats every line-oriented
scanner in both tools. So does an unprefixed proprietary token that looks like base64.

## On the model pass, without recommending one

What a model pass buys, factually: it is the only mechanism that can answer "is this string a
customer name" or "does this file content belong to a private client". Presidio, the established
non-model baseline, combines regex, checksums, deny lists, context words and NER
(<https://presidio.dataprivacystack.org/>), and ships this disclaimer verbatim: "Presidio can help
identify sensitive/PII data in un/structured text. However, because it is using automated detection
mechanisms, there is no guarantee that Presidio will find all sensitive information." That sentence
is the honest ceiling for the whole non-model category, and a model pass does not remove it, it
moves it. A model also has no recall guarantee, cannot be unit tested the way a regex can, and
costs a pass over a corpus whose whole point is being large.

The testable part and the untestable part do not mix. A rule engine over the first list can have a
fixture suite with known-answer cases and a hard fail. A model pass over the second list cannot,
and any test you write for it measures the fixtures, not the model. Ticket 19's "what proves it
worked" question probably has different answers for the two halves, and that is the shape of the
decision rather than a decision I am making here.

---

# Is there an existing tool that redacts an agent trajectory?

Yes. One, and it is close enough to Hydra's problem to be worth reading rather than reinventing.

**agent-sweep** (`agentsweep` on PyPI), <https://github.com/Ishannaik/agent-sweep>. MIT, Python,
76 stars, created 2026-06-11, last pushed 2026-08-31. It scans local AI coding agent history and,
unlike gitleaks and trufflehog, **rewrites it in place while preserving the JSONL structure**. It
covers 31 agents including Claude Code, Cursor, Gemini CLI and Hermes, which is three of Hydra's
four providers. 207 regex rules, 187 of them mapped to the gitleaks pack, plus a checksum-validated
BIP-39 detector. The pipeline is discover, Aho-Corasick pre-filter, regex, report, redact, with
atomic writes, mandatory `.bak` backups, symlink rejection, an mtime gate, a running-process gate,
post-write JSONL validation, and an `undo`. It emits SARIF and a JSON report, exits 1 on findings,
and makes zero network calls.

Its own "What's NOT detected" list is the honest part, and it matches my analysis above:
custom/proprietary secrets without a prefix, unknown base64-looking tokens, secrets split across
messages, anything in a non-UTF-8 file. It says outright: "For deeper detection, run `gitleaks` or
`trufflehog` alongside agentsweep, since their rule packs are more exhaustive."

Two adjacent things, both narrower:

- **claude-code-redaction-hooks**, <https://github.com/l-mb/claude-code-redaction-hooks>. Apache-2.0,
  20 stars, last pushed 2026-05-11. Claude Code hooks that block or mask secrets at tool boundaries
  before they reach the transcript. This is the write-time-filter shape, not the export shape.
- **claude-replay**, <https://github.com/es617/claude-replay>. MIT, 822 stars, actively maintained.
  Renders agent sessions to shareable HTML with pattern-based secret redaction on by default, and
  says in its own docs that pattern-based redaction is best-effort and the output should be
  reviewed before publishing.

What does **not** exist, as far as I can find: any tool that redacts an agent trajectory *for
training-corpus export*. Everything above targets "make this session safe to leave on disk" or
"safe to paste in a GitHub issue". Nobody publishes a redactor whose contract is "this file is safe
to fine-tune on", and nobody publishes a test suite for that claim. If ticket 19 wants one, it is
being built here, and agent-sweep's safety invariants and rule pack are the closest prior art to
borrow from.

---

# Sources

Formats:

- <https://developers.openai.com/api/docs/guides/supervised-fine-tuning>
- <https://developers.openai.com/api/docs/guides/fine-tuning-best-practices>
- <https://developers.openai.com/api/docs/guides/direct-preference-optimization>
- <https://developers.openai.com/cookbook/examples/chat_finetuning_data_prep>
- <https://learn.microsoft.com/en-us/azure/foundry/openai/how-to/fine-tuning-functions>
- <https://huggingface.co/docs/trl/en/dataset_formats>
- <https://huggingface.co/docs/trl/en/dpo_trainer>
- <https://cookbook.openai.com/articles/openai-harmony>
- <https://docs.axolotl.ai/docs/dataset-formats/conversation.html>
- <https://github.com/hiyouga/LLaMA-Factory/blob/main/data/README.md>
- <https://unsloth.ai/docs/get-started/fine-tuning-llms-guide/datasets-guide>

Redaction:

- <https://github.com/gitleaks/gitleaks>
- <https://github.com/gitleaks/gitleaks/blob/master/config/gitleaks.toml>
- <https://lookingatcomputer.substack.com/p/regex-is-almost-all-you-need>
- <https://github.com/trufflesecurity/trufflehog>
- <https://arxiv.org/abs/2307.00714>
- <https://presidio.dataprivacystack.org/>
- <https://github.com/Ishannaik/agent-sweep>
- <https://github.com/l-mb/claude-code-redaction-hooks>
- <https://github.com/es617/claude-replay>
