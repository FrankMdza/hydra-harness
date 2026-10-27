# Build 341 - `IMPLEMENTATION.md` closes the twelve remaining stale claims of build 313 and the measured 361 breakdown

Status: claimed
Size: haiku

## Goal

Update `IMPLEMENTATION.md` only so that the twelve remaining items from `.scratch/hydra/research/313-stale-claims.md` (`3, 4, 12, 19, 20, 21, 25, 30, 31, 48, 52, 53`), the `gpt-6-astra` to `gpt-6-luna` rename, and the measured `RejectedBeforeHook` breakdown from build 361 are accurate. Do not touch any file other than `IMPLEMENTATION.md` and this ticket file. Follow `~/.claude/skills/unslop/SKILL.md`: no em dashes in new prose.

## What to change in `IMPLEMENTATION.md`

1. **Lines 192 to 195 (research 313 item 3, `protected_worktree_paths`):**
   - Current text says `six files Hydra writes inside a worktree (`protected_worktree_paths`: hook and MCP configs for Cursor and agy, plus Claude's three project config files)`.
   - Replace so it says `nine entries in protected_worktree_paths in src/policy/mod.rs: the hook and MCP configs for Cursor, agy and Codex, plus Claude's three project config files, build ticket 48`.

2. **Lines 198 to 202 (research 313 item 4, `write_denied_worktree_paths`):**
   - Current text says `the three skill roots (`write_denied_worktree_paths`) are denied to writes and left readable by shell`.
   - Replace `the three skill roots` with `the four skill roots (`claude`, `cursor-agent`, `agy` and `codex`, build ticket 87)`.

3. **Lines 760 to 765 (research 313 item 12, `provider_native` count in `CLAUDE`):**
   - Current text says `Sixteen of the twenty-seven named rows in the table are provider_native` and `since build ticket 125 it is eighteen of twenty-nine`.
   - Replace the final clause so it says: `since build tickets 117 (`Agent`), 125 (`Glob` and `Grep`) and 350 (`Workflow`) it is nineteen of the thirty-three named rows in src/policy/table.rs::CLAUDE`.

4. **Lines 1745 to 1750 (research 313 item 19, Cursor unhooked tools):**
   - Current text says `The table holds three of them, on Cursor:` and lists `ReadLints`, `getMcpToolsToolCall` and `awaitToolCall`.
   - Update to say the Cursor table holds five unhooked tools: those three plus `WebSearch` (`webSearchToolCall`, build ticket 113) and `updateTodos` (`updateTodosToolCall`, build ticket 286).

5. **Lines 1832 to 1834 (research 313 item 20, `config/mcp.md`):**
   - Current text says `config/mcp.md declares no server on this machine, so both runs launched without --mcp-config and their MCP section is absent`.
   - Replace with: `Those two measured runs predated the shipped hydra server in config/mcp.md, admitted under permissive in build ticket 26 and widened to standard in map ticket 82, so both runs launched without --mcp-config and their MCP section is absent`.

6. **Lines 1968 to 1970 (research 313 item 21, `gate.py::check_dialect` vs `as_tool_call`):**
   - Current text says `Because the gate checks argv[1] against the payload's own shape (build ticket 77), an agy payload fed to a cursor script is refused as a malformed payload rather than read through the wrong normalizer`.
   - Replace with: `Because gate.py::check_dialect checks argv[1] against HYDRA_CONFIG["dialect"] while as_tool_call unwraps the payload shape (toolCall for agy, beforeReadFile for cursor, or tool_name and tool_input for claude, codex and the replay, build ticket 77), a wrong dialect argument is refused before the payload is normalized`.

7. **Line 2217 (`gpt-6-astra` rename):**
   - Replace `gpt-6-astra` with `gpt-6-luna`.

8. **Lines 2263 to 2266 (research 313 item 25, `skills` on `run_start`):**
   - Current text says `skills is present on every run with a checkout and absent on the one without`.
   - Replace with: `skills is present on every recorded non-judge run with a checkout and absent on the rest: judge skips skill injection since build ticket 25, and ungated interactive runs write no skills`.

9. **Lines 2355 to 2358 (build 361 measured `RejectedBeforeHook` breakdown):**
   - Replace `The machine-wide breakdown is not yet read: a build worktree cannot reach ~/hydra/runs/, so the driving session runs hydra doctor --walk to get it. The fixture test checks one Edit row, not the machine's corpus.` with:
     `Measured on 2026-10-27 by hydra doctor --walk over 997 records on this machine: 59 RejectedBeforeHook rows and 4 Ungated across twelve shapes: Bash: <tool_use_error>Blocked: 18, Read: InputValidationError: 17, Write: <tool_use_error>Error calling tool: 6, Edit: <tool_use_error>String to replace not found: 4, bash: No such tool available: 4, Edit: <tool_use_error>No changes to make: old_string and new_string are exactly the sa: 3, Edit: InputValidationError: 2, Edit: <tool_use_error>File does not exist: 1, TaskStop: <tool_use_error>No task found with ID: 1, WebFetch: InputValidationError: 1, Write: <tool_use_error>File has not been read yet: 1, and Write: safety classifier stopped call: 1.`

10. **Lines 2610 to 2611 (research 313 item 30, `gate_sha` omission):**
    - Current text says `**One place still writes gate_sha: null.** The harness record hydra evolve writes compiles no gate at all, and null is the truth there.`
    - Replace with: `**A launch that compiles no gate omits gate_sha.** RunStart::gate_sha carries skip_serializing_if = "Option::is_none", so the harness record hydra evolve writes and an ungated interactive run omit gate_sha rather than writing null.`

11. **Lines 2620 to 2625 (research 313 item 31, `Finding` variants):**
    - Current text says `Seven of them: a gate leak, subagents spawned under a policy that denies them, a disagreement between the derivation and the gate's recorded answer, a gated call the gate never answered, a tool name the capability table does not know, a policy_sha that is not today's, and a bare limit for anything else.`
    - Replace `Seven of them:` and add the two newer variants so it says: `Nine of them in src/normalize/mod.rs: the seven original variants (a gate leak, subagents spawned under a policy that denies them, a disagreement between the derivation and the gate's recorded answer, a gated call the gate never answered, a tool name the capability table does not know, a policy_sha that is not today's, and a bare limit for anything else) plus UnhookedDenied (an unhooked vendor tool whose capability the policy denies, build ticket 124) and RejectedBeforeHook (a tool call the provider rejected before any hook ran, build ticket 334).`

12. **Lines 5619 and 5624 to 5625 (research 313 item 48, `rungs/` branch prefix):**
    - At line 5619, replace `` `ouroboros/blocked/<pass-id>` `` with `` `rungs/blocked/<pass-id>` (formerly `ouroboros/blocked/<pass-id>` before build ticket 62) ``.
    - At lines 5624 to 5625, replace ``on `ouroboros/<pass-id>` when the panel passed and `ouroboros/blocked/<pass-id>` when it did not`` with ``on `rungs/<pass-id>` when the panel passed and `rungs/blocked/<pass-id>` when it did not (since build ticket 62; readers still accept the older `ouroboros/` branch names)``.

13. **Lines 5933 to 5936 (research 313 item 52, cap refusal and UUID minting):**
    - Current text says `so the refusal goes to stderr and no run id is minted.`
    - Replace with: `so the refusal goes to stderr and writes no run record, although reserve_on_layout in src/run/mod.rs has already minted the UUID it tried to reserve, build ticket 92.`

14. **Lines 5992 to 5993 (research 313 item 53, `PATH` without `/usr/bin`):**
    - Current text says `**A PATH without /usr/bin refuses the launch** before a run id exists, with an error naming the silent logout it would otherwise cause. It writes no run record:`
    - Replace with: `**A PATH without /usr/bin refuses the recorded launch** when ChildEnv::build runs in prepare_machine, after reserve_on_layout has minted an id and before a record is written, build ticket 92, with an error naming the silent logout it would otherwise cause. It writes no run record:`

## Done when

- Only `IMPLEMENTATION.md` and `.scratch/hydra/build/341-implementation-md-twelve-remaining-stale-claims.md` are modified.
- `git diff --check` passes.
