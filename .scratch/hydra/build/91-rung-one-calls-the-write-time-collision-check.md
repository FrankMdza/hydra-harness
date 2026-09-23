# 91 - Rung one calls the write-time collision check

Type: build
Status: open
Blocked by:

## What

Finding G23 of the second hiring panel, 2026-09-23, raised by `cursor-agent`; the human chose to
wire it on 2026-09-23. `skills::admit` (`src/skills/mod.rs` ~475) is harness spec 6.4's write-time
check, and its doc comment says nothing calls it, "a gap rather than a decision". The caller it
names is rung one's step that turns a curator's writes into a store commit. That step is
`curate::propose` (`src/rungs/curate.rs` ~253): it reads the checkout's `HEAD` as `started_at`, then
`commit_everything`s whatever the curator wrote.

Before that commit, list the skill directories the curator added or renamed relative to
`started_at` (a skill is a directory with a `SKILL.md`; `git diff --name-status` against
`started_at` over the working tree will show them) and call `admit` for each one against the user's
index. If any is refused:

- commit nothing and remove nothing, so the curator's work stays in its checkout for the human;
- say which skill and which root holds the name, in `Proposal.left` or a new field on `Proposal`,
  your call, and make `hydra curate` print it.

A skill the curator only edited, whose name was already in the store, is not re-checked: the launch
check already covers names the store holds.

Remove "Nothing in this build calls it" from the doc comment and say where it is called.

## Done when

- `tests/curate.rs`: a throwaway store, a scratch `HOME` whose user skill root holds `foo`, and a
  curator checkout that adds `foo/SKILL.md`: `propose` commits nothing, keeps the checkout, and
  names the collision. The same with `bar` commits as before.
- `tests/skills.rs` unit tests of `admit` still pass.
- `IMPLEMENTATION.md` no longer says the check has no caller.

## Notes for the run

Tests take a scratch `Layout` and a scratch `HOME`, never `Layout::discover()` or the real home.
Ignore `no_record_on_this_machine_shows_a_gate_that_leaked_or_stayed_silent`. Run
`cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` before you stop.
