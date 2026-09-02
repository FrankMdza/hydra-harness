# 11 - The hydra run CLI and the MCP delegation face

Type: grilling
Status: resolved
Blocked by: 01, 09

## Question

D7 fixes the shape: `hydra run --provider <p> --task "..." --parent <session-id>` returns JSON on stdout, with a thin MCP wrapper over the same path so Claude Code gets a typed `delegate` tool. This ticket fills it in.

Settle:

- The full argument surface of `hydra run`, and which arguments the MCP tool exposes
- The result JSON: what an orchestrator gets back, and how much of the child's trajectory rides in it against staying in the run file
- Whether delegation blocks or returns a handle the caller polls
- Concurrency: how many child runs at once, and what happens when one hangs
- Working directory and file conflicts, when parent and child touch the same repo. Worktree per child, shared tree, or the caller's problem
- How a child's permission prompts are answered when no human is watching that pane
- Failure and timeout semantics, and what the parent sees on each
- Depth: is a child allowed to delegate further, and is there a limit

Scenarios: Claude Code delegates the same file to cursor-agent and gemini at once. A child asks for permission to run `rm`. A child runs for forty minutes. The parent session dies while a child is mid-flight.

## What ticket 01 changed

**Target ACP, not print mode.** Across default print modes only five fields are common to all four providers. Driving `gemini --acp` and `hermes acp` instead widens that to streamed text, thought chunks, tool calls with arguments and results, and answerable permissions. Cursor has no ACP and becomes the permanent laggard.

That reshapes this ticket's permission question. In print mode, only Claude offers a two-way channel, its `can_use_tool` control request. Cursor auto-denies silently, Gemini throws, Hermes forces yolo. Over ACP, Gemini and Hermes both gain a real `session/request_permission`. So "how are a child's permission prompts answered when no human is watching" has three different answers depending on provider and mode, and Cursor's answer is that it cannot be asked.

**Session id minting differs per provider**, so `--parent <session-id>` threads Hydra's own run id, never the provider's. See the amendment on ticket 09.

**Claude alone supports bidirectional stdin** through `--input-format stream-json`. If the delegation contract needs to send anything to a running child rather than only reading from it, that is a Claude-only capability today.

## Answer

**Every child is spawned detached, and blocking is a view over it.** `hydra run` in its default mode tails the child's run file and waits. That makes the timeout degradation below trivial, because giving up on a call only means it stops watching, and it means closing a terminal orphans nothing. The cost is reaping: `hydra status` reconciles live pids against open run files, the concurrency cap stops runaways, and stale worktrees need a cleanup command. That is the price of not building a daemon, which would be a bigger thing than this map's destination.

**Calls block with a timeout, then degrade to a handle.** `--timeout` defaults to roughly ten minutes. On expiry the call returns a handle rather than killing the child, so short delegations return inline where they are easiest to reason about, and long ones become something you poll with `hydra status <run-id>`. Concurrency comes free from the orchestrator's own parallel tool calls, so Hydra needs no scheduler, only a cap on live children.

**A git worktree per child**, when the cwd is a repo, branch named from the run id. `--shared-tree` opts out for the case where you deliberately want two agents in the same files. The spawn seeds the worktree with the parent's uncommitted changes applied as a patch, because a clean checkout would hide exactly the work-in-progress most delegations are about. This also keeps us consistent with Ouroboros, which ticket 06 found uses worktrees rather than clones.

Two children editing one file with neither aware of the other is the most likely way this feature produces garbage, and the worktree is what prevents it.

**Permissions come from a policy the parent passes at spawn**: allowed tools, denied tools, deny by default for anything unlisted. Hydra answers from it and writes a `permission_decision` event with decider `policy`, so the record shows who allowed what. Nothing escalates to a human by default, because a blocked child stalls the whole delegation and nobody is watching that pane.

The consequence to state plainly: Cursor cannot be asked. It has no permission channel in print mode and no ACP at all, so it silently auto-denies or takes `--force`. That means Cursor gets `--force` only under a fully permissive policy, and under any narrower policy **Cursor cannot be delegated tool work**. That is Cursor's missing channel, not a choice, and the worktree is what makes `--force` survivable at all.

**Depth limit of 2, configurable, plus a cap on total concurrent runs.** `parent_run_id` is already in the envelope from ticket 09, so depth is computable at spawn with no extra bookkeeping. A child that delegates is how three subscriptions get drained overnight and you find out from a billing page.

**The result stays small.** `run_id`, `status`, the final assistant text, the worktree branch, a diff stat, `usage` with its `usage_source`, and the run file path. Nothing else. The orchestrator asks for more explicitly with `hydra show <run-id> --events`.

The reasoning matters more than the field list: delegation exists to keep a child's trajectory out of the orchestrator's context. A result that returns the whole trajectory defeats the feature it implements.

**Status is derived and says how confident it is.** A `status` enum of `completed`, `failed`, `timed_out`, `denied`, `crashed`, derived per provider by the adapter, alongside the raw `exit_code` and an `error` string. Plus `status_source` of `provider`, `derived` or `unknown`, mirroring `usage_source` from ticket 09.

That field earns its place on Hermes, where status can only ever be `completed` or `crashed` and never "the agent failed", because Hermes exits 0 on a failed turn. Without `status_source: unknown` an orchestrator would confidently report success on a Hermes run that failed.

**The MCP face is narrow: `delegate(provider, task, files?, policy?)`.** No `--shared-tree`, no timeout override, no depth. Those three are how a delegation escapes its worktree, outlives its budget or spawns a tree, and they belong in config as a human decision rather than a per-call choice a model makes unattended. The CLI keeps the full surface.

**Not decided here, and left to the harness spec:** how the four incompatible tool-call representations map onto one shape. Ticket 09 already noted this is adapter work, and keeping the raw frame makes getting it wrong recoverable.

## Amended by ticket 18

**`agy` joins Cursor in the category that cannot be asked.** This ticket's answer has Hydra
answering permission prompts from a policy and writing a `permission_decision` event with decider
`policy`. That is not possible for `agy` at 1.1.23. Its stream-json input accepts exactly one
message shape, `{"event":"user","message":{"content":"..."}}`, and anything else comes back as
`warning: ignoring unsupported stream input message event`, `permission_response` included.
Permission refusals surface as tool errors after the fact, never as answerable requests.

So `agy` gets `--dangerously-skip-permissions` only under a fully permissive policy, and under any
narrower policy it can be delegated read-only work. Same shape as Cursor's `--force`, and the
worktree is again what makes it survivable. Two of four providers now sit in this category, which
is worth weighing before the policy engine gets built for the other two.

## Amended by ticket 12

**`--allow-billed` is outside the delegation policy.** The flag that unlocks the one billed path is
typed by a human per run. The policy cannot set it, and a parent carrying it does not pass it to a
child. Depth is capped at 2, so without this an agent two levels from the human could spend money.

**A delegated child receives no credential of any kind.** No virtual key, because the raw-model
plane does not ship in v1. No inherited secret, because Hydra builds each child's environment from
an allowlist. No browser login, because children spawn detached and a detached process cannot
complete OAuth. A child that hits an auth wall fails fast with a remediation string naming the
provider and the command the human should run.
