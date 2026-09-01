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
