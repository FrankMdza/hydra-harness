# 22 - Where D16's gate sits for hermes

Type: grilling
Status: resolved
Blocked by: 21

## Question

D16 says Hydra never spends on a billed API key without asking, and ticket 12 gave it a mechanism:
an `--allow-billed` flag typed per run, with the environment allowlist making sure no key reaches a
child that was not granted one. Ticket 21 found the allowlist cannot enforce that on hermes.

`~/.hermes/.env` loads with `override=True` at `hermes_cli/env_loader.py:214`, on the `cli.py` and
`run_agent.py` entry points, and its own docstring says it overrides stale shell-exported values.
So a key sitting in that file beats anything Hydra passes in. Scrubbing the environment does not
gate this provider.

## What is actually loaded today, checked 2026-09-02

The hole is latent, not open. Hermes holds no usable Google credential:

- `credential_pool.gemini` has a **1-character** `access_token`, `source: manual`, `priority: 1`,
  `request_count: 0`. This is the junk entry ticket 08 identified.
- `credential_pool.copilot` holds the `gh` token whose exchange has never succeeded (ticket 12).
- `providers` in `auth.json` holds Spotify and nothing else.
- `GOOGLE_API_KEY` is declared in `~/.hermes/.env` with an **empty value**.

So `hermes acp` advertising `authMethods[0].id = "gemini"` under an empty environment is one junk
character passing a truthiness check, not a key. Nothing can be spent through hermes today. What
makes this worth a ticket is that the name is already declared in a file that overrides Hydra, so
the day anyone pastes a real key there, hermes spends it and D16 never sees it.

## The lever

`HERMES_HOME` redirects the config root, and both paths hang off it:
`hermes_cli/auth.py:803` resolves `get_hermes_home() / "auth.json"`, and
`hermes_constants.py:387` resolves `get_hermes_home() / ".env"`. Ticket 21 put `HERMES_HOME` on the
never-pass list, so promoting it to a deliberate per-run set is a reversal that needs stating.

Settle:

- Whether hermes is routable in v1 at all. D14 already reduced it to a Copilot token whose exchange
  has never succeeded, and the inventory above shows every other credential is junk or empty. A
  provider with nothing that works is a strange thing to spec a gate for
- If it stays, whether Hydra sets `HERMES_HOME` to a config root it owns, and what that root
  contains
- Whether Hydra ever writes to a provider's own config to disarm it. Ticket 12 established Hydra
  touches no vendor credential store, and this would be the first exception
- What `hydra run --provider hermes` does without `--allow-billed`: refuse to launch, launch
  against a Hydra-owned config root, or launch and record that it could not be gated
- Whether the run record should mark a provider as ungatable, so a reader can tell a run that was
  gated from one that only looked gated

This is the first case where a provider's own state can defeat a harness-level control, so the
answer sets the precedent for any provider that behaves the same way later.

## Answer

**Hermes is not routable in v1, and D14 is amended.** The gate this ticket was opened to design does
not get built, because the provider it would gate is not launched.

### Why it goes

D14 made hermes a fourth routable provider on the strength of the accounts it would bring. Every
part of that has since failed:

- Ticket 08 found neither the xAI nor the Nous account exists
- Ticket 12 found the Copilot token exchange has never succeeded, and the failure is swallowed
- This ticket found the rest is junk: a 1-character `access_token` on the `gemini` pool entry, an
  empty `GOOGLE_API_KEY` in `~/.hermes/.env`, and Spotify as the only entry under `providers`

So hermes holds nothing that works, and it is the only provider Hydra cannot gate through the
environment, because `~/.hermes/.env` loads with `override=True` and beats anything Hydra passes.

D13 already says Hermes is worth copying rather than depending on, and D15 takes its guardrail rules
rather than its code. What this project gets from Hermes is ticket 05's fifteen rules. It does not
need to launch the binary to have them.

Three providers that work beat four where one is theatre.

### The rule this sets, which outlives the decision

**A provider Hydra cannot gate is either given a config root Hydra owns, or not routed.** There is
no third branch where Hydra launches it and records that the gate did not apply. That would make
D16 a preference.

`HERMES_HOME` is what the first branch looks like in practice. It redirects both `auth.json`
(`hermes_cli/auth.py:803`) and `.env` (`hermes_constants.py:387`), so the mechanism exists and is
specced if hermes ever comes back with a working credential. It comes off ticket 21's never-pass
list for that reason.

**Hydra still never writes into a vendor's own config.** Redirecting a provider's config root is not
the same as editing the user's, and ticket 12's rule holds unchanged.

### What the record carries

`auth.gated`, a boolean, beside the three fields ticket 12 added. On a provider Hydra fully controls
it is always true, so it costs nothing to carry. If Q2's rule holds, `false` is unreachable in v1,
which makes a `false` appearing in the corpus later a signal that something about the provider set
changed. Ticket 09 owns the schema; this is input to it.

### Knock-on effects

**The raw-model plane is now emptier than ticket 12 left it.** Its two entries were the gated Gemini
fallback and the Copilot token lifted from `gh`. The Copilot token only ever reached Hydra through
hermes, so with hermes unrouted it has no consumer at all. In v1 the raw plane is one gated Gemini
fallback and nothing else.

**Ticket 09's schema rules survive without hermes.** "The schema must not require a terminator
event" was justified by Hermes writing none, but `cursor-agent` writes one only on success, so the
rule stands on its own.

**Ticket 21's hermes measurements stay useful.** They are the evidence for this decision, and they
are what the spec cites if the provider is ever reconsidered.
