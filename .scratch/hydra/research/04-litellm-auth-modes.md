# LiteLLM auth modes: virtual keys, upstream credentials, subscriptions

Researched 2026-09-01. Pinned to LiteLLM v1.98.0 (released 2026-08-23, the latest
stable tag with published release notes at research time). v1.99.0 shipped the same
day as this research (2026-09-01) and is noted where it changes the picture.
LiteLLM is not installed on this machine; every claim below traces to
docs.litellm.ai, github.com/BerriAI/litellm, or code.claude.com.

## Inbound: virtual keys, and whether OAuth/SSO gates the proxy

A virtual key is created with `POST /key/generate` against a running proxy,
authenticated with the proxy's `master_key`:

```
curl 'http://0.0.0.0:4000/key/generate' \
  --header 'Authorization: Bearer <your-master-key>' \
  --header 'Content-Type: application/json' \
  --data-raw '{"models": ["gpt-3.5-turbo", "gpt-4"], "metadata": {"user": "ishaan@berri.ai"}}'
```

(docs.litellm.ai/docs/proxy/virtual_keys). The same page documents the UI and a
teams flow as the other two creation paths.

Scoping fields on that call, per the same page:

- `models`: an allowlist of model names the key may call. Anything not on the list is refused.
- `max_budget` / `budget_duration`: a spend ceiling in USD and the window it resets on (e.g. `"10d"`).
- `tpm_limit` / `rpm_limit`: tokens-per-minute and requests-per-minute caps.
- `max_parallel_requests`: a concurrency cap.
- `duration`: relative expiry on the key itself, e.g. `"30min"` or `"30d"`.
- `team_id` / `user_id`: attaches the key to a team or user, which the key can then inherit budget and rate limits from unless it overrides them. Model access itself is evaluated on the key's own row, not inherited from the team, unless the key is created with `"all-team-models"`.

Running this needs Postgres (`DATABASE_URL`) and a `general_settings.master_key`
in config.yaml. This is the full answer to D6: Hydra holds one virtual key scoped
to whichever models it's allowed to call, with its own budget, rate limit, and
expiry, and LiteLLM holds the four provider secrets behind it. Hydra's key never
needs to know an upstream's shape.

On OAuth/SSO gating the proxy itself: LiteLLM's SSO integration
(docs.litellm.ai/docs/proxy/admin_ui_sso) only gates the admin UI login, not API
calls. The page title is literally "SSO for Admin UI," and the fallback path it
documents (`/fallback/login`) is a UI login screen, not an API auth mode. A
request to `/chat/completions` or any other proxy route still needs a virtual
key.

There is a separate mechanism that does gate API calls with something other than
a static key: OIDC/JWT auth (docs.litellm.ai/docs/proxy/token_auth). Set
`general_settings.enable_jwt_auth: true` and point `JWT_PUBLIC_KEY_URL` at your
identity provider's JWKS endpoint, and the proxy validates a bearer JWT per
request instead of (or alongside, via JWT-to-virtual-key mapping) a virtual key,
with role-based model access via `user_roles_jwt_field` and `role_permissions`.
The docs mark this "JWT-based Auth is on LiteLLM Enterprise" and gate it behind a
sales contact for production use, so it's not available on the open-source
proxy Hydra would self-host. For Hydra's single-operator case this doesn't
matter: a virtual key is the right shape either way.

## Outbound: which upstreams accept more than a plain API key

**Vertex AI, confirmed.** LiteLLM authenticates to Vertex three ways
(docs.litellm.ai/docs/providers/vertex):

```yaml
model_list:
  - model_name: gemini-2.5-pro
    litellm_params:
      model: vertex_ai/gemini-2.5-pro
      vertex_project: "your-project-id"
      vertex_location: "us-central1"
      vertex_credentials: "/path/to/service_account.json"
```

`vertex_credentials` takes a service account JSON file path or the JSON itself
inline. Omit it and LiteLLM falls back to Application Default Credentials
(`gcloud auth application-default login`, or `GOOGLE_APPLICATION_CREDENTIALS` in
the environment). For non-GCP hosts there's also Workload Identity Federation,
which impersonates a service account without a long-lived key file. The service
account needs the Vertex AI User IAM role. None of this is a plain API key.

**Bedrock, confirmed, and the strongest case of the two.** LiteLLM authenticates
through boto3, so it inherits boto3's full credential chain
(docs.litellm.ai/docs/providers/bedrock):

```yaml
model_list:
  - model_name: bedrock-cross-account
    litellm_params:
      model: bedrock/anthropic.claude-instant-v1
      aws_role_name: arn:aws:iam::123456789012:role/BedrockRole
      aws_session_name: bedrock-session
```

`aws_role_name` does an STS AssumeRole. `aws_profile_name` reads a named profile
from `~/.aws/config`, SSO profiles included. `aws_web_identity_token` covers
OIDC federation (EKS IRSA). Because this is boto3's standard chain, if the
LiteLLM process itself runs on AWS compute with an attached IAM role (EC2, ECS,
Lambda), it needs no credentials in config at all, the instance role satisfies
every request. That's IAM in the fullest sense of the term, no static secret
anywhere. A newer bearer-token mode (`AWS_BEARER_TOKEN_BEDROCK`) also exists for
cases that want a single rotating token instead.

## The subscription question: does a Claude subscription route through the proxy

Short answer: LiteLLM built real support for this, but it only carries a
subscription through when the actual, unmodified Claude Code binary is the
client, and Anthropic's own terms say routing subscription credentials through
anything else is a violation they can and do enforce. This is a terms limit,
not a LiteLLM code gap, and it doesn't give Hydra a shared credential the way an
API-key upstream does.

**What LiteLLM built.** A merged PR
(github.com/BerriAI/litellm/pull/19453, merged 2026-01-21, closing
github.com/BerriAI/litellm/issues/13380) added detection for Anthropic OAuth
tokens (the `sk-ant-oat01-` prefix) arriving in the `Authorization` header. When
one shows up, LiteLLM authenticates the caller against LiteLLM's own budgets and
limits using a separate `x-litellm-api-key` header, and forwards the OAuth
token itself untouched to Anthropic, along with the OAuth-specific headers
Anthropic expects (`anthropic-beta`,
`anthropic-dangerous-direct-browser-access`). This is documented today at
docs.litellm.ai/docs/tutorials/claude_code_max_subscription under
`general_settings.forward_client_headers_to_llm_api: true`, with no
deprecation notice on the page. v1.99.0 (2026-09-01) adds a `lite login
--config-claude` command that wires this up at login, so it's an actively
maintained path, not an abandoned one.

The important limit: this only works because the real Claude Code CLI performs
its own OAuth handshake with Anthropic and then points its `base_url` at the
LiteLLM proxy. LiteLLM never mints or exchanges subscription access on anyone's
behalf, it relays a token someone else already holds. A generic caller of
LiteLLM's OpenAI-shaped endpoint, holding only a LiteLLM virtual key, gets
nothing from this path unless they separately run and log into their own Claude
Code session. That's the opposite of what a virtual key does for an API-key
upstream, where the key itself is the credential. So there's no way to make one
shared virtual key stand in for a Claude subscription the way D6 wants it to
stand in for a provider API key.

**Why, per Anthropic's own terms.** Anthropic's Claude Code legal page
(code.claude.com/docs/en/legal-and-compliance, redirected from
docs.anthropic.com, fetched 2026-09-01) states OAuth authentication "is intended
exclusively for purchasers of Claude Free, Pro, Max, Team, and Enterprise
subscription plans and is designed to support ordinary use of Claude Code and
other native Anthropic applications." It goes on: developers building any
product or service that touches Claude, "including those using the Agent SDK,
should use API key authentication," and "Anthropic does not permit third-party
developers to... route requests through Free, Pro, or Max plan credentials on
behalf of their users," or to "collect, store, or intermediate Claude.ai
credentials or session tokens." Anthropic "reserves the right to take measures
to enforce these restrictions and may do so without prior notice." The same
page carves out the case that matters for LiteLLM's tutorial: it does not
restrict "an end user from signing in to the unmodified Claude Code binary with
their own Claude subscription," including through a hosted platform, as long as
the binary itself isn't modified and usage isn't resold or intermediated to
other people.

That carve-out is exactly why LiteLLM's tutorial can exist without contradicting
the policy: it's framed for a company that wants visibility into its own
employees' Claude Code Max usage, where each employee still signs into the real
binary with their own subscription. It is not a mechanism for turning a
subscription into a shared credential behind a virtual key, which is the
specific thing the terms forbid.

**Enforcement is real, not theoretical.** Anthropic cut off Claude Pro/Max
subscription access for third-party agentic frameworks on 2026-04-04, starting
with OpenClaw, announced by Boris Cherny (Anthropic, head of Claude Code) with
under a day's notice (reported by VentureBeat and others). The Register's
2026-02-20 piece quotes Anthropic's rationale: third-party harnesses "generate
unusual traffic patterns without any of the usual telemetry that the Claude
Code harness provides, making it really hard for us to help debug." Anthropic
had signaled this in its terms since at least 2024; 2026 is when it started
acting on it.

**Verdict.** No supported path lets Hydra hold one shared credential that grants
subscription-priced Claude access to arbitrary callers. The only working
pattern requires the genuine Claude Code binary and a personal OAuth session per
user, which is precisely the shape D4 already assigns to the agent-CLI plane
(subprocess, per-provider OAuth, subscription pricing). D6's "one virtual key,
never four provider secrets" should be read as covering the API-key and
cloud-IAM upstreams only. Subscription-priced Claude access stays out of
LiteLLM entirely and lives in the agent-CLI plane, exactly where the map already
put it.

**Hermes confirms the general pattern, checked live on this machine
(2026-09-01):**

```
$ hermes proxy providers
Available proxy upstream providers:
  nous  — Nous Portal
  xai  — xAI Grok OAuth
```

Unchanged from the ticket's claim: neither Anthropic nor Google. LiteLLM's
Anthropic OAuth support required a dedicated patch that sniffs Anthropic's
specific token prefix; nothing about it is generic. The equivalent problem for
OpenAI's ChatGPT subscription is an open, unresolved bug in LiteLLM
(github.com/BerriAI/litellm/issues/27175, "Cannot use ChatGPT subscription
successfully"), and there's no documented path for a Google consumer
subscription at all. Subscription OAuth proxying is provider-specific,
narrow even where a provider bothers to build it, and never a generic
capability of the proxy layer, on hermes or on LiteLLM.

## Sources

- docs.litellm.ai/docs/proxy/virtual_keys
- docs.litellm.ai/docs/proxy/admin_ui_sso
- docs.litellm.ai/docs/proxy/token_auth
- docs.litellm.ai/docs/providers/vertex
- docs.litellm.ai/docs/providers/bedrock
- docs.litellm.ai/docs/proxy/forward_client_headers
- docs.litellm.ai/docs/tutorials/claude_code_max_subscription
- github.com/BerriAI/litellm/issues/13380
- github.com/BerriAI/litellm/pull/19453 (merged 2026-01-21)
- github.com/BerriAI/litellm/pull/14821 (merged 2025-09-24)
- github.com/BerriAI/litellm/issues/27175
- github.com/BerriAI/litellm/discussions/20072
- github.com/BerriAI/litellm/releases/tag/v1.98.0 (published 2026-08-23)
- github.com/BerriAI/litellm/releases/tag/v1.99.0 (published 2026-09-01)
- code.claude.com/docs/en/legal-and-compliance (Anthropic first-party, fetched 2026-09-01)
- theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access
- `hermes proxy providers`, run locally 2026-09-01
