# 242 - A refusal names the path the agent asked for

Type: build
Status: claimed
Blocked by:

## What

Usability probe of 2026-10-02, run `01a0fe73-5a9c-7c81-aff3-9726beb4ea60` (`claude` sonnet,
`standard`). The refusal read:

`hydra/out-of-scope: policy standard allows file_write inside worktree and /users/user/prueba-fuera-2.txt is outside it.`

The agent asked for `/Users/USER/prueba-fuera-2.txt`. The gate folds ASCII case before it
compares, because the volume does not distinguish case (map ticket 68, H4), and prints the folded
path. The same shape is on run `01a0fe11-9cb2` in `~/hydra-oficina/runs/` for a `Read` of
`~/.ssh/config`. A tester read it as the gate looking at another file.

## Do

1. The gate keeps comparing folded and prints the path as the tool call gave it. Find every
   refusal string that interpolates a path (`policies/` is protected and is not touched; the
   compiled script comes from `src/policy/`).
2. `tests/gate.rs` pins refusal text. Change the pinned strings that carry a path, and add one
   case with an upper-case component that asserts the message carries it unfolded while the
   decision is unchanged.
3. Verify by reverting: with the fix off, the new case fails.

## Done when

- The new case passes and fails with the fix reverted; fmt, clippy and `cargo test` pass;
  `hydra doctor --walk` passes.
- `gate_sha` changes, since the script does. Say so in the Outcome and in `IMPLEMENTATION.md`.
