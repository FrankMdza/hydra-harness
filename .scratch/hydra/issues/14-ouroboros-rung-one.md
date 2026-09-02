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

## What the evidence pass found, 2026-09-01

Provider-against-signal table and method in
[research/14-failure-signals.md](../research/14-failure-signals.md), 607 lines.

- **Three of four providers report failure at the tool level and success at the run level, in the
  same run.** `claude` closed `terminal_reason: "completed", is_error: false` with a real denial
  sitting in `permission_denials`. `agy` returned `status: "SUCCESS"`, empty response, exit 0 on a
  run whose only tool was auto-denied. `cursor-agent` returned `is_error: false` with one tool
  rejected and one failed. Anything keyed off the terminator files a failed run as clean.
- **Two of this ticket's four failure classes are not derivable at all.** A session abandoned has no
  pid, no exit code, no heartbeat and nothing written at exit; a Cursor run killed at 6 seconds
  leaves zero bytes, indistinguishable from one that never started. A human rejecting a diff has no
  producer: `claude` labels a headless auto-deny `user-rejected` exactly like a real one, and ticket
  09's `correction` event has no producer anywhere.
- **Repeated tool errors are derivable but not comparable across providers.** `agy` gives a
  permission denial and a file-not-found the same `state: "ERROR"` and `error.type: "TOOL_ERROR"`,
  and both messages contain the word "permission", so the obvious substring rule is wrong rather
  than merely weak.
- **Ticket 09 rests on an expired fact.** It gave Cursor `usage_source: estimated` because Cursor
  emitted no token counts. The 2026.08.31 build emits `usage` with four token fields, and emits
  `thinking` in print mode where ticket 09 says it never does. Settle that before ticket 16 writes
  the record contract.
- **For ticket 16's seam check:** ticket 11 puts `exit_code`, `status`, `status_source` and `error`
  in the JSON `hydra run` returns, and ticket 09 puts none of them in the run file.

## What ticket 12 settled

**Auth failure is a terminal outcome distinct from task failure, and rung one must never learn from
it.** A skill does not need evolving because a token expired. The run ends carrying a remediation
string that names the provider and the command to re-authenticate.

The signature taxonomy is this ticket's to own. The failure-signals research names truncation, rate
limit and quota classes and no auth class at all, so that slot is empty and needs filling here.

## What ticket 13 added

**Some records arrive late and in a different shape.** Interactive runs cannot stream, because a
vendor CLI keeping its own TUI cannot also speak ACP. They are harvested from the vendor's own
transcript when the run exits, and for `agy` and `cursor-agent` that transcript is a sqlite database
or a chat directory rather than an append-only log. So rung one reads a corpus of mixed provenance,
and a run that looks empty may simply not have been harvested yet.

**Ouroboros cannot call `hydra prune` or `hydra reap` at either rung.** Cleanup stays a human
command, so nothing the loop does can destroy the evidence it learns from.
