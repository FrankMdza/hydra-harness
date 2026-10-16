# 348 - A relative path in an MCP argument is resolved against the run's cwd before the protected-path check, so `mcp__writer__put` with `../runs/x.jsonl` is refused like the absolute spelling

Type: build
Status: open
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from the grok seat of the plan of that night
(`.scratch/hydra/research/133-seat-grok-security-audit.md`). `src/policy/gate.py::protected_path_named_in`
walks an MCP call's arguments and searches each string, case-folded, for a denied spelling. It
resolves nothing, and its docstring says so: "a relative path in an MCP argument walks through,
which is the same residual the raw shell pass has". The shell check has a second pass,
`protected_path_named_by`, that resolves each token against the cwd and catches `..`; the MCP
check has no second pass. IMPLEMENTATION.md lines 253 to 259 describe the MCP door as closed and
SECURITY.md lines 21 to 24 limit the residual to a shell token, so neither document is right.

## What

A second pass for MCP arguments: every string that looks like a path, contains `/` or starts with
`.` or `~`, is resolved the way the shell token pass resolves a token (`os.path.realpath` against
the worktree cwd, the folded comparison, `link_crossed` for a symlink) and refused with
`hydra/protected-path` and the resolution facts when it lands under a denied root. Strings that are
not paths are left to the raw pass as today. The docstring, IMPLEMENTATION.md's paragraph and
SECURITY.md's residual sentence change in the same commit.

## Contract

Input: a hook payload for an `mcp__<server>__<tool>` call under a policy that grants `mcp_call`,
with arguments of any shape. Output: the decision and reason. Latency: the gate's own; the Outcome
measures `tests/gate.rs` before and after, since `realpath` is a syscall per string.

## Cases

In `tests/gate.rs`, beside `an_mcp_call_naming_a_protected_path_is_refused_under_permissive`,
written first, the first three red today:

1. `mcp__writer__put` under `permissive` with `{"path": "../runs/x.jsonl"}` from a worktree cwd is
   refused `hydra/protected-path`, and the reason names the resolved path.
2. The same with the path two levels deep in the arguments, `{"files": [{"to": "../../hydra/policies/standard.json"}]}`.
3. The same through a symlink planted in the worktree pointing at `~/hydra/runs`.
4. `{"path": "notes/../src/main.rs"}`, inside the worktree, is allowed.
5. A string with no `/`, `"runs"`, is allowed, as today.
6. `a_protected_path_split_or_assembled_still_walks_past` keeps its shell cases and loses the MCP
   `//` case if the resolution now catches it; say which in the Outcome.

## Do

1. The cases, red.
2. The pass, the cases green, the suite green.
3. The docstring, IMPLEMENTATION.md, SECURITY.md.
4. `hydra doctor --walk` on this machine: how many recorded MCP calls carry a relative path, in
   the Outcome.

## Done means

The four checks green and the Outcome quotes case 1's refusal reason.
