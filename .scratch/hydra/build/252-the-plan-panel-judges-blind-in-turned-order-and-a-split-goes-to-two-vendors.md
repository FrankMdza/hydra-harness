# 252 - The plan panel judges blind, in turned order, and a split goes to two vendors

Type: build
Status: resolved
Blocked by:

## What

Decided by the human on 2026-10-03, all three at once ("1, 2 y 3 juntos dan mayor robustez").
Reversible by the human alone.

`choose --judge` in `crates/puppets/src/panel.rs` (`choose_judge`, `build_choose_seat_prompt`,
`plan_panel_verdict`) shows every seat the proposals under headings that name the proposer's
provider (`### claude`), in the same order for every seat, and asks for a provider name back. A
verdict is recorded only when all three seats agree; otherwise there is none and the human picks.
Three known judge biases act on that: a judge prefers a proposal it can tell came from its own
vendor or style, prefers the one it reads first, and prefers the longer one. The first two are in
our hands.

1. **Blind.** Proposals are shown under neutral labels (`### Proposal A`, `B`, ...), and a seat
   answers with a label or `composed`. The map from label to provider lives only in the helper and
   in the `panel_verdict` event, never in a prompt. Strip the provider's name from the heading
   only; a plan that names its own author in its text stays as written, and the report says so.
2. **Turned order.** Each seat reads the proposals in a different order: seat n starts at proposal
   n and wraps. Labels follow the proposal, not the position, so an answer means the same thing
   whatever order the seat read.
3. **A split goes to two judges of two vendors.** When the three seats do not all agree, two
   tie-break runs follow: `claude` with model `claude-fable-5-1` and `codex` with model
   `gpt-6-astra`, both under `judge`, both blind, the second reading the proposals in the reverse
   order of the first. If both name the same label, that is the verdict, and the event says it
   came from the tie-break. If they differ, there is no verdict, as today, and the human picks.
   A provider that `vendors.json` marks down follows the same degraded-approval rule
   `resolve_panel_seats` applies to the seats.

## Do

1. Change `choose_judge` and `build_choose_seat_prompt` for 1 and 2; parse the label back to the
   provider before `plan_panel_verdict` sees it, so that function and the events keep their shape.
2. Add the tie-break for 3 through `run_judge_panel`, so each tie-break is one `hydra run` waited
   for like a seat. Record in the `panel_verdict` event: the label map, each seat's order, and the
   tie-break run ids and answers when there were any.
3. `print_judges` prints the tie-break lines after the seats' lines, in the same shape.
4. Unit tests in `panel.rs`: no provider name appears in any seat prompt; the three seats get
   three different orders; a label answer maps back to its provider; a 2-1 split with both
   tie-breaks agreeing records a verdict; with them disagreeing records none. Each test fails
   with its part of the change reverted; say so.
5. `IMPLEMENTATION.md`: one paragraph where `choose --judge` is described (`grep -n 'choose --judge'`).

`review` (one diff, no proposals to order) is out of scope.

## Done when

- fmt, clippy over the workspace and `cargo test -p hydra-puppets` are clean, and no test
  launches a provider.

## Outcome

Resolved 2026-10-03 by run `01a10389-ec1d` (`codex` gpt-6.1-sol, `standard`, 2.28M tokens).
The session added one change to the agent's diff and verified one test by breaking it.

Built, in `crates/puppets/src/panel.rs`: proposals go to the seats as `Proposal A`, `B`, and so
on; seat n reads them starting at proposal n and wrapping; an answer names a label, and
`parse_blind_answer` maps it back to the provider before `plan_panel_verdict` sees it. On a split,
`TIE_BREAK` runs `claude` `claude-fable-5-1` and `codex` `gpt-6-astra` under `judge`, the second
in reverse order, through `run_judge_panel`; their agreement is the verdict, otherwise there is
none. A down vendor goes through the same degraded approval as a seat, and a substitute stays
under `judge`. `panel_verdict` gains `verdict_source` (`panel`, `tie_break` or null), `label_map`,
`seat_orders`, `tie_break_orders`, `tie_break_run_ids` and `tie_break_answers`. Five unit tests;
the agent reverted each part and saw its test fail. The session repeated that for one: with the
rotation removed, `blind_seats_read_three_rotations_with_stable_labels` fails, and restored it
passes.

What the session changed: `the_launch_commands_match_puppets_py_and_hand_the_stubs_the_same_calls`
in `crates/puppets/tests/differential.rs` failed on all twelve `choose --judge` cases, since the
prompts and the stub calls now differ from what `puppets.py` recorded. Those cases are no longer
compared (`DIVERGED_SINCE_252`); they still run so the fixture count holds, and the test asserts
at least one was found. Every other launch case is still compared. The agent's report said this
needed an edit outside its permitted files, which is right.

Assumed and false: three distinct orders need three proposals. With two, seat three reads seat
one's order. Plan bodies are not rewritten, so a plan that names its author still does.
