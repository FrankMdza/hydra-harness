# 04 - LiteLLM: virtual keys and what it will accept upstream

Type: research
Status: resolved

## Question

The raw-model plane rests on LiteLLM. Establish what it can actually authenticate, in both directions.

- **Inbound.** Virtual keys, and whether OAuth or SSO gates who may call the proxy. This is the part that satisfies D6: Hydra holds one virtual key, never four provider secrets.
- **Outbound.** Which upstreams accept something other than a plain API key. Vertex AI with a service account is the interesting case for the Google side, Bedrock IAM likewise. Confirm or kill both.
- **The subscription question.** Whether any supported path routes a Claude subscription through the proxy, and if the answer is no, whether that is a technical limit or a terms limit.

Context: `hermes proxy` already does this shape of thing, but `hermes proxy providers` lists only Nous Portal and xAI Grok, so it covers neither Anthropic nor Google. Confirm that has not changed.

A clear negative is the useful answer here. It fixes the boundary between the two planes.

## Answer

Read against LiteLLM v1.98.0 on 2026-09-01.

Inbound, virtual keys do exactly what D6 needs. `POST /key/generate` scopes a key to specific models, with `max_budget` and `budget_duration`, `tpm_limit` and `rpm_limit`, and `duration` for expiry. SSO gates only the admin UI, not API calls. Gating actual requests with OIDC or JWT is an Enterprise feature.

Outbound, both non-key paths are confirmed. Vertex AI takes a service account JSON or application default credentials through `vertex_credentials` and `vertex_project`. Bedrock inherits boto3's whole chain, including STS AssumeRole via `aws_role_name` and bare instance-role credentials with no static secret at all.

Subscriptions: no. Nothing lets Hydra hold one shared credential for subscription-priced Claude access. LiteLLM does forward a real Claude Code OAuth session through itself, merged January 2026, but only when the genuine `claude` binary holds that session: one user, one login. Anthropic's legal and compliance page states OAuth tokens are for native Anthropic applications and forbids routing subscription credentials on behalf of users, reserving enforcement rights, which it used in April 2026 against OpenClaw.

So the wall between the two planes is a terms limit, not a technical one, which makes it firmer rather than softer. Hermes' Nous-and-xAI-only proxy list, reconfirmed live, fits the same pattern: proxying subscription OAuth is always provider-specific and narrow, never generic.

D4 stands, now with evidence. Subscriptions stay in the agent-CLI plane. LiteLLM handles API keys and IAM only.

Detail: [research/04-litellm-auth-modes.md](../research/04-litellm-auth-modes.md)
