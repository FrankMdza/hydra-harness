# 110 - `~/hydra/puppets/` joins the deny list and the seatbelt profile

Type: build
Status: resolved
Blocked by: 108

## What

Map ticket 76, decision B: the goal log lives under `~/hydra/puppets/` and no gated run may
write it, because a run that can rewrite its own assignment or approval walks around the mode.
`denied_write_paths` in `src/policy/mod.rs` is the list every policy compiles into its gate, and
`src/sandbox.rs` composes the seatbelt profile from the same paths. Add `layout.puppets()` to
`src/paths.rs` and to that list, so the gate refuses a write there under every policy and the
seatbelt denies it on macOS. Reads stay as they are: the log is data a run may read if its scope
reaches it, and under `standard` it does not.

## Done when

A test beside `tests/spawn.rs::the_seatbelt_refuses_a_checkout_write_and_allows_the_worktree`
compiles a `standard` gate for a scratch `Layout` (never `Layout::discover()`), feeds it a
`Write` payload under `<layout>/puppets/x.jsonl` and reads the refusal back, and the profile the
same spawn writes names the directory under its deny rules. `IMPLEMENTATION.md`'s deny-list
paragraph names the new entry. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
and the suite green.

## Outcome

Built 2026-09-24 by composer-2.5 on run 01a0d41b-c2cf (1.26M tokens): `Layout::puppets()`,
the entry in `denied_write_paths`, one test that compiles a `standard` gate for a scratch
layout, sends a `Write` under `<layout>/puppets/` through the real hook and reads
`hydra/protected-path` back, then checks the seatbelt profile names the directory. No change to
`src/sandbox.rs` was needed: the profile already takes its write denies from the same list, which
the ticket had assumed it did not. Applied as written. The agent's report named "pre-existing
failures" in `run::tests` and the git-parent spawn tests under a full parallel run; on master
after applying, `cargo test --test spawn` passed 34 of 34 and `run::tests` 6 of 6, so those were
the worktree's, not the tree's. fmt and clippy clean.
