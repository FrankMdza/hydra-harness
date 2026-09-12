# 58 - Codex CLI as the fourth provider, and what a provider without a hook is

Type: research
Status: open
Blocked by:

## Question

The human wants more providers that sign in rather than take a key, and named OpenAI's
Codex CLI as the next one. Its auth fits D4's agent-CLI plane as written: a vendor binary on
a ChatGPT subscription, no API key in any environment Hydra builds.

What is not known, and decides whether it is routable at all: **does Codex CLI expose a
pre-tool hook the gate can compile into?** Every routed provider today does (`claude`'s
`PreToolUse`, Cursor's `preToolUse` and `beforeReadFile`, `agy`'s own), and the gate is a
script registered in that hook. Codex ships approval modes and an OS sandbox of its own
(seatbelt on macOS, landlock on Linux) rather than, as far as the map knows, a hook a third
party can register. If that holds, Hydra has three options and none is free:

1. Route it ungated and record `policy: null`, the way pre-gate records already read. Honest,
   and it makes the fourth provider the one population the design says it does not want.
2. Drive Codex's own sandbox from the policy: translate `standard` into its approval and
   sandbox flags at spawn. A fourth dialect that is not a hook, so the capability table gains
   rows the gate never sees and the record cannot carry a per-call decision.
3. Refuse to route it until it has a hook, and say so in the README's roadmap.

Measure before choosing: install `codex`, read its `--help` and config for anything named
hook, and run one read-only task under its strictest mode to see what its stream carries and
whether per-call decisions appear anywhere Hydra could tee. Ticket 01's rule applies, drive
what the binary offers rather than what the docs promise.

Not a build ticket until the answer is one of the three.
