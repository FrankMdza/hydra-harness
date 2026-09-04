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

## Amended by ticket 13

**The concurrency cap refuses rather than queues or blocks.** It returns a typed error naming what
is in flight and its run ids, so the caller decides. Queueing needs a live scheduler, which D10
rules out, and blocking parks the orchestrator that delegation exists to protect.

**Reaping a crashed child never destroys its work.** `hydra reap` is typed by a human, and it
commits a worktree's uncommitted changes to the child's own branch before removing the worktree.
Abandonment is derived from the run record's pid and start time, not from a timeout.

## Amended by ticket 26, 2026-09-03

**"Cursor cannot be delegated tool work under any policy narrower than fully permissive" is too
strong, and so is ticket 18's amendment putting `agy` in the same category.** Both were right about
the permission-prompt surface and both missed the hook surface, which does not ask and can refuse.

**What changed.** All three routable providers can hard-block a tool call from a config file Hydra
writes per run. Probed on 2026-09-03: `claude` denies under `--permission-mode bypassPermissions`,
`agy` denies with the model otherwise unconstrained, `cursor-agent` rejects in print mode with
`--force` passed. The gate covers shell, file writes, file reads, MCP calls and subagent spawns. So
this ticket's sentence should now read: **Cursor and `agy` cannot be *asked*, and both can be
*gated*.** Under a narrower policy they get the permissive flag plus a hook config that enforces the
policy, rather than being cut down to read-only work.

**What still stands.** Everything about the prompt channel. Cursor has no permission channel in
print mode and no ACP. `agy`'s stream-json input still accepts one message shape and ignores
`permission_response`. Nothing escalates to a human mid-run on either, and this ticket's reason for
that, a blocked child stalls the delegation and nobody is watching that pane, is unchanged. A hook
denial is a decision Hydra made before the run started, not a question it answered during one.

**What this costs the policy engine.** `permission_decision` gains a decider of `hook` alongside
`policy`, and it must be derived from the event stream: all three providers exit 0 and report
success on a run where the gate refused every action. It also has to emit up to three config
shapes. Cursor reads Claude Code's hook format and translates it, so one `.claude/settings.json`
written into the worktree gates both `claude` and `cursor-agent`, while `agy` needs its own
`.agents/hooks.json`. The three disagree on failure: `agy` fails closed, `claude` fails open on a
response in a dialect it does not recognize, and Cursor's Claude-conversion path sets
`failClosed:false`.

**Two spawn-time rules this adds.** The hook config must be a real file, never a symlink, because
`cursor-agent` refuses a symlinked project config silently and runs the command anyway, which
collides with D9. And it must be rewritten at every spawn: each provider snapshots the config at
process start, so mid-run tamper fails, but a worktree reused after a child edited the file starts
with no gate.

Detail and probe transcripts in
[research/26-hook-level-action-gate.md](../research/26-hook-level-action-gate.md).

## Amended by ticket 27, 2026-09-03

**"Allowed tools, denied tools, deny by default" becomes a named capability allowlist.** The
`denied` list goes, because with deny-by-default it can only subtract from the allow list and a
policy where two rules can disagree is a bug with a schema. Argument conditions live inside an allow
entry instead. Entries name capabilities rather than provider tool names, since ticket 24 and ticket
23 both found the three providers share no vocabulary, and Hydra owns the mapping table. A tool name
the table does not recognize is denied and recorded.

**`policy?` on the MCP face takes a name, not a list.** This ticket made the argument optional and a
model fills it in. Five names ship: `judge`, `curate`, `read-only`, `standard`, `permissive`. A name
is something a human wrote once; a list is security composed at call time by the least accountable
party in the chain.

**The depth cap has a blind spot this ticket did not see.** It counts spawns through `hydra run`, and
all three providers can spawn a subagent inside a run that never reaches Hydra, gets no run id and
writes no run file. Every policy but `permissive` denies in-process subagent spawns, which makes the
cap mean what it says and keeps D10's record complete. A delegated Claude child loses `Task`.

**A delegated child inherits the human's MCP servers.** Ticket 23 probed it: user-scope servers
appear in a headless child's tool list and their processes run, on all three providers, and only
`claude` can scope them off per run. The gate blocks every call, so this is exposure rather than
execution, but a policy claiming "this child sees no MCP" would be claiming something Hydra cannot
deliver on two of three.

## Amended by ticket 28, 2026-09-03

**Monotonic narrowing on the policy name.** Ticket 27 turned this ticket's `policy?` argument into a
name from a fixed set of five. Ticket 28 adds that the name a caller passes may never be wider than
the caller's own policy, measured as capability set containment, with incomparable pairs refused.
The MCP face's caller is a model, so without this a poisoned parent names `permissive`, and the depth
cap does nothing about it because the cap bounds recursion rather than escalation.

**A child's result is marked on the way back.** `hydra run` keeps the child's text as a plain string
field in its JSON, and the MCP face renders it inside a boundary whose delimiter is randomised per
call, picked after the child has returned so the child cannot close its own envelope. Results already
stay small on purpose under this ticket; this says what wraps them.

## Amended by ticket 29, 2026-09-03

**Cleanup reaps a spawn directory as well as a worktree.** Ticket 29 puts everything Hydra generates
at launch in `~/hydra/spawn/<run-id>/`, a sibling of `runs/`: the `--mcp-config` file for `claude`,
the `CURSOR_DATA_DIR` root for Cursor, and ticket 27's compiled gate script. It dies with the
worktree, which is why ticket 09 gained fields rather than the record pointing at files.

**`--shared-tree` costs MCP on Cursor.** A shared tree is one Hydra did not create, so it gets no
`CURSOR_DATA_DIR` redirect, and rather than writing an approval into `~/.cursor/projects/` Hydra
injects no MCP config there at all. That is ticket 22's rule applied one level down. The run still
happens with the human's own servers. If the run's policy admits a server, the launch fails instead,
because ticket 29 governs the shortfall rather than recording it.

## Amended by ticket 20, 2026-09-04

**`model` stays off the delegate face.** The MCP face remains `delegate(provider, task, files?,
policy?)`, and the model comes from the policy and Hydra's config. This ticket kept `--shared-tree`,
the timeout override and depth off that face because they are how a delegation escapes its worktree,
outlives its budget or spawns a tree, and called them a human decision rather than a per-call choice
a model makes unattended. `model` joins them for the same reason. `agy` serves Claude Opus 4.6, three
releases behind what `claude --model opus` resolves to, so a `model` argument is the lever that lets
an unattended orchestrator send real work to an older generation to dodge a limit it noticed. The CLI
keeps the full surface, which is this ticket's own split.

## Amended by ticket 16, 2026-09-04

**`denied` leaves the status enum and `gated` replaces it.** This ticket's five values were
`completed`, `failed`, `timed_out`, `denied` and `crashed`; ticket 27 later minted `gated` for the
same situation. The record contract owns the enum and settles it at nine values with `gated` and no
`denied`. Two labels for one state is a bug with a schema, and `denied` also read as a per-tool-call
fact, which is `permission_decision`'s job.

**`status_source` loses `provider` and gains `refusal`.** This ticket gave it `provider`, `derived`
and `unknown`, and `provider` earned its place against hermes. No routable provider produces a
run-level status worth reading: `cursor-agent`'s `result.subtype` is only ever `success` in shipped
code, `agy` reports `SUCCESS` on a fully denied run, and `claude` reports `success` on a 429. The
value has no producer. `refusal` replaces it, for the outcomes Hydra decides itself rather than
reads. `unknown` survives, because the adapter's `unclassified` gives it a new producer.

**The returned JSON is a projection of the record.** This ticket put `exit_code`, `status`,
`status_source` and `error` in the JSON and ticket 09 put none of them in the file, which under D10
meant the JSON carried facts the source of truth did not have. They are on `run_end` now, and the
JSON adds exactly one thing that is not in the record, the diff stat, which is computed from git at
return time.
