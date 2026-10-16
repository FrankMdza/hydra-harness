# 348 - A relative path in an MCP argument is resolved against the run's cwd before the protected-path check, so `mcp__writer__put` with `../runs/x.jsonl` is refused like the absolute spelling

Type: build
Status: resolved
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

## Outcome

**Built.** `src/policy/gate.py` has a second pass for `mcp_call`, `protected_path_resolved_in`,
which runs after the raw pass. Each string that contains `/`, or starts with `.` or `~`, resolves
the way a shell token does: `path_candidates`, then `resolved_pair` against the cwd, then
`protected_root`, with `link_crossed` for a symlink. A refusal names the resolved path and any
link. `strings_in` walks the arguments at any depth. The `network_read` branch is unchanged.

**A choice the ticket did not spell out.** The pass also tries each whitespace-separated word of a
string. Without that, `"copy the log from ../../runs/x.jsonl"` walks through, because the whole
string resolves to a path under the cwd. The shell pass gets the same effect by tokenizing.

**Docs.** The docstring of `protected_path_named_in` now says the raw pass sees a path only as
written. The `mcp_call` comment in `evaluate` names the second pass. IMPLEMENTATION.md has a
paragraph after "`mcp_call` met neither door". SECURITY.md has a new bullet after the shell
bullet, which says what still walks through: two arguments that join into one path, a path a
server assembles itself, and a relative path a server resolves against a cwd other than the hook's.

**Tests, `tests/gate.rs`.**
- `an_mcp_call_naming_a_protected_path_relatively_is_refused_under_permissive` (new): the corpus
  as `../../runs/x.jsonl` at the top level, two levels deep, and inside a sentence. The reason
  names the resolved corpus.
- `an_mcp_call_through_a_link_in_the_worktree_is_refused_under_permissive` (new): `notes/x.jsonl`,
  where `notes` links to the corpus. The reason says "through the link".
- `an_mcp_call_whose_relative_path_stays_in_the_worktree_is_allowed_under_permissive` (new):
  `notes/../src/main.rs`, `runs`, and a sentence with `src/lib.rs` and `./Cargo.toml`.
- `a_protected_path_split_or_assembled_still_walks_past`: the MCP `//` case moved to the denied
  side. The two-field case stays allowed, with its comment rewritten.

**Red first.** Before the pass, each new test failed at the assertion the change should flip. The relative case
reported `{"path":"../../runs/x.jsonl"} walked through: hydra/allow: policy permissive allows
mcp_call.` The split test failed on the `//` spelling. Afterward `tests/gate.rs` is 133 of 133.

**Case 1 as the ticket spells it,** `{"path": "../runs/x.jsonl"}` from the worktree, verbatim:

> hydra/protected-path: an argument of mcp__writer__put names /private/var/folders/p7/gp62r5wd75ndqg6cdz2zc4sm0000gn/t/hydra-gate-24841-permissive-0/worktrees, denied under every policy by harness spec 7.5. `../runs/x.jsonl` resolves to /private/var/folders/p7/gp62r5wd75ndqg6cdz2zc4sm0000gn/T/hydra-gate-24841-permissive-0/worktrees/runs/x.jsonl.

**Measured, latency.** `cargo nextest run --release --test gate --no-fail-fast`, three runs each.
With the pass switched off, the three new cases red: 17.4 s, 12.5 s, 14.3 s. With it on, 133 green:
10.2 s, 10.4 s, 10.1 s. The off runs are slower on average, but they carry three failing tests
and the first run was noisy, so the gap is not a measured cost of the pass. I did not isolate the
cost of the `realpath` calls, and I claim no per-call number.

**Not measured: the live walk.** Step 4 is `hydra doctor --walk`, which reads the corpus under
`~/hydra`, outside this worktree. The rules for this run forbid looking outside the working
directory, so I did not run it. The count of recorded MCP calls with a relative path is still
unknown. It needs a run from outside this session.

**What the ticket assumed that was false.**
1. Case 1's spelling `../runs/x.jsonl` does not reach the corpus. From `worktrees/<run>` it lands
   in `worktrees/runs/`. It is refused because `worktrees` is a denied root, and the refusal names
   `worktrees`. The corpus from a worktree is `../../runs/x.jsonl`, which the tests use.
2. Case 2's `../../hydra/policies/standard.json` lands in `~/hydra/hydra/policies/`. The policies
   directory is `../../policies/standard.json` from a worktree. I worked this out from the layout
   and did not run it.
3. The "first three red today" claim holds only loosely. Each new test stops at its first failing
   assertion, so I did not see the later cases red on the old code.
4. The raw-pass docstring said a relative MCP path "walks through, which is the same residual the
   raw shell pass has". That was accurate before this change and is now false, so I rewrote it.

**Measured by the session on master, 2026-10-15:** `hydra doctor --walk` over 976 records raised no new finding from the resolved pass; the suite is 1460 tests, all green, with builds 339 and 349 applied in the same working tree.
