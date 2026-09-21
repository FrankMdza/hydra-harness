# 48 - Codex CLI as the fourth provider

Type: build
Status: resolved
Blocked by:

## What

Map ticket 58 measured `codex-cli 0.154.0` and found a `PreToolUse` hook in Claude's dialect,
a JSONL stream under `codex exec --json`, and ChatGPT-account auth. Route it the way the other
three are routed. The pieces, in the order the other providers grew them:

1. `src/provider/codex.rs`: argv for a recorded run (`exec --json`, `--sandbox` and
   `--ask-for-approval never` as belt to the gate, `--dangerously-bypass-hook-trust` because a
   per-spawn hook is never trusted, `-c` overrides to point the hook at Hydra's script), what it
   reads from the worktree, its version probe, and its auth pre-flight (`codex login status`).
2. `src/policy/table.rs`: rows for `Bash`, `apply_patch`, `mcp__<server>__<tool>`,
   `update_plan`, `spawn_agent`; hosted tools such as web search never reach the hook and want
   an `unhooked` row (map ticket 51's kind). The gate's `claude` dialect answers Codex; the
   config writer registers the hook in the shape `docs/hooks` gives.
3. `src/normalize/codex.rs`: `item.*` and `turn.*` events to the contract's tool_call,
   tool_result, message, thinking and usage; outcome derived the way contract 8 says.
4. A decision this ticket must not take alone: Codex has no read tool, so under `read-only` and
   `judge` every read is a shell command. Either those policies grant a bounded shell read on
   Codex only, or Codex is routable under `standard` and `permissive` first. Open a map ticket
   if the second reads wrong.
5. Measure on a logged-in binary before any of this lands: whether a hook crash or timeout
   fails open, and whether `-c hooks...` on the command line is honoured as a hook source.
6. `tests/drift.rs` gains a Codex probe that skips without the binary; `docs/harness-spec.md`
   sections 6.6 and 17 and `IMPLEMENTATION.md` say what is true.

Needs a human logged in first: `codex login` on any ChatGPT plan. Map ticket 58 has the
measurements and their sources.

## Outcome

Resolved 2026-09-21. Items 1, 2, 3 and the tests half of 6 built on 2026-09-20 by `cursor-agent` on
Composer 2.5, run 01a0bcdb-e6ed under `standard`, 9.4M tokens, 14 files, 1365 insertions, applied
to master after a mechanism read. Item 4 decided by the human on 2026-09-20: `standard` and
`permissive` only, the bounded shell read is map ticket 63. Item 5 measured on 2026-09-21 by the
orchestrating session on a logged-in binary, four recorded runs and ten direct probes, and the
docs half of 6 is the `codex` section of `IMPLEMENTATION.md`, which holds every measurement.

**Three things the ticket assumed that turned out false, each of which changed the code.**

1. `exec` has no `--ask-for-approval` flag; it is the interactive command's. Runs 01a0c220-909f
   and 01a0c220-ce0e died on it with exit 2. The belt is `-c approval_policy="never"` now.
2. A `<repo>/.codex/hooks.json` is not read inside a git worktree, from the worktree or from its
   main checkout; probes p7 and p8 in a scratch repo showed it, and run 01a0c223-9ec2 ran a
   protected-path read with the file in place and the gate silent. The `-c hooks.PreToolUse`
   override is read in a worktree but cannot hold the multi-line inline command (run
   01a0c222-4305, exit 1 at config load), so the Python goes to `spawn/<id>/hook.py` and the
   override names the file. Run 01a0c231-6583 measured the gate refusing both a patch and a
   protected-path read through that route.
3. `apply_patch` is not a pathed write. The hook sees the patch text, the stream sees
   `file_change` with `changes[].path`, and under `standard` the gate refuses it as
   `hydra/no-path`. A `standard` run writes through the shell or not at all until `gate.py`
   parses the patch header, which nobody has asked for.

**Measured and now in the reader.** A denied call leaves no `item.*` frame, only a stderr line
`Command blocked by PreToolUse hook: <reason>. Command: <command>`; the normalizer reads it into
a call, an error result and a decision. A hook that exits non-zero without JSON fails open
(probe p3). Usage keys are `cached_input_tokens`, `cache_write_input_tokens` and
`reasoning_output_tokens`. The stream never names the model; the hook payload does.

**Ten direct probes, all in `scratchpad/hookprobe`, none recorded, each a one-line prompt.**
p1 and p2, project file fires with and without the approval override; p3, `-c` with a one-line
command runs the hook and a bare exit 2 fails open; p4, a top-level `description` key is
harmless; p5, Hydra's real gate command fires from the project file in a plain repo; p6, the
hook payload for `apply_patch` and `Bash`; p7 and p8, no project hook inside a worktree; p9,
my own quoting, no finding; p10, the gate through a wrapper file in a worktree, deny held.
