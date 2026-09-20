# 48 - Codex CLI as the fourth provider

Type: build
Status: claimed
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

## Outcome, so far

Items 1, 2, 3 and the tests half of 6 built on 2026-09-20 by `cursor-agent` on Composer 2.5, run
01a0bcdb-e6ed under `standard`, 9.4M tokens, 14 files, 1365 insertions. Applied to master after a
mechanism read; nothing needed correcting before the suite, and `clippy --all-targets`, `fmt` and
the targeted tests were green in the worktree and on master. The docs half of item 6 is the
orchestrating session's, in `IMPLEMENTATION.md` under the `codex` heading.

**Item 4 was decided by the human on 2026-09-20: `standard` and `permissive` only.** `read-only`
and `judge` refuse a `codex` launch at pre-flight with a message that names this item, and the
bounded shell read for `codex` alone is map ticket 63, open. `codex` cannot sit on the `evolve`
panel until that closes.

**Item 5 is still open**, because `codex login status` on this machine said not logged in when
the run landed. Four `MEASURE 48.5` comments in `src/provider/codex.rs` and `src/normalize/codex.rs`
say what to measure; the ticket closes when a recorded `codex` run exists and each of them is
replaced by what the record showed.

**One thing the ticket assumed that turned out false.** `apply_patch` carries no path, so under
`standard` the gate refuses it as `hydra/no-path` and a run writes through the shell or not at
all. The ticket read `apply_patch` as a pathed write. Whether the patch body can be parsed for
its paths is part of item 5.
