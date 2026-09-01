# 08 - Inventory the accounts and tiers

Type: task
Status: open

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
