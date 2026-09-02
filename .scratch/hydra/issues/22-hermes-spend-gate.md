# 22 - Where D16's gate sits for hermes

Type: grilling
Status: claimed
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
