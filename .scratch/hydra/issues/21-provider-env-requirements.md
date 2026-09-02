# 21 - What environment each provider needs, and what it leaks

Type: research
Status: claimed
Blocked by: 12

## Question

Ticket 12 decided that Hydra builds each child's environment from an allowlist, so no provider
secret reaches an agent by inheritance. The allowlist is written as `PATH`, `HOME`, `TERM`, `LANG`
"plus whatever a given provider genuinely needs", and that last clause is unmeasured. Scrub too
hard and a provider fails to launch or silently loses its login. Scrub too little and D6 is a claim
rather than a fact.

Settle, for `claude` 2.1.257, `cursor-agent` 2026.08.31-4057e58, `agy` 1.1.23 and `hermes` 0.14.0:

- Which environment variables each binary reads, and which of those it needs to start, to reach its
  own credential, and to run a task
- Which variables carry a secret, so the scrub has a complete target list rather than
  `GEMINI_API_KEY` alone
- What each one does when a needed variable is missing: a clear error, a silent degrade, or a
  fallback to a different auth path
- Whether any of them read a proxy, CA or telemetry variable that Hydra should pass through on
  purpose rather than by accident
- The minimum viable environment for each provider, stated as a list

The answer becomes the allowlist in the harness spec and feeds ticket 13's launch sequence.
