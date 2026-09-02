# 22 - Where D16's gate sits for hermes

Type: grilling
Status: open
Blocked by: 21

## Question

D16 says Hydra never spends on a billed API key without asking, and ticket 12 gave it a mechanism:
an `--allow-billed` flag typed per run, with the environment allowlist making sure no key reaches a
child that was not granted one. Ticket 21 found that mechanism does not reach hermes.

Driven with a completely empty environment, `hermes acp` still advertises
`authMethods[0].id = "gemini"`, which `acp_adapter/auth.py:11` only emits when a credential actually
resolved. Hermes persisted a byte-identical copy of `GEMINI_API_KEY` into `~/.hermes/auth.json` at
seed time and reads it from there. `~/.hermes/.env` also loads with `override=True`, so it beats
anything Hydra passes in. Scrubbing the environment changes nothing about what hermes can spend.

The AI Studio project behind that key has billing enabled, which is why D16 exists.

Settle:

- Whether hermes is routable in v1 at all, given D14 already reduced it to a Copilot token whose
  exchange has never succeeded. If its only working credential is one that spends money, the case
  for routing it in v1 is thin
- If it stays, where the gate goes. `HERMES_HOME` pointed at a Hydra-controlled config root is the
  obvious candidate, since it would leave the user's own `~/.hermes/` untouched. Ticket 21 lists
  `HERMES_HOME` as never-pass, so promoting it to a deliberate set is a reversal that needs stating
- Whether Hydra ever writes to a provider's own config to disarm it, or refuses to route a provider
  it cannot gate. Ticket 12 established Hydra touches no vendor credential store, and this would be
  the first exception
- What `hydra run --provider hermes` does without `--allow-billed`: refuse to launch, launch with
  the credential removed, or launch and record that it could not be gated

This is the first case where a provider's own state defeats a harness-level control, so the answer
sets the precedent for any provider that behaves the same way later.
