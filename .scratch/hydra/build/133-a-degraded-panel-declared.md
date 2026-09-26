# 133 - The mode's judge panel can sit degraded, declared, with a role and evidence slice per seat

Type: build
Status: resolved
Blocked by:

## What

Map ticket 81-a, resolved 2026-09-26. Read its `## Answer` and research/81 section 3 first.

`PUPPETS_PANEL_SEATS` in `integrations/puppets/puppets.py` near line 39 is three fixed seats,
`claude` opus under `judge`, `codex` under `standard`, `cursor-agent` composer-2.5 under
`judge`. `run_judge_panel` near line 818 launches each with the same prompt. When codex is out of
quota the panel cannot sit, and the skill (`.claude/skills/puppets/SKILL.md` step 3) falls back
to the session choosing alone. The answer to 81-a: every seat gets a distinct role and evidence
slice, and when a seat's provider is out the human may approve a degraded panel for that goal.

## Do

1. A seat becomes provider, model, policy, **role** and **evidence**. Three roles to start, one
   per seat: `reviewer` (does the diff do what the task asked), `adversary` (argue the diff should
   be rejected; name the strongest reason), `security` (the checklist items about secrets, paths,
   gates and the record). Evidence slices: `diff`, `diff+tests`, `diff+task`. The prompt each seat
   gets names its role and holds only its slice. The closed answer shape `parse_judge_answer`
   reads does not change.
2. `approve <goal> --degraded <missing-provider>=<substitute-provider[:model]>` records the human's
   approval in the log, bound to the current plan revision like every other approval. Without it,
   a panel with a provider out refuses as it does today and names the command.
3. A degraded panel runs the substitute in the missing seat's role and slice. Its verdict event
   in the log carries `degraded: true` and the substitution; `board` prints the verdict as
   degraded; nothing reports it as a full panel.
4. Rung two's panel (`src/rungs/review.rs`) does not change. Say so in a comment where the seat
   table is defined.
5. The skill's step 3: replace "If codex is out of credits the panel cannot run; then read the
   plans yourself" with the degraded path, and keep "say in your message that the panel did not
   sit" for when the human declines it. `integrations/puppets/README.md`: the paragraph on the
   panel says what degraded means.

This is the other session's territory. Check `git log -3 -- integrations/puppets/` first.

## Done when

- Tests under `integrations/puppets/tests/` with a stub `hydra`: a full panel builds three
  prompts with three different roles and slices; a panel with codex marked out refuses without
  the approval; with it, the substitute runs in codex's role and the verdict event carries
  `degraded: true`; an approval from an earlier plan revision does not count.
- The puppets suite passes (`python3 -m unittest discover integrations/puppets/tests`).

## Outcome

Resolved 2026-09-26. Built by `composer-2.5` on run 01a0ded9-8e70 under `standard`, about 1.81M
tokens. The session applied the diff with two fixes and one test repair, and wrote the skill step.

A seat is a `PanelSeat`: provider, model, policy, role and evidence. The three roles are
`reviewer` (evidence `diff+task`), `adversary` (`diff`) and `security` (`diff+tests`), and
`build_diff_panel_prompts` and `build_choose_panel_prompts` give each seat only its role and its
slice. `parse_judge_answer`'s shape is unchanged. A provider is out when its row in
`vendors.json` says `"available": false`. A panel with a seat out refuses and names `approve <goal>
--degraded <missing>=<substitute[:model]>`, which records a revision-bound approval. With it, the
substitute runs in the missing seat's role and slice, and the `panel_verdict` and `review` events
carry `degraded: true` and the substitutions. `board` labels them degraded. A comment at the seat
table says rung two's panel in `src/rungs/review.rs` does not substitute.

The session's fixes. First, the substitute inherited the missing seat's policy, so composer
standing in for codex would have judged under codex's `standard`, with write and network access,
where its own seat runs `judge`. It now takes its own seat's policy, or `judge` if it has none. The
degraded test asserts `judge`, and with the old line it fails with `'standard' != 'judge'`. Second,
the stand-in's answer was parsed under the missing provider's name; it is now parsed under the
provider that wrote it. The test repair: `test_panel_degraded.py` called `cmd_review` in process
with `execvp` stubbed, so `cmd_review`'s `chdir` into a temporary checkout stayed behind, and the
next test file failed on `getcwd`. The test now restores the working directory.

The skill: the agent could not write under `.claude/skills`, because the gate denies it from a
run, which is harness spec 6.4 working. The session rewrote step 3: mark a provider out in
`vendors.json`, offer the degraded panel through AskUserQuestion, and fall back to choosing alone
and saying so. On master the puppets suite passes, 118 tests.
