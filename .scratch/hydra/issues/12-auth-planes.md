# 12 - The two auth planes, and where secrets live

Type: grilling
Status: open
Blocked by: 04, 08, 18

## Question

D4 splits auth into two planes and D6 says agents never hold provider secrets, they ask the harness. This ticket says how.

Settle:

- The agent-CLI plane: each vendor CLI owns its own OAuth token today. Does Hydra touch those at all, or only launch the CLI and let it authenticate itself? If Hydra never touches them, D6 is satisfied by doing nothing, which is worth stating out loud
- The raw-model plane: where the real provider keys live, and what stands between an agent and them. macOS Keychain, an age-encrypted file, Bitwarden as Hermes uses, or the LiteLLM proxy holding everything behind one virtual key
- Multi-account routing, since the whole point is using several accounts. Who picks which account, on what signal, and how exhaustion is detected. Hermes' pooled credentials with exhaustion tracking are the prior art
- What a delegated child receives: a virtual key, a socket, or nothing at all
- What Hydra does when a token expires mid-session
- What lands in the run record about auth, and what must never land there

Scenario: a delegated cursor-agent run needs a Gemini call on the raw plane, one Gemini account is rate-limited and the other is in an error state, which is the situation `hermes auth list` shows right now. What happens?

## What ticket 08 found

The inventory removes a component and dissolves the scenario above.

**No multi-account routing.** One account per provider: Anthropic one, Google one, Cursor one, Nous zero. Nobody picks between accounts because there is nothing to pick between. Hydra's plurality comes from running several providers, not several accounts inside one. Keep Hermes' record shape, meaning `priority` alongside the `last_error_*` block, since it costs nothing against a list of one and costs a migration later.

**The scenario in this ticket is built on a misreading.** There are not two Gemini accounts, one rate-limited and one erroring. There is one AI Studio key and one junk entry someone added by hand: `label: "error"`, `source: manual`, an `access_token` one character long, and `priority: 1` so Hermes tries it first on every call. `request_count` is 0 across the whole pool, so none of it has ever run. Rewrite the scenario against a real failure, a single key hitting its quota, or drop it.

**The raw-model plane is nearly empty.** Anthropic has no API key and no credit, and ticket 04 already ruled out carrying the Pro subscription upstream on Anthropic's terms. Cursor has no API key either. So the raw plane holds one Google AI Studio key and one Copilot token lifted from `gh`. Anthropic and Cursor are both agent-CLI only. Do not spec a raw plane richer than that without someone buying credit first.

**On the first question, the answer looks like yes.** Each vendor CLI already owns its login and keeps it somewhere Hydra has no reason to touch: Claude Code and Cursor in the macOS Keychain, Google either in `agy` or in `GEMINI_API_KEY`. Hydra launching the CLI and letting it authenticate itself satisfies D6 by writing no code, which this ticket asked to be stated out loud.

Full inventory in [research/08-account-inventory.md](../research/08-account-inventory.md).

## What ticket 18 found

**Google leaves the raw plane.** The Google provider is `agy` on the Google AI Pro subscription,
which authenticates itself by browser OAuth and needs no key. The raw-model plane keeps the AI
Studio key only as a cold fallback, so it now holds one unused Google key and one borrowed Copilot
token. Do not build a plane around that.

**D16 is the new constraint here.** The Cloud project behind `GEMINI_API_KEY` has billing enabled,
so Hydra never spends on it without asking. That turns "where do the keys live" into a smaller
question than this ticket assumed: the key already lives in the environment, and what matters is
the gate in front of it rather than the storage under it.

**The first question is answered, with a correction to how.** Each vendor CLI owns its own login,
as ticket 08 said. The correction is that `~/.gemini/settings.json` proves Hydra cannot be entirely
hands-off: `selectedAuthType: oauth-personal` was pinned there, so `gemini` ignored a working
`GEMINI_API_KEY` and failed with `IneligibleTierError` for reasons that had nothing to do with the
key. If Hydra ever promotes the fallback, it has to set the auth type per run. `GEMINI_CLI_HOME`
points the CLI at a different config root, which is how to do that without touching the user's own
`~/.gemini/`.
