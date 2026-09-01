# 08 - Inventory the accounts and tiers

Type: task
Status: resolved

## Question

The auth spec cannot be written against guesses about what you are subscribed to. This ticket produces the inventory, with the human in the loop, because only you can see the billing pages.

For each of Anthropic, Google, Cursor and Nous, record:

- Which accounts you hold, and which you intend Hydra to use
- The tier of each: subscription, pay-as-you-go API, free
- Whether an API key exists today, and where it currently lives
- The rate and usage limits that apply, and how you would notice hitting one

Also record the two credential stores already on this machine, since they are prior art and possibly prior mess. `hermes auth list` shows a pooled `copilot` credential taken from the `gh` CLI, and two `gemini` entries, one of them in an error state. `~/.hermes/.env` is 23KB, which suggests secrets are already scattered.

Done when the inventory exists as a file in the repo, with no secret values in it, only names and locations.

## What ticket 01 found, which changes one question here

`gemini` 0.42.0 with `selectedAuthType: "oauth-personal"` cannot authenticate on this machine. It returns `IneligibleTierError` with `reasonCode: UNSUPPORTED_CLIENT`, saying Google retired Gemini Code Assist for individuals on this client and pointing at the Antigravity suite.

So the question "which tier is your Google account on" now has a sharper edge. Your Gemini Pro subscription appears to ride `agy`, the Antigravity CLI, which has a `useG1Credits` setting for Google One AI Premium quotas. The `gemini` binary needs `GEMINI_API_KEY` or Vertex, both of which are pay-per-token.

Also worth recording here: nothing is logged into Antigravity on this machine. `cli.log` repeats "You are not logged into Antigravity", and all three conversation directories are empty.

## Answer

Inventory lives in [research/08-account-inventory.md](../research/08-account-inventory.md). Three things came out of it that change other tickets.

**One account per provider.** Anthropic one, Google one, Cursor one, Nous zero. So ticket 12 builds no account selection and no rotation, because there is nothing to rotate between. Hydra's plurality comes from running several providers, not several accounts inside one. Copy Hermes' pooled-credential record shape anyway, since `priority` and an error block cost nothing against a list of one and cost a migration later.

**Anthropic has no raw-model plane.** No API key, no credit balance, only Claude Pro at USD 20/month. Ticket 04 already found that nothing carries a Claude subscription upstream and that Anthropic enforces it in its terms. Together those mean Anthropic is agent-CLI only. Any design assuming a raw Anthropic model call assumes a purchase nobody has made.

**D14 is wrong and is corrected in the map.** It claims Hermes brings xAI and Nous along for free. No Nous account exists, no Portal subscription, and xAI is out of scope by choice. Hermes brings a Copilot token lifted from `gh` and a second copy of the Google key.

Which leaves the raw-model plane thinner than D4 reads: one Google AI Studio key and one borrowed Copilot token. Cursor has no API key either, so it is agent-CLI only too.

**Where the truth lives.** Each vendor CLI keeps its own login, Hydra touches none of them, and D6 is satisfied by writing no code. LiteLLM virtual keys cover what little raw plane exists.

**On the Google question this ticket sharpened.** The direction is OAuth2, which in practice means signing into `agy`, since it is the only client pairing the Pro subscription with an agent. Nobody has logged in yet. `~/.gemini/oauth_creds.json` is a stale copy of the path that returns `IneligibleTierError`, not a usable store. `GEMINI_API_KEY` is an AI Studio key at 39 characters, pay-per-token, and today it is the only Google credential that works. `gcloud` is not installed and no GCP project exists, so Vertex is not reachable from here.

`agy` 1.0.10 offers `--print`, `--continue`, `--conversation <id>`, `--model` and a plugin system, but still no `--output-format`. The trigger the map set for reopening ticket 18 has not fired.

**Two things to fix on the machine.** The Hermes `gemini` entry in an error state is not an exhaustion marker, it is junk: `label: "error"`, `source: manual`, an `access_token` one character long, and `priority: 1` against the real key's `priority: 2`, so it gets tried first on every call. `request_count` is 0 across the pool, so none of it has ever run. And the Cursor credit balance of USD 25 expires today at 9:25 PM.

**Still open, and small.** Cursor's plan tier is unnamed on the credits screen, and whether to buy Anthropic API credit is a purchase decision rather than a research one.
