# 14 - Ouroboros rung one: watching failures, evolving skills

Type: grilling
Status: open
Blocked by: 05, 06, 09

## Question

D11 ships rung one first: Ouroboros edits skills, prompts and config, never Hydra's own source. This ticket designs it.

Settle:

- What counts as a failure worth learning from, read out of the run records. Repeated tool errors, a human rejecting a diff, a retry loop, a session abandoned
- How many occurrences before it acts, and over what window. The paper's phrase is durable error classes; decide what durable means in numbers here
- What it proposes: a new skill, an edit to one, a prompt change, a config change
- Who reviews a proposal, and whether a human approves before it lands, or only sees it afterwards
- Which guardrails from ticket 05 apply verbatim, and which need changing for Hydra
- Which model does the reviewing, and on which auth plane it runs
- How to tell whether a change helped, which means deciding now what evidence a later comparison would need

Scenario: the same skill gets rewritten three weeks running and each version is worse than the last. What catches that?

## What the research changed

**Ticket 05 supplies the rule list**, fifteen of them, each with the reason behind it. Read the findings file rather than restating them here.

Its headline finding is a missing guard in Hermes, and it reframes this whole ticket. The curator's reviewing model is told in its prompt that archiving moves a directory into `.archive/` and is recoverable. The tool it actually calls runs `shutil.rmtree`. So per-skill recovery does not exist for anything the model consolidates or prunes, and the only safety net is the pre-run snapshot. Dry-run's no-mutation promise rests on prompt text with no tool whitelist.

The rule that follows: **a guarantee written in a prompt is not a guarantee.** Every limit Hydra's curator claims has to be enforced in code, at the tool-dispatch layer, or it does not exist. Design this ticket's guardrails on that basis.

**Ticket 06 supplies the number to design against.** Ouroboros blocks 63.5% of recent self-edits. Two of every three proposals get rejected. Any design that treats a proposal as usually landing is wrong, and the rejection path is the common path.
