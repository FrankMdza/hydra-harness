# 91 - Rung one calls the write-time collision check

Type: build
Status: resolved
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

## Outcome

Built by `claude` on Opus 5.5 under `standard`, run 01a0cfa1-2185, 1.5M tokens, then fixed by the
session after an Opus review. `curate::propose` lists the checkout's top-level skill directories
whose `SKILL.md` the store did not hold, calls `skills::admit` on each against the user index, and
on any refusal commits nothing, removes nothing and returns `Proposal::refused`, which `hydra
curate` prints. The listing walks the working tree because `git diff --name-status` never shows an
untracked directory, measured by the agent. The review found the new `cat-file` check ran in the
curator's checkout, whose git metadata the curator can write; the session moved it to the store,
against the proposal branch's own ref. The old `rev-parse HEAD` in the checkout stays for build
ticket 95. It also noted that `admit` validates the frontmatter first, so a malformed new skill is
refused at write too; the wording now says "a proposed skill" and `IMPLEMENTATION.md` says so.
Measured on master: with the `admit` call replaced by a no-op,
`a_proposed_skill_whose_name_a_user_root_holds_is_not_committed` fails; with it, curate 22 and
skills 27 pass. fmt and clippy clean.
