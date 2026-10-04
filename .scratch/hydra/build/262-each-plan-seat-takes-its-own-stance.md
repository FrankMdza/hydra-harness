# 262 - Each plan seat takes its own stance

Type: build
Status: resolved
Blocked by:

## What

Decided by the human on 2026-10-03: the planning seats should differ in more than their model.
Today `hydra-puppets plan <goal> --seat ...` (`crates/puppets/src/launch.rs`, `plan`) hands every
seat the same brief, `integrations/puppets/prompts/plan.md` with `{{GOAL}}` filled, so three
vendors answer the same question from the same angle. He also wants a `claude` seat on planning,
on a model other than the orchestrating session's, so the orchestrator's own model does not
plan; the skill rule "keep `claude` off planning" was about the session's quota, not about
independence.

1. **Stances.** A seat may name one: `--seat provider[:model][@stance]`. Four stances, each a
   short file under `integrations/puppets/prompts/stances/` that is appended to the brief:
   - `minimal`: the smallest change that meets the goal; say what you deliberately leave out.
   - `risk`: start from what can break, and plan verification before implementation.
   - `contrarian`: question the goal's framing first; propose the plan you would defend if the
     obvious one were wrong, and say why.
   - `builder`: the fastest path to something running end to end, then harden.
   A seat with no stance gets one by its position, in that order, so three plain seats get three
   different stances and four get all four. Two seats never share a stance unless there are more
   than four. Each stance file ends with: "You will not see the other proposals; do not hedge
   toward a middle you imagine they take."
2. **Record.** `plan_proposed` gains `stance`. `plans` and `choose` keep working on older logs
   that have none.
3. **The judges stay blind to stance.** `choose --judge` (build 252) shows proposals as
   `Proposal A`, `B`, ... and must not print the stance in the seat prompts either; the
   `panel_verdict` event may record it beside `label_map`.
4. Unit tests: three plain seats get three different stances in order; `@risk` overrides
   position; an unknown stance is refused with the four names; the brief a seat receives contains
   its stance file's text and not another's; no seat prompt of `choose --judge` contains a stance
   name. Each fails with its change reverted; say so.

The `puppets` skill's text lives under `.claude/skills/`, which a gated run cannot write since
build 257; the session edits it. Say in the report what it should say.

## Done when

- fmt, clippy over the workspace and `cargo test -p hydra-puppets` are clean; no test launches a
  provider. `IMPLEMENTATION.md`: one paragraph.

## Outcome

Resolved 2026-10-03 by run `01a1044a-13b1-7391` (`codex` gpt-6.1-sol, `standard`, 1.33M tokens),
applied with one curly quote straightened in `contrarian.md`.

Built: `--seat provider[:model][@stance]`; four stance files under
`integrations/puppets/prompts/stances/`, appended to the brief; plain seats take the first stances
no named seat took, in the order `minimal`, `risk`, `contrarian`, `builder`; an unknown stance is
refused with the four names, and two seats naming the same one are refused when there are four
seats or fewer. `plan_proposed` carries `stance`, and older logs fold without it. The
differential test no longer compares `plan:` cases (`DIVERGED_SINCE_262`). Five unit tests, each
failing with its part reverted; the judge-prompt test passed before and after, since `choose
--judge` never printed a stance.

The session edited the `puppets` skill, which a gated run cannot write since build 257: step 2
now seats `claude` on a model other than the orchestrator's and names the stances. A proposal's
own text can still name its stance or author; the judges see it as written.
