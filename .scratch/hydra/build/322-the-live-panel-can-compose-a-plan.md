# 322 - The live panel's choose prompt asks for the task array, so a `composed` verdict can be written at all

Type: build
Status: open
Blocked by:
Opened: 2026-10-15, by the session, from map 122's answer. The inline choose prompt in
`crates/puppets/src/panel.rs` asks each seat for one JSON object with `chosen` and `findings`
and "nothing else"; `plan_panel_verdict` (around line 604) then refuses a `composed` verdict that
carries no fenced task array (`composed_plan`, line 555). The prompt that does ask for the
array, `integrations/puppets/prompts/choose.md`, is read by nothing: `launch.rs` reads `plan.md`
and `stances/` from the prompts directory and no other file (lines 420 and 433). So a panel
composition today needs every seat to say `composed` and one of them to disobey "nothing else".
No panel has ever composed; `ea69da1f`'s `composed` verdict was the session's file.

## What

One prompt, read from one place. Either `launch.rs` reads `prompts/choose.md` the way it reads
`plan.md`, or the inline prompt gains the paragraph that offers `composed` and says what to
attach: after the JSON object, a fenced ```json block holding the task array of the composed
plan, in the shape `prompts/plan.md` defines. Prefer reading the file, since it exists and is
the one a human edits; if its text and the inline prompt disagree, the file wins and the inline
copy goes. Do not delete `choose.md` in either case; if it ends up unread, say so in the Outcome
and leave it for the human.

The puppets prompts and the goal log are not protected config (harness spec 15.1 names six and
these are not among them), so this is a build.

## Do

1. Read `panel.rs` lines 540 to 660 and `launch.rs` around 400 to 450, and `prompts/choose.md`.
2. The prompt change; `plan_panel_verdict` unchanged in what it accepts.
3. `crates/puppets/tests/`: a case where every seat answers `composed` and one attaches a fenced
   array, and the verdict is written with that plan; a case where no seat attaches one, and the
   refusal says what was missing.

## Done means

The two cases green, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo nextest run --release --workspace`, `cargo test --release --workspace --doc`.
Amend `IMPLEMENTATION.md` where the panel's choose step is described.

Rules inside the run: relative paths only, never look outside your own worktree, write files with
your file tool and not with shell heredocs, and the test suite is green inside a run since build
302 without any `env -u`.
