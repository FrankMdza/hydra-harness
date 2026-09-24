# 110 - `~/hydra/puppets/` joins the deny list and the seatbelt profile

Type: build
Status: open
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
