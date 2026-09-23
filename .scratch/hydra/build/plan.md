# Hydra build

Label: build:index

The companion to `../map.md`. That map charted the design and closed at 38 of 38; its
destination was three documents and D1 forbade production code while it was open. It
reached that destination on 2026-09-05 and the code started after. **This file indexes
the build, and nothing in it reopens a design decision.** A ticket here that finds
itself needing a decision rather than an implementation has found the edge of the
design, and it should say so and stop rather than decide.

## Destination

`docs/` implemented: one router that launches all three providers on a gate it
compiled, records them per the contract, and hands Ouroboros a corpus worth reading.

## Notes

**Read in this order.** `docs/` for what to build, `IMPLEMENTATION.md` for what is
already built and every departure from the design with its reason, then this file,
then your ticket. Amend `IMPLEMENTATION.md` in the same change that narrows the gap;
a ticket is not done while that file still claims the gap is open.

**Re-read all three provider versions at the top of every session** and date every
provider-specific claim you write. Two of the three self-update without asking, `agy`
moved four times in four days, and `claude` moved between two tickets on this map's
predecessor. A finding has been invalidated by drift four times so far.

**Measure, do not read.** Ticket 34 on the design map verified a field by reading the
ternary that gated it, and ticket 35 found the field empty every time a human used it.
A field's presence in a conditional says the branch exists, not that anything writes
it.

**Prose follows `~/.claude/skills/unslop/SKILL.md`.** No em dashes.

## The tickets

- [01 - The policy set, the capability table, and the gate compiler](01-policy-set-and-gate-compiler.md)
- [02 - Wire the gate into a recorded `claude` run](02-gate-into-the-run.md)
- [03 - Normalize a `claude` run into the ten event types](03-normalizer-event-types.md)
- [04 - Derive the outcome and complete what `hydra run` returns](04-outcome-derivation.md)
- [05 - Detached spawn, the timeout, and `hydra status`](05-detached-spawn-and-status.md)
- [06 - `hydra reap` and `hydra prune`](06-reap-and-prune.md)
- [07 - The skill store, the snapshot, and per-run injection](07-skill-store-and-injection.md)
- [08 - MCP injection on `claude`](08-mcp-injection-claude.md)
- [09 - Route `cursor-agent`](09-route-cursor-agent.md)
- [10 - Route `agy`](10-route-agy.md)
- [11 - Delegation and the MCP delegate face](11-delegation.md)
- [12 - Ouroboros rung one](12-ouroboros-rung-one.md)
- [13 - `skill_invoke` and the `skill_snapshot` scope](13-skill-invoke-and-snapshot-scope.md)
- [14 - Two scope names, the two-kind deny list, and the reap sweep](14-two-scope-names-and-the-deny-list.md)
- [15 - `scopes` and `floor_lifted` on the record](15-scopes-and-floor-lifted-on-the-record.md)
- [16 - `judge` gets `--strict-mcp-config`, the probe follows, and `ToolSearch` moves](16-judge-strict-mcp-and-toolsearch.md)
- [17 - `claude` runs record their thinking text again](17-thinking-text-on-claude-records.md)
- [18 - `hydra show <run-id> [--events]`](18-hydra-show.md)
- [19 - The Herdr bridge, as a plugin on Herdr's side](19-herdr-bridge-plugin.md)
- [20 - A run's sequence as a graph in the terminal](20-hydra-graph-in-the-terminal.md)
- [21 - A smoke test for the bridge, against a running Herdr](21-bridge-smoke-test.md)
- [22 - Join a gate answer to its call when the provider pipelines calls](22-join-gate-answers-under-pipelined-calls.md)
- [23 - `base_tree` on the record](23-base-tree-on-the-record.md)
- [24 - The pane says what the record knows](24-the-pane-says-what-the-record-knows.md)
- [25 - No skill injection on `judge`](25-no-skills-on-judge.md)
- [26 - The face gets its parent, and the document admits it](26-the-face-gets-its-parent.md)
- [27 - `hydra run --interactive` on `claude`, and the harvest](27-interactive-claude-and-harvest.md)
- [28 - `hydra run --interactive --policy <name>` on `claude`](28-interactive-with-a-policy.md)
- [29 - A harvested run that met the rate limit derives `rate_limited`](29-harvested-rate-limit-is-rate-limited.md)
- [30 - `hydra reap` stages nothing of Hydra's own](30-reap-stages-nothing-of-hydras.md)
- [31 - `hydra usage recount`](31-hydra-usage-recount.md)
- [32 - `hydra resume <run-id>`](32-hydra-resume.md)
- [33 - The tee caps a line](33-tee-line-cap.md)
- [34 - `hydra export`](34-hydra-export.md)
- [35 - An `unhooked` row in the capability table](35-unhooked-tools.md)
- [36 - `hydra evolve`, Ouroboros rung two](36-hydra-evolve.md)
- [37 - A `claude` tool result with no `is_error` is classified from `tool_use_result`](37-claude-results-without-is-error.md)
- [38 - A Cursor run may read its own `cursor-data/`, so it can await a background task](38-a-run-reads-its-own-cursor-data.md)
- [39 - `hydra show` and `hydra reap` take a run-id prefix](39-run-id-prefix.md)
- [40 - Admission refuses the code rows harness 15.2 names](40-admission-refuses-the-code-rows-too.md)
- [41 - `rate_limit_abort` reads a field that is not there](41-the-rate-limit-abort-reads-a-field-that-is-not-there.md)
- [42 - A `hydra curate` run writes `command: curate`](42-a-curate-run-writes-command-curate.md)
- [43 - `propose()` deletes its own proposal branch](43-propose-deletes-its-own-proposal-branch.md)
- [44 - A run can rewrite the hook config Hydra wrote inside its worktree](44-a-run-can-rewrite-its-own-hook-config.md)
- [45 - `agy` cuts tool parameters at 513 characters](45-agy-truncates-tool-parameters-at-513.md)
- [46 - Parallel calls to one tool on `claude` leave the first `Ungated`](46-parallel-calls-to-one-tool-leave-one-ungated.md)
- [47 - Snapshot creation races when several runs launch on one store sha](47-snapshot-race-on-concurrent-launch.md)
- [48 - Codex CLI as the fourth provider](48-codex-as-the-fourth-provider.md)
- [49 - A gate whose code changed reads every older record as a leak](49-the-replay-cannot-tell-a-changed-gate-from-a-leak.md)
- [50 - The binary is not relocatable and nothing says so](50-the-binary-is-not-relocatable.md)
- [51 - The corpus walk ignores `HYDRA_ROOT`](51-the-corpus-walk-ignores-hydra-root.md)
- [52 - The demo scripts only work in one checkout](52-the-demo-scripts-only-work-in-one-checkout.md)
- [53 - Six contract sentences the code does not keep](53-six-contract-sentences-the-code-does-not-keep.md)
- [54 - The validator checks less than the contract claims](54-the-validator-checks-less-than-the-contract-claims.md)
- [55 - A finding means three different things](55-a-finding-means-three-different-things.md)
- [56 - The shell check only saw whole tokens](56-the-shell-check-only-saw-whole-tokens.md)
- [57 - A run can relaunch the binary and name its own policy](57-a-run-can-relaunch-hydra-and-name-its-own-policy.md)
- [58 - The floor's `git` parser eats the wrong token](58-the-floor-git-parser-eats-the-wrong-token.md)
- [59 - A shell with no command string, and an MCP path nobody checks](59-a-shell-with-no-command-string-and-an-mcp-path-nobody-checks.md)
- [60 - The compiler and the gate fold case differently](60-the-compiler-and-the-gate-fold-case-differently.md)

## Out of scope

Everything `docs/harness-spec.md` section 18 lists as a non-goal, unchanged: the TUI,
the raw-model plane beyond one gated `gemini` path, hermes, multi-machine use,
Langfuse, training, Cursor GUI automation, reading the user's credential files,
interpreting shell commands, ACP, and a mid-run input channel.

Rung two was out of scope for this file until rung one had run; build ticket 36 built it on
2026-09-08, and the tenth pass offered the first patch on 2026-09-21.
- [61 - Codex sits on the judge panel](61-codex-on-the-judge-panel.md)
- [62 - The self-improvement module is called rungs](62-ouroboros-becomes-rungs.md)
- [63 - The judge sees the task, the checklist in full, and the baseline test](63-the-judge-sees-the-task-and-the-baseline-test.md)
- [64 - A human asks for a fresh review](64-a-human-asks-for-a-fresh-review.md)
- [65 - Judge verdicts become preference pairs](65-judge-verdicts-become-preference-pairs.md)
- [66 - The spawn-config writer is a protected row](66-the-spawn-config-writer-is-a-protected-row.md)
- [67 - A foreign checkout refuses, and hydra doctor](67-a-foreign-checkout-refuses-and-hydra-doctor.md)
- [68 - The seatbelt around the child](68-the-seatbelt-around-the-child.md)
- [69 - The gate reads the paths out of a patch](69-the-gate-reads-the-paths-out-of-a-patch.md)
- [70 - Git's index lock under the seatbelt](70-git-index-lock-under-the-seatbelt.md)
- [71 - `hydra resume` on `codex`](71-codex-resume.md)
- [72 - A deadline on the pty tee](72-a-deadline-on-the-pty-tee.md)
- [73 - The gated interactive launch under the seatbelt](73-the-gated-interactive-launch-under-the-seatbelt.md)
- [74 - The checkout joins the deny list](74-the-checkout-joins-the-deny-list.md)
- [75 - Admission under the proposer's seatbelt](75-admission-under-the-proposers-seatbelt.md)
- [76 - The delegate face refuses inside the seatbelt](76-the-face-refuses-inside-the-seatbelt.md)
- [77 - The gate reads the payload it was compiled for](77-the-gate-reads-the-payload-it-was-compiled-for.md)
- [78 - A non-ASCII root is refused before launch](78-a-non-ascii-root-is-refused.md)
- [79 - The attack corpus in `tests/corpus`](79-the-attack-corpus.md)
- [80 - Baseline-test provenance](80-baseline-test-provenance.md)
- [81 - The cap is a reservation](81-the-cap-is-a-reservation.md)
- [82 - The panel type carries independence](82-the-panel-type-carries-independence.md)
- [83 - `run_start` takes a struct](83-run-start-takes-a-request.md)
- [84 - CI prints the corpus count](84-ci-prints-the-corpus-count.md)
- [85 - An export verifier of its own](85-an-export-verifier-of-its-own.md)
- [86 - One interpreter per replay](86-one-interpreter-per-replay.md)
- [87 - `codex` reads a copy of its skills](87-codex-reads-a-copy-of-its-skills.md)
