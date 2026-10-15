# 129 - What stands between Hydra and a public repository, and what it has that the shelf does not

Type: research
Status: resolved
Blocked by:
Opened: 2026-10-15, by the session, from the human's note of the same day ("qué nos falta para hacer
público nuestro proyecto", "dime si tenemos algo diferenciador", "qué crees que deba mejorarle").
Research: the session read the repository's public face, four projects on the same shelf, and
three curated lists, and wrote what it found. The decisions it names are the human's.

## Question

The repository `FrankMdza/hydra-harness` is private on 2026-10-15, 681 commits, 0 stars. What has
to change before the switch is flipped, what does Hydra do that OpenCode 2.0, ghostex, Omnigent
and Scion do not, and what should it do better before strangers read it?

## Findings

`research/129-ready-to-go-public.md` holds the full reading. The short version:

- Before the flip, in order: a secret scan of the whole history with a real tool (a pattern grep
  found nothing, which is not the same); the CI made green on master and on Dependabot's PRs
  (`sha2` 0.11 breaks `hydra-grader`, and the `security` workflow fails at startup on every PR);
  the README rewritten and its comparison table re-dated; `CHANGELOG.md`'s Unreleased section
  closed into a `v0.1.0` tag and a GitHub release; topics and a one-line homepage on the
  repository; the author's absolute paths out of `.scratch/` and `IMPLEMENTATION.md`, or left
  with a sentence saying why.
- The differentiator is one sentence and it is measured: the four CLIs exit 0 and report success
  on a run where every tool call was refused, and Hydra's record says `gated` instead. None of
  the four projects read derives an outcome from the event stream or checks a record against a
  contract. Omnigent is the closest in shape (nine agents, bwrap and seatbelt, approval and spend
  policies, 10.8k stars) and documents no per-run record at all.
- What to improve first: make that sentence demonstrable in one command (`hydra show --verdict`
  or a `verdict` subcommand, today a Python script under `docs/demo/`); compile on Linux (build
  315 is running); add OpenCode as a fifth provider, since it logs in, has hooks and is the agent
  with the most users; shorten the public documents and move the ticket-speak behind a link.
