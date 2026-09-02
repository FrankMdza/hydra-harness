# 12 - The two auth planes, and where secrets live

Type: grilling
Status: resolved
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

## What the evidence pass found, 2026-09-01

Method and detail in [research/12-auth-evidence.md](../research/12-auth-evidence.md), 776 lines.

- **The Copilot token has never made a call.** It is two tokens, not one: a raw GitHub OAuth token
  from `gh auth token`, then an exchange at `copilot_internal/v2/token` for a short-lived Copilot
  API token. That exchange has never succeeded here. `get_copilot_api_token()` swallows the failure
  and returns the raw GitHub token, so nothing downstream notices. `request_count` is 0 and every
  `last_error_*` is null.
- **So the raw-model plane holds nothing that has ever run.** Ticket 18 took Google off it, leaving
  a cold fallback nobody starts without approval, and the Copilot half is unexercised. Whether D4's
  raw-model plane exists in v1 is the question this ticket now has to answer, and it is bigger than
  the one the ticket was written to ask.
- **Every vendor CLI owns its own login, confirmed.** Keychain services: `Claude Code-credentials`,
  `Cursor Safe Storage`, `cursor-access-token`, `cursor-refresh-token`, `gh:github.com`, and
  `gemini` with account `antigravity` for `agy`. `agy` has no auth subcommand at all. Hydra
  satisfying D6 by writing no code is evidence now, not a guess.
- **Three plaintext exposures on this machine.** `~/.hermes/auth.json` holds the raw GitHub token in
  plaintext with repo scope. `gemini-credentials.json` is AES-256-GCM keyed by scrypt over hostname
  and username, so anyone who can read the file can derive the key. `~/.gemini/oauth_creds.json` is
  plaintext and expired.
- **One provider secret lives in the environment**, `GEMINI_API_KEY`, exported from `~/.zshrc`.
  Every other provider reads a store. That is the raw input for what a delegated child receives.

## Answer

**Hydra runs on subscription OAuth and holds no provider credential.** Every vendor CLI owns its
own login in the macOS Keychain. Hydra launches the CLI and stays out of the way, which satisfies
D6 by writing no code. The only secret Hydra ever handles is `GEMINI_API_KEY`, and only on a run
where a human typed `--allow-billed`.

The ticket asked six questions. Three died on the evidence from tickets 08 and 18 and the
2026-09-01 evidence pass. The three that survived produced seven decisions.

### The raw-model plane does not ship in v1

D4 is amended. Nothing on that plane has ever run: ticket 18 moved Google off it, and the Copilot
half never completed its token exchange, so `request_count` is 0 across the pool. Building LiteLLM,
virtual keys and a credential store around that is speccing fiction.

v1 ships the agent-CLI plane only. The raw plane is one gated path, `GEMINI_API_KEY` handed to
`gemini`, and nothing else. The harness spec names it as an extension point and builds none of it.
Ticket 04's virtual-key finding is not discarded; it is the design for the day the plane gets built.

### Children get an allowlisted environment

`GEMINI_API_KEY` is exported on line 1 of `~/.zshrc`, so today every child Hydra spawns inherits it
silently. That makes D6 false in practice while nobody notices.

Hydra builds each child's environment from an allowlist: `PATH`, `HOME`, `TERM`, `LANG`, plus
whatever a given provider genuinely needs. No secret arrives by inheritance. The raw plane injects
the key deliberately, per run, after approval. This is the only place D6 requires real code.

### `--allow-billed`, typed per run, never inherited

D16 needed a mechanism. A blocking prompt trains you to hit yes and a config toggle stays on for
months, so it is a flag in argv. An agent that did not have it in its argv cannot reach the money
path, and the flag lands in the run record as an argument, so the record shows why a run was
allowed to spend.

Delegation cannot set it and cannot pass it down. Ticket 11's policy has no authority over this
flag, and a child needing the billed path hits the auth wall below like any other. Ticket 11 caps
depth at 2, so without this rule an agent two levels away from the human could spend. The record
can never show a spend nobody authorised.

### The write-time filter strips what Hydra injected

Ticket 09 filters "the ones visible in its auth config" at write time. Hydra has no auth config, so
that list would be empty and the promise hollow.

The list is instead the values Hydra injected for that run: `GEMINI_API_KEY` on an approved run,
nothing otherwise. Hydra knows the value at injection time, so it strips that exact string from
frames as it writes them. That is a guarantee it can keep.

The leak this closes has nothing to do with where the key is stored. An agent runs `env` or reads
`~/.zshrc`, the provider streams that output back as a frame, and ticket 09 writes frames verbatim.
Hydra never stored the key and it lands on disk anyway. Broad scanning for anything key-shaped is
best-effort by nature and belongs to ticket 19 at export, not to a write-time guarantee.

### Auth failure is its own run outcome, and it says what to do

An expired token is not a task failure. Ouroboros rung one must never evolve a skill because a
login lapsed. The run ends on a distinct terminal outcome carrying a remediation string that names
the provider and the command to run. Hydra refreshes nothing, since the CLI owns its own login.

Ticket 14's failure-signals research names truncation, rate limit and quota classes and no auth
class at all, so the taxonomy slot is empty. Ticket 14 owns the signatures; this is 12's input.

### Auth never happens inside a run

Three layers. Pre-flight before launch, and refuse to start a provider that is not logged in. A run
that hits the wall anyway fails fast with the remediation string. And if a provider ever asks for a
pasted code, Hydra suspends teeing, hands the bare pty to the human, and resumes teeing after.

This is structural rather than a policy choice: ticket 11 spawns delegated children detached, and a
detached child cannot complete a browser login.

Flow shapes, checked 2026-09-02. `gemini` 0.42.0 uses a loopback redirect. Its 84-chunk bundle
carries `oauth2callback`, `http://localhost` and `createServer`, with zero `device_code` and zero
`user_code`, so the browser hands the code to a server the CLI runs itself and nothing is pasted
through the terminal.

`agy` never asks for a pasted code either, settled the same day. Its `device_code` and `user_code`
strings belong to `google3/third_party/golang/oauth2/oauth2.(*Config).DeviceAuth`, Google's vendored
copy of `golang.org/x/oauth2`. That package ships device-flow support unconditionally and Go links
whole packages, so the symbols say nothing about what `agy` calls. The nearby strings
`endpoint missing DeviceAuthURL` and `oauth2: cannot auth device: %v` are that library's own error
text. No loopback callback URL literal is in the binary either.

What `agy` does instead is better than this ticket assumed. It carries a first-class
`AuthResult_HeadlessAuthRequired` state, with `GetHeadlessAuthRequired` and a `HeadlessAuthWriter`
beside it, so a login it cannot complete becomes a typed result rather than an improvised terminal
prompt. Interactive login prints `Open in your browser: %s`, which sends a URL out and takes nothing
back through the pty. On this machine the credential is already cached: `server_oauth.go:192` logged
`applyAuthResult: email=<address>, authMethod=consumer` 250ms after process start on 2026-09-01,
with no browser involved.

So Hydra does not sniff for a paste prompt on Google. It reads `HeadlessAuthRequired`, which is
exactly the signal the pre-flight check and the auth-failure outcome need. The teeing suspension
stays as the generic net for the other three providers, where nothing equivalent is confirmed. The
remediation string for Google is to run `agy` once interactively and follow its prompts.

### What the record carries about auth

Drop `plane` as a field name. D4's correction says a plane names the auth mechanism and nothing
else, so a field called `plane` is a second word for mechanism, and with one plane shipping it
carries no information.

- `auth.mechanism`: `subscription-oauth` or `api-key`
- `auth.source`: `keychain`, `env` or `none`
- `auth.allow_billed`: whether the flag was present

Never on the record, in any form: the injected value, a prefix of it, or its length. "Plane"
survives in the spec prose, where it is useful for talking about D4's two halves, and never becomes
a field. Ticket 09 owns the schema, so this is input to it rather than a schema edit made here.

### The sub-questions this ticket listed

**Does Hydra touch the vendor OAuth tokens?** No, with one exception. Every provider authenticates
itself and Hydra reads none of their stores. The exception is ticket 18's finding that
`~/.gemini/settings.json` pins `selectedAuthType`, so if the fallback is ever promoted Hydra sets
the auth type per run through `GEMINI_CLI_HOME` rather than editing the user's own `~/.gemini/`.

**Where the raw keys live.** One of them, in the environment. v1 builds no store.

**Multi-account routing.** Dissolved by ticket 08. One account per provider, nothing to route.

**What a delegated child receives.** Nothing. No virtual key, because no plane. No inherited
secret, because of the allowlist. No `--allow-billed`, because it is not inheritable. No browser
login, because it is detached.

**Token expiry mid-session.** Covered above.

**What lands in the record.** Covered above.

**The scenario.** Dropped, per ticket 08. It was built on a misread of `hermes auth list`.

### Out of scope, and one thing that is not a Hydra problem

The three plaintext exposures stay out of scope for this map. The harness spec gets one line naming
it a non-goal: Hydra reads none of those files.

Separately, and not as map work, `~/.hermes/auth.json` holds a repo-scoped GitHub token in
plaintext. Worth rotating on its own account.

Checked 2026-09-02: `~/hydra` has no git remote, and a scan of the working tree and all eight
commits for Google, Anthropic and GitHub token shapes returned nothing. There is no exposure to
purge.
