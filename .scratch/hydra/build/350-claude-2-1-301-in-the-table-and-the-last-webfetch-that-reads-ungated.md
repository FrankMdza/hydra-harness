# 350 - `claude` 2.1.301's `LSP` and `Workflow` tools are in the capability table, and the last `WebFetch` of two haiku runs that reads `Ungated` is explained

Type: build
Status: claimed
Size: haiku
Blocked by:
Opened: 2026-10-15, by the session, from `hydra doctor --walk` after the three builds of that night
landed. Two `claude` haiku 5.5 runs under `permissive` on Claude Code 2.1.301 (`01a14290-f30d`,
`01a14295-9d3f`) each raise one `Ungated { tool: "WebFetch" }`, the last `WebFetch` of the run in
both cases (seq 304 of the first), while `hydra show --events` prints a derived `allow` and an
`unclassified` result for the same call id; the nineteen earlier `WebFetch` calls of the first run
carry hook evidence. The same two records raise `unknown_tool` findings for `LSP` and `Workflow`,
two tool names the `CLAUDE` table (`src/policy/table.rs`) does not hold, so a run that calls either
is refused `hydra/unknown-tool` the way build 338's `webrun` was. The walk's totals moved from
`Ungated: 5` to `Ungated: 7` in one night on account of these two.

## What

Two rows and one explanation. The rows: `LSP` and `Workflow` as 2.1.301 names them, with the
capability a free probe measures (a `-p` run with the hook pointed at a logging script, one call
each, the payload quoted in the Outcome), the version in the row's comment, and the drift fixture
of build 317 moved to 2.1.301 if it keys on version. The explanation: read the two records around
the last `WebFetch`, the hook sidecar's rows for that call id if any, and the `run_end` timing, and
say which of three it is: the hook's evidence line arrived after the record closed, the provider
ran the call without the hook (the shape map 123 found for `Edit`), or the normalizer joins the
decision to the call and the finding to a different id. Then either a fix in the normalizer, a
`RejectedBeforeHook`-style variant that names the shape, or a sentence in IMPLEMENTATION.md that
says why the last call of a run can read `Ungated` and what a reader should do with it.

## Contract

Input: a `claude` hook payload naming `LSP` or `Workflow`; and the two records above. Output: the
gate's decision for the first; for the second, one named shape and the walk's totals after.

## Cases

In `tests/gate.rs` beside the `claude` table cases, written first: `LSP` and `Workflow` under
`standard` and `read-only` answer with the measured capability's reason and not
`hydra/unknown-tool`; a third name, `Workflow2`, still does. In `tests/normalize.rs`, a synthetic
record with the two runs' shape around a last `WebFetch`, asserting the finding the explanation
names, written after the reading and before the fix.

## Do

1. The probe and the two rows.
2. The reading of the two records, in the Outcome with seqs.
3. The normalizer case and whatever it needs.
4. IMPLEMENTATION.md's `claude` paragraph.

## Done means

The four checks green, `hydra doctor --walk` on this machine prints `Ungated: 5` or says in the
Outcome why the two stay, and the Outcome quotes both hook payloads.

## Outcome

Stopped with the ticket open. Two steps are blocked from this run, and the table rows and the
normalizer case wait on them. Status stays `claimed`. No code changed.

**The two records cannot be read from here.** `./target/release/hydra show 01a14290-f30d --events`
fails with `Error: Operation not permitted (os error 1)`. Do step 2 needs the seqs around the last
`WebFetch`, the sidecar rows and the `run_end` timing, and none of that is in the worktree. Without
it the explanation cannot be chosen among the three shapes, so the normalizer case would be written
against a guess.

**`LSP` and `Workflow` are not measured.** The free probe is a `-p` run with a model name no
catalog holds. The binary refuses before the API call and still writes the init frame. I ran it on
the installed binary, `claude` 2.1.307, under a scratch `HOME`, which I removed afterwards. The
init frame offered:

`Task`, `Bash`, `CronCreate`, `CronDelete`, `CronList`, `DesignSync`, `Edit`, `EnterWorktree`,
`ExitWorktree`, `ListAgents`, `NotebookEdit`, `Read`, `ReportFindings`, `ScheduleWakeup`,
`SendMessage`, `Skill`, `TaskStop`, `ToolSearch`, `WebFetch`, `WebSearch`, `Workflow`, `Write`.

So `Workflow` is offered and `LSP` is not. Two gaps follow.

- `LSP` cannot be probed here. I did not find what makes 2.1.307 offer it, and 2.1.301 is not
  installed on this machine.
- `Workflow` needs a paid turn that runs a workflow, and a workflow starts agents. I did not run
  that without a go from the session. Its capability is also a decision. It may take the same
  capability as `Task`, `SubagentSpawn`, but that is a reading and not a measurement.

The table stays as it is. An absent name is refused `hydra/unknown-tool`, which fails closed. A row
written from a guess could widen what a policy grants.

**The code path the explanation would test.** `src/normalize/claude.rs:330` records a gate answer
only when a `hook_response` has an `output` that parses as JSON with a `permissionDecision` of
`allow` or `deny`. An empty or unparseable `output` records nothing, so a call answered that way
reads `Ungated`. The walk reads every line (`src/normalize/claude.rs:171`) and has no `break`, so a
hook line written after `run_end` is still read if the file holds it. The "evidence arrived after
the record closed" shape therefore needs the file to lack the line. The two records would show
that. This is a reading of the code, not of the records.

## The session's reading of the two records, 2026-10-21

Read outside the sandbox after the first run (`01a1609d-b8ee`) could not. Both records carry a
`hook_started` and a `hook_response` frame for every `WebFetch` call, 20 of 20 on `01a14290-f30d`
and 21 of 21 on `01a14295-9d3f`, so no evidence is missing and the first two shapes of the ticket
are out. The third shape is the one, with a cause the normalizer's comments do not foresee. On
`01a14290-f30d` the first calls read `call 104, start 106, response 107`, `call 108, start 110,
response 111`: the start lands after the call's own frame, which is what `group_starts` in
`src/normalize/claude.rs` assumes when it gives a call every start between its own seq and the
next call of the same tool. The last calls read `call 288, start 287`, `call 300, start 299`,
`call 304, start 303`: the start lands one seq before the call it belongs to. That start falls
in the previous call's window, every later call takes the start of the call after it, and the
last call's window holds nothing. The borrow rule only takes from a later window holding more
than one, and the last call has no later window, so it reads `Ungated`. The shifted joins before
it are wrong too, and invisible, because every answer is `allow`. On `01a14295-9d3f` the shape
is the same around seq 215.

So the fix is in the join, not in the table or in `claude`. Rule to build: per tool, when the
record holds exactly as many `PreToolUse:<Tool>` starts as `<Tool>` calls, the n-th start is the
n-th call's, in seq order, whatever side of the call frame it landed on. When the counts differ,
the window join and the borrow rule stay as they are. The `hook_response` is already joined to
its start by `hook_id`, so nothing else moves.

The two names. `Workflow` is offered by 2.1.307 in print mode and has no row. Measure it with
the hook pointed at a logging script that denies, so no agent starts: one haiku `-p` turn that
asks for a `Workflow` call and nothing else, under a scratch `HOME`, with the payload quoted in
the Outcome. Its capability follows the payload; a tool that starts agents is `SubagentSpawn`
unless the payload says otherwise. `LSP` is not offered by 2.1.307 in print mode and 2.1.301 is
not installed, so it gets no row: it stays unknown and refused, and one sentence in
`IMPLEMENTATION.md` says which version announced it and that nothing on this machine offers it
today.

## Cases, revised

In `tests/normalize.rs`, written first: a synthetic `claude` record with three `WebFetch` calls
whose `hook_started` frames land one seq before the second and third calls and one seq after
the first, each `hook_response` joined by `hook_id` with an `allow`. Before the fix the walk
raises `Ungated` on the third call; after it, no finding, and each call's answer is its own
hook's. A second record with two calls and three starts keeps the window join and the borrow
rule, pinned by the existing cases. In `tests/gate.rs`: `Workflow` under `standard` answers
with the measured capability's reason, `Workflow2` is still `hydra/unknown-tool`.
