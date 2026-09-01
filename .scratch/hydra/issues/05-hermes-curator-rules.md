# 05 - Extract the Hermes curator's guardrail rules

Type: research
Status: resolved

## Question

D15 says Hydra builds its own curator on Hermes' rules. The source is on disk at `~/.hermes/hermes-agent`. Read it and write down the rules, not the code.

From `hermes curator --help`, the visible behaviour is: an auxiliary-model background task that reviews agent-created skills, consolidates overlaps and archives stale ones. It snapshots the skill directory before every real run, never touches bundled or hub-installed skills, and never deletes. Pin opts a skill out. Rollback and restore undo it. Prune bulk-archives skills idle past a threshold.

Answer the parts the help text does not cover:

- What triggers a run, on what schedule, and what stops one mid-flight
- How it decides a skill is stale, overlapping or obsolete, and what evidence it uses
- Which model does the reviewing, and how it is prompted
- What the snapshot actually captures and where it lands
- Every failure mode its authors guarded against, since each guard implies a bug they hit

The output is a list of rules Hydra's curator must honour, each with the reason behind it. Rules without reasons are cargo cult.

## Answer

Fifteen rules, and two bugs to avoid copying.

The rules, in brief: scope edits to skills the background self-improvement fork itself created, which is narrower than "non-bundled and non-hub". Never touch pinned skills, but let patches through. Anchor staleness on activity, with `created_at` as fallback. Never let usage counters gate an overlap judgment. Require `absorbed_into` to be declared at the moment of deletion, as the authoritative signal. Validate that any claimed merge target exists. Snapshot before every mutating pass, and make rollback itself reversible. Treat a failed snapshot as a warning, not a blocker. Exclude hub-managed content from snapshots. Rewrite other subsystems' name references after a consolidation. Defer the first-ever run by a full interval. Keep deterministic transitions independent of, and prior to, the model pass. Always install a non-interactive approval callback on any fork that can reach a shell.

The two Hermes gets wrong, which Hydra must fix rather than inherit: make archive and delete the same real operation everywhere, and enforce dry-run's no-mutation guarantee at the tool-dispatch layer rather than in prompt text.

The most valuable finding is an absent guard, and it is load-bearing. The reviewing model's prompt states as a hard rule that archiving means moving the directory into `~/.hermes/skills/.archive/`, and that archives are recoverable while deletion is not. The tool it is told to call for that, `skill_manage action=delete`, is implemented in `tools/skill_manager_tool.py:_delete_skill` as `shutil.rmtree(skill_dir)`. That is a permanent filesystem delete, not a move. The genuine archive function, `skill_usage.archive_skill()`, is only ever reached from the deterministic staleness sweep and from manual CLI commands, never from the model's own delete path.

So for anything the reviewer consolidates or prunes, per-skill recovery does not exist. The whole safety net is the pre-run snapshot. Related: dry-run's no-mutation promise rests on prompt text with no tool whitelist, unlike the sibling per-turn review fork, which gates its tools at dispatch.

The lesson for D15 is sharper than the rule list. A guarantee written in a prompt is not a guarantee. Hydra enforces its curator's limits in code or not at all.

Detail: [research/05-hermes-curator-rules.md](../research/05-hermes-curator-rules.md)
