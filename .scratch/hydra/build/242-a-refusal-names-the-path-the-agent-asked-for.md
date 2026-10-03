# 242 - A refusal names the path the agent asked for

Type: build
Status: resolved
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

## Outcome

Resolved 2026-10-03 by run `01a1018f-20c1` (`claude` opus 5.5, `standard`, 1.36M tokens),
applied by the session unchanged.

Built: `src/policy/gate.py` compares the ASCII-folded path as before and prints the resolved path
with its case kept. `resolve` is now `fold_ascii(resolve_shown(...))`; `target_path`,
`target_paths`, `parse_patch_paths` and `paths_from_changes` return `(folded, shown)` pairs and
dedupe on the folded one. Three refusals print the shown spelling: the 7.5 protected-path one of a
file tool, the 6.4 skills-root one, and `hydra/out-of-scope`. The refusals that print a configured
root (shell, `mcp_call`, `network_read`) are unchanged. Test:
`a_refusal_prints_the_path_in_the_case_the_call_gave` in `tests/gate.rs`.

Measured by the session on master: with `gate.py` reverted the new test fails with
`.../gn/t/prueba-fuera-242.txt is outside it` against the expected `.../gn/T/Prueba-Fuera-242.TXT`;
restored, it passes. `cargo test --test gate` 113 of 113 (the `Straße` case the run saw fail in its
worktree passes on master), `--test spawn` 36 of 36, `--lib` 224 of 224, fmt and clippy clean.

What the ticket assumed that was false: no test pinned refusal text carrying a path, so nothing
pinned changed. "As the tool call gave it" means resolved (absolute, symlinks followed) with the
case kept, so `/var/...` still prints as `/private/var/...`; printing the raw argument instead is
a small change if wanted.

Not done: `target/release/hydra` embeds `gate.py` at compile time (`src/policy/mod.rs:1181`), so
runs keep the old text until the next `cargo build --release`. `hydra doctor --walk` with the
release binary of 2026-10-01, which still carries the old gate, reports a gate finding on run
`01a0fd01-cc30` (five `Bash` calls whose replay says `hydra/protected-path` for
`/users/user/hydra`); that run predates this ticket and the finding does not come from it.
