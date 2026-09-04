# 30 - When two hooks answer, does deny win?

Ticket: [30 - When two hooks answer, does deny win?](../issues/30-hook-precedence-deny-vs-allow.md)
Type: research. This file reports measurements. It proposes mechanisms but decides nothing.

Measured 2026-09-03 on this machine, 19:00 to 21:45 local. Ticket 26 probed one hook at a time.
This is the same rig with two hooks answering at once.

## Versions under test

Read at 19:01 local, before any probe, and again at 21:43 after the last one. Identical both times,
so nothing self-updated mid-ticket and every result below belongs to one set of builds.

```
claude        2.1.257 (Claude Code)
cursor-agent  2026.09.02-c22c1a3
agy           1.1.25
```

One thing to carry: **ticket 26 probed `agy` 1.1.24 and this ticket probed 1.1.25.** Every `agy`
claim below was re-measured rather than inherited, and one of ticket 26's conclusions turns out to
need narrowing. See Q5.

## Probe hygiene

Everything created for this ticket lives in
`/private/tmp/claude-501/-Users-USER/9e5e00b0-4259-4f48-ac34-1ff4a7a85104/scratchpad/t30/` and is
prefixed `t30-`. No user-scope config was written. `~/.claude/settings.json` still has its
2026-08-31 22:57 mtime and sha `7fd1e3d3637f3f62dc77d77268afebba015ae4a5`, `~/.cursor/hooks.json`
still does not exist, and neither does `~/.gemini/config/hooks.json`. The full check is at the end.

The user's own guards were read, never edited, and they were used as a live counterparty in two
runs. That is the only way user scope could be probed at all, and it needed a command their guard
refuses that is harmless if the guard loses. `git t30probe` is that command: `guard_git.py` blocks
unknown subcommands, and if a hook allow ever beat it, git prints `'t30probe' is not a git command`
and exits 1.

## The short version

**Deny wins on all three providers, in both declaration orders, in every scope pair I could set,
and against the most permissive permission mode each provider offers.** Ticket 27's composition
decision stands as written. It does not invert. `--restricted` does not become mandatory, and
ticket 22's rule is not triggered.

The result is stronger than ticket 27 needed. The question was whether a hook allow can beat a hook
deny. The answer is that on all three providers **a hook cannot widen anything at all**. It is a
veto and nothing else.

- On `claude` a hook allow loses to a `permissions.deny` rule and loses to a `permissions.ask` rule.
  All it can do is skip the interactive approval prompt for a call no rule refuses.
- On `cursor-agent` no code path reads a hook's `allow`. The value is computed by the merge function
  and never consulted.
- On `agy` a hook allow did not lift the headless auto-deny on a file write.

Two of three ship the precedence rule as readable code, and both spell it the same way.

```
claude 2.1.257   deny > defer > ask > allow, and deny assigns unconditionally
cursor 2026.09.02 deny > ask > allow, computed pairwise as results arrive
agy    1.1.25    not readable in the Go binary, measured instead: deny wins and short-circuits
```

Five things that will bite whoever writes the spec:

1. `agy` stops at the first deny, so Hydra's hook may never run and the recorded reason may belong
   to the user's hook instead. The order separately named hooks run in is not the file order.
2. Cursor concatenates every hook's message into one reason string. In one run the model read the
   allowing hook's text and reported it as the reason for the block.
3. `--setting-sources` on `claude` drops the user's hooks while keeping all 25 tools. Ticket 27 knew
   only about `--restricted`, which strips Bash.
4. Cursor's team hook path is built from the home directory, not the workspace, so ticket 26's path
   table is wrong on that row.
5. All three still exit 0 and report success on a run where the gate refused everything. Seven of
   seven `agy` runs reported `status: SUCCESS`.

## The rig

One script answers in whichever dialect it is told to, and logs every invocation with its label and
its payload, so a run's fired set is recoverable afterwards.

`t30/bin/t30hook.sh`, argv is `<label> <dialect> <decision>`:

```sh
#!/bin/sh
LABEL="$1"; DIALECT="$2"; DECISION="$3"
LOG=.../t30/fired.log
PAYLOAD=$(cat)
printf '%s\t%s\t%s\t%s\n' "$(date +%H:%M:%S)" "$LABEL" "$DECISION" "$PAYLOAD" >> "$LOG"
REASON="t30 $LABEL says $DECISION"
case "$DIALECT" in
  cc)     printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"%s","permissionDecisionReason":"%s"}}' "$DECISION" "$REASON"; exit 0 ;;
  cc2)    if [ "$DECISION" = "deny" ]; then printf '%s\n' "$REASON" >&2; exit 2; fi; exit 0 ;;
  cursor) printf '{"permission":"%s","user_message":"%s","agent_message":"%s"}' "$DECISION" "$REASON" "$REASON"; exit 0 ;;
  agy)    printf '{"decision":"%s","reason":"%s"}' "$DECISION" "$REASON"; exit 0 ;;
esac
```

Three workspaces, one per provider, so a stray config from one probe cannot reach another:
`t30/ws` for `claude`, `t30/cws` for `cursor-agent`, `t30/aws1` and `t30/aws2` for `agy`.

Every command below is reproducible as written. Nothing depends on state left by an earlier run
except where the text says so.

## Q1: two hooks on one tool call, one denying and one allowing

### `claude` 2.1.257

Two `PreToolUse` matcher groups on `Bash` in a single settings file handed to `--settings`, each
pointing at a different script invocation so the deduplication described in Q4 cannot collapse them.

```json
{"hooks":{"PreToolUse":[
  {"matcher":"Bash","hooks":[{"type":"command","command":"<rig> A cc allow"}]},
  {"matcher":"Bash","hooks":[{"type":"command","command":"<rig> B cc deny"}]}
]}}
```

Run C1, allow declared first:

```
cd t30/ws && claude -p "Run this bash command: echo T30MARKER" \
  --output-format stream-json --verbose --model sonnet \
  --settings t30/s-allow-deny.json --permission-mode bypassPermissions
```

```
INIT permissionMode=bypassPermissions tools=25
TOOL_USE: Bash {"command": "echo T30MARKER", "description": "Print T30MARKER string"}
TOOL_RESULT is_error=True : t30 B says deny
RESULT subtype=success is_error=False : The command was denied ("t30 B says deny"), so `echo T30MARKER` didn't run.
```

The raw frame:

```json
[{"type":"tool_result","content":"t30 B says deny","is_error":true,"tool_use_id":"toolu_01V1n5c12TaaTM6UHnCTdAFS"}]
```

Both hooks ran. From `t30/fired.log`:

```
19:06:15 A allow
19:06:15 B deny
```

Run C2 is the same file with the two groups swapped, `s-deny-allow.json`. Same outcome, same reason
string, both hooks fired in the new declaration order:

```
TOOL_RESULT is_error=True : t30 B says deny
19:06:29 B deny
19:06:29 A allow
```

**Deny wins, and the order the hooks are declared in does not matter.** The allowing hook is not
skipped and its answer is not an error, it is simply outranked.

The rule is in the shipped binary. From `strings -a ~/.local/share/claude/versions/2.1.257`, in the
hook result loop:

```js
switch(Rn.permissionBehavior){
  case"deny":  Cn="deny"; break;
  case"defer": if(Cn!=="deny") Cn="defer"; break;
  case"ask":   if(Cn!=="deny"&&Cn!=="defer") Cn="ask"; break;
  case"allow": if(!Cn) Cn="allow"; break;
  case"passthrough": break
}
```

`deny` assigns unconditionally and every other branch guards against overwriting it. `allow` only
assigns when nothing has been decided yet. That is order-independent by construction, which is why
C1 and C2 agree.

The same loop treats a non-zero exit the same way. A few hundred characters earlier:

```js
if(Rn.blockingError) yield{blockingError:Rn.blockingError,...}, Cn="deny";
```

So a hook that writes to stderr and exits 2, which is what the user's own guards do, lands in the
same lattice as a hook that returns JSON. There is one decision channel, not two.

### `cursor-agent` 2026.09.02-c22c1a3

Two `beforeShellExecution` hooks in the project config, `t30/cws/.cursor/hooks.json`:

```json
{"version":1,"hooks":{"beforeShellExecution":[
 {"command":"<rig> CUR-A cursor allow"},
 {"command":"<rig> CUR-B cursor deny"}
]}}
```

```
cd t30/cws && cursor-agent --workspace t30/cws -f \
  -p "Run this shell command: echo T30MARKER" --output-format stream-json
```

Run U1, allow declared first:

```json
{"rejected": {"command": "echo T30MARKER", "workingDirectory": "",
  "reason": "Command execution was blocked by a hook: t30 CUR-A says allow\n\n---\n\nt30 CUR-B says deny\n\nTo view or modify configured hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly": false}}
```

Run U2 with the two entries swapped:

```json
{"rejected": {"command": "echo T30MARKER", "workingDirectory": "",
  "reason": "Command execution was blocked by a hook: t30 CUR-B says deny\n\n---\n\nt30 CUR-A says allow\n\nTo view or modify configured hooks, go to Cursor Settings > Hooks.\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly": false}}
```

**Deny wins, order does not matter, and both hooks run.** Cursor runs them concurrently rather than
in sequence. From `190.index.js`, `executeAllAndMerge` builds a promise per hook, awaits
`Promise.all`, keeps the successful results, and hands the array to a merge function:

```js
const s=yield Promise.all(n),r=[];
for(const e of s) e.success&&void 0!==e.data&&r.push(e.data);
return (0,_.wb)(e,r)
```

`wb` resolves to function `O` in `../hooks/dist/index.js`, bundled into `index.js` at offset
4973149. The permission field is the only one merged by rank:

```js
"permission"===e && i
  ? c[e]=(u=r, "deny"===(l=c[e])||"deny"===u ? "deny"
         : "ask"===l||"ask"===u ? "ask"
         : "allow"===l||"allow"===u ? "allow" : void 0)
  : ...
```

`l` is the accumulator and `u` is the incoming value, so the comparison is symmetric and the result
does not depend on which hook finished first. Every other field, `user_message` included, is
concatenated by the helper `x`, which is why both messages appear in the reason above.

That concatenation is a real hazard for the run record. In run U7 below the model read the wrong
half and reported the block as `Rejected: t30 CUR-PROJ says allow`. Hydra's policy identifier
travels into the reason string, but it does not travel alone, and the model narrates whichever line
it noticed.

### `agy` 1.1.25

The embedded documentation, read out of the binary with `strings -a ~/.local/bin/agy`, says only
this about several hooks meeting:

> **Merging**: Multiple named hooks (e.g., from different plugins or configs) for the same event
> type are merged and executed sequentially.

Nothing about how conflicting decisions resolve, so it had to be measured.

Run A1 used two separately named hooks in `t30/aws1/.agents/hooks.json`, the allow first in the
file. `agy --add-dir "$A" -p "/hooks" --output-format json` confirmed both were mounted at zero
token cost, listed in file order. Then:

```
cd t30/aws1 && agy --add-dir t30/aws1 \
  -p "Run the shell command: echo T30MARKER. If it is blocked, stop immediately and do not investigate." \
  --output-format stream-json
```

```json
{"event":"step_update","step_update":{"step_index":2,"state":"ERROR","step_type":"tool",
 "tool_name":"run_command",
 "tool_info":{"parameters":{"CommandLine":"echo T30MARKER"},
   "error":{"type":"TOOL_ERROR","message":"tool call denied by pre-tool hook: t30 AGY-B says deny"}}}}
```

Deny won, but only the denying hook fired:

```
19:21:23 AGY-B deny
```

Two possible readings, and they matter in opposite directions. Either an allow short-circuits the
rest, in which case a user hook could stop Hydra's deny from ever running, or a deny short-circuits
the rest and the deny simply happened to run first. Top-level hook names live in a Go map, whose
iteration order is not the file order, so the file could not settle it.

Handler arrays inside one named hook are a JSON array and keep their order, so runs A2 and A3 pinned
it down. A2, handlers `[allow, deny]`:

```json
{"t30-gate": {"PreToolUse":[{"matcher":"run_command","hooks":[
   {"type":"command","command":"<rig> AGY-A agy allow","timeout":10},
   {"type":"command","command":"<rig> AGY-B agy deny","timeout":10}
]}]}}
```

```
STEP 2 run_command state=ERROR err={"type":"TOOL_ERROR","message":"tool call denied by pre-tool hook: t30 AGY-B says deny"}
19:22:00 AGY-A allow
19:22:00 AGY-B deny
```

A3, the same two handlers reversed:

```
STEP 2 run_command state=ERROR err={"type":"TOOL_ERROR","message":"tool call denied by pre-tool hook: t30 AGY-B says deny"}
19:22:22 AGY-B deny
```

**A deny short-circuits the remaining handlers. An allow does not.** Both orders end in a denial,
which is the answer the ticket needs, and the asymmetry is the safe one: nothing a permissive hook
returns can stop a later deny from running.

The cost is recordability. When the user's hook denies first, Hydra's hook never fires, so Hydra
learns of the denial only from the tool error, and the reason string in that error belongs to the
user's hook. Combined with the map ordering, which hook answers is not deterministic across runs.

## Q2: does scope change the outcome

### `claude`, three scopes I own plus the user's live one

`--settings` composes rather than replaces. `claude --help` calls it "Path to a settings JSON file or
a JSON string to load additional settings from", and run C9 below shows the user's hooks running
alongside it.

Run C3, allow on the command line against deny in the project file:

```
cp t30/s-deny.json t30/ws/.claude/settings.json
cd t30/ws && claude -p "Run this bash command: echo T30MARKER" \
  --output-format stream-json --verbose --model sonnet \
  --settings t30/s-allow.json --permission-mode bypassPermissions
```

```
TOOL_RESULT is_error=True : t30 B says deny
19:06:44 B deny
19:06:44 A allow
```

Run C4 swaps the two files, deny on the command line and allow in the project file:

```
TOOL_RESULT is_error=True : t30 B says deny
19:06:57 A allow
19:06:57 B deny
```

The higher-precedence scope loses when it is the one allowing, in both directions. That is expected
from the aggregation code, which carries no notion of a hook's source at all. The switch quoted in
Q1 sees a flat list, and `hookSource` is attached only for telemetry and for the message shown after
the decision is made.

Run C9 is the one that matters, because it is the real composition on this machine. My allow sits on
the command line. The user's three guards sit in `~/.claude/settings.json` at user scope and deny by
writing to stderr and exiting 2, since `_cmdparse.py` has `EMIT_JSON_DECISION = False`. So the two
answers are in different scopes and in different dialects.

```
cd t30/ws && claude -p "Run this bash command exactly: git t30probe" \
  --output-format stream-json --verbose --model sonnet \
  --settings t30/s-allow.json --permission-mode bypassPermissions
```

```
TOOL_USE: Bash {"command": "git t30probe", "description": "Run git t30probe"}
TOOL_RESULT is_error=True : PreToolUse:Bash hook error: [python3 /Users/USER/.claude/hooks/guard_git.py]: 'git t30probe' is not a known subcommand: it could be an alias wrapping a destructive operation.
19:11:33 A allow
```

**The user's user-scope deny beat a command-line allow, across scopes and across dialects, under
`bypassPermissions`.** This is exactly the shape ticket 27 assumed and never tested.

### `claude`, the two isolation levers

Ticket 27 names `--restricted` as the clean slate. It works, and there is a second lever it did not
know about.

Run C10a, `--restricted --tools Bash` with an empty `--settings`, asking for `git t30probe`:

```
INIT permissionMode=default tools=1
TOOL_RESULT is_error=True : This command requires approval
```

Only one tool survives and nothing runs, so this run alone cannot tell an ignored guard from a
confined session refusing everything. Run C10b adds the grant the help text says a confined session
needs:

```
cd t30/ws && claude -p "Run this bash command exactly: git t30probe" \
  --output-format stream-json --verbose --model sonnet \
  --restricted --tools Bash --allowedTools "Bash(git t30probe)" --settings '{}'
```

```
TOOL_RESULT is_error=True : Exit code 1\ngit: 't30probe' is not a git command. See 'git --help'.
```

The command ran. `guard_git.py` never fired. **`--restricted` genuinely drops the user's hooks.**

Run C11 uses `--setting-sources project`, which `claude --help` describes as a "Comma-separated list
of setting sources to load (user, project, local)":

```
cd t30/ws && claude -p "Run this bash command exactly: git t30probe" \
  --output-format stream-json --verbose --model sonnet \
  --setting-sources project --settings t30/s-allow.json --permission-mode bypassPermissions
```

```
INIT permissionMode=bypassPermissions tools=25
TOOL_RESULT is_error=True : Exit code 1\ngit: 't30probe' is not a git command. See 'git --help'.
19:12:31 A allow
```

The user's guard did not fire, all 25 tools stayed available, `bypassPermissions` was accepted, and
Hydra's own `--settings` hook still ran. **`--setting-sources` isolates without the costs
`--restricted` imposes**, which is worth knowing whether or not composition holds.

`--restricted` also does something to a hook's answer that `--setting-sources` does not. From the
binary:

```js
function Qgn(e,n){
  if(!MXe())return e;
  if(e.permissionBehavior==="allow") t(`${n} permissionDecision=allow ignored: a confined session takes grants only from its command line`), e.permissionBehavior=void 0;
  ...
}
```

Under `--restricted` a hook's allow is discarded before it reaches the lattice. A confined session's
hooks can only deny.

### `cursor-agent`, and a correction to ticket 26's path table

Ticket 26 lists the team config as `<workspace>/.cursor/managed/active-team-hooks/hooks.json`. Run
U3a put an allow there against a project-scope deny and the team hook never fired.

The reason is in the bundle. `190.index.js` defines the path builder against a root:

```js
function L(e){return d.join(e,".cursor","managed","active-team-hooks","hooks.json")}
```

and `U(e)`, which builds the rest of the table from the workspace, does not include a
`teamConfigPath` key at all. The CLI adds it separately, in `4274.index.js`:

```js
ct.teamConfigPath=(0,nr.lV)(o.homedir());
```

with `lV:()=>L`. **For `cursor-agent` the team hook file is `~/.cursor/managed/active-team-hooks/hooks.json`,
in the user's home, not in the workspace.** The workspace-relative row in ticket 26's table is wrong
and should be amended. It also means only three of the seven locations sit inside a directory Hydra
creates: `<workspace>/.cursor/hooks.json`, `<workspace>/.claude/settings.json` and
`<workspace>/.claude/settings.local.json`.

Runs U4a and U4b use two of those three, on the same event, in different formats. Cursor-native
`preToolUse` in `.cursor/hooks.json` against Claude-format `PreToolUse` matcher `Bash` in
`.claude/settings.json`, which the loader translates to `preToolUse`/`Shell`.

U4a, project allows and claude-project denies:

```json
{"rejected": {"command": "echo T30MARKER",
  "reason": "t30 CUR-PROJ says allow\n\n---\n\nt30 CC-PROJ says deny\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly": false}}
```

U4b, reversed:

```json
{"rejected": {"command": "echo T30MARKER",
  "reason": "t30 CUR-PROJ says deny\n\n---\n\nt30 CC-PROJ says allow\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly": false}}
```

Both fired in both runs, deny won both times. The load order in `executeHookForStep` is enterprise,
team, project, user, runtime, claude-project-local, claude-project, claude-user, then plugins, and
since the merge is pairwise and symmetric that order decides only which hooks join the list.

Run U7 is Cursor's version of C9, against the user's live config. `cursor-agent` reads
`~/.claude/settings.json` as its claude-user scope, so the user's three guards are already loaded on
every Cursor run on this machine. Project scope allows, user scope denies by exiting 2:

```
cd t30/cws && cursor-agent --workspace t30/cws -f \
  -p "Run this shell command exactly: git t30probe" --output-format stream-json
```

```json
{"rejected": {"command": "git t30probe", "workingDirectory": "",
  "reason": "t30 CUR-PROJ says allow\n\n---\n\nHook blocked with message: 'git t30probe' is not a known subcommand: it could be an alias wrapping a destructive operation.\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly": false}}
```

**The user's real user-scope guard beat a project-scope allow inside Cursor, through Cursor's
translation of Claude's config format, deciding by exit code rather than by JSON.** The wrapper text
comes from the helper that handles a non-zero exit:

```js
function M(e,t,n){const r=e.trim(),i=t.trim();
  return r||(i?`Hook blocked with message: ${i}`:`Hook "${n}" blocked this action (exit code 2) but provided no reason.`)}
```

The model's own summary of that run read `Rejected: t30 CUR-PROJ says allow`, quoting the hook that
allowed. The block was correct and the narration was not.

### `agy`, two config files

`agy`'s other config locations are `~/.gemini/config/hooks.json` for the shared file and the
`/hooks` TUI, plus the settings files under `~/.gemini/antigravity-cli/` and
`~/.gemini/config/projects/`. All of them are in the user's home and none was written.

A second config source is still reachable without touching the home directory, because `--add-dir`
is repeatable and each root carries its own `.agents/`. The free `/hooks` oracle confirms both load:

```
cd t30/aws1 && agy --add-dir t30/aws1 --add-dir t30/aws2 -p "/hooks" --output-format json
usage total: 0
t30-root1-allow <- .../t30/aws1/.agents/hooks.json
t30-root2-deny  <- .../t30/aws2/.agents/hooks.json
```

Run A7, allow in the first root and deny in the second:

```json
{"error":{"type":"TOOL_ERROR","message":"tool call denied by pre-tool hook: t30 ROOT2-DENY says deny"}}
```

```
19:24:19 ROOT2-DENY deny
```

Deny won across two config files, and the deny short-circuited the allow again. This is the same
merge path the embedded doc describes for "different plugins or configs", so it is the closest
reachable stand-in for the user's own config answering. It is a stand-in, not the thing itself. See
the gaps section.

## Q3: can a hook widen at all

Ticket 26 established that a hook deny beats a permissive permission mode. This is the reverse
question, and the answer is the same on all three providers for different reasons.

### `claude`, measured twice and then read out of the binary

Run C0 establishes the baseline. Default permission mode, no hooks, no rules:

```
cd t30/ws && claude -p "Run this bash command: echo T30MARKER" \
  --output-format stream-json --verbose --model sonnet --settings '{}'
```

```
INIT permissionMode=default tools=25
TOOL_RESULT is_error=False : T30MARKER
```

Print mode approves Bash on its own here, so there is no ambient refusal for a hook to overturn. A
refusal has to be created, and there are two kinds.

Run C5, a `permissions.deny` rule and no hooks:

```json
{"permissions":{"deny":["Bash(echo:*)"]}}
```

```
TOOL_RESULT is_error=True : Permission to use Bash with command echo T30MARKER has been denied.
```

Run C6, the same rule plus a hook returning allow:

```json
{"permissions":{"deny":["Bash(echo:*)"]},
 "hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"<rig> A cc allow"}]}]}}
```

```
TOOL_RESULT is_error=True : Permission to use Bash with command echo T30MARKER has been denied.
19:07:45 A allow
```

Run C7, a `permissions.ask` rule and no hooks, which in a headless session has nobody to ask:

```
TOOL_RESULT is_error=True : Claude requested permissions to use Bash, but you haven't granted it yet.
```

Run C8, the same rule plus the allow hook:

```
TOOL_RESULT is_error=True : Claude requested permissions to use Bash, but you haven't granted it yet.
19:08:21 A allow
```

The hook fired and answered in a shape the binary clearly understands, since the identical shape
carrying `deny` produced verbatim reason strings in C1 through C4. It just does not win. The
function that consumes the hook's verdict says why:

```js
async function Rro(e,n,r,o,d,f,_){
  if(WXe(n))return{decision:{behavior:"allow",updatedInput:r},input:r};
  let v=o.requireCanUseTool;
  if(e?.behavior==="deny")return t(`Hook denied tool use for ${n.name}`),{decision:e,input:r};
  if(e?.behavior!=="allow"&&e?.behavior!=="ask")return{decision:await d(n,r,o,f,_),input:r};
  let C=e.behavior,I=e.updatedInput??r,
      F=await WI(n,I,{...o,toolUseId:_},{hookUpdatedInput:e.updatedInput});
  if(F?.behavior==="deny")return t(`Hook returned '${C}' for ${n.name}, but deny rule overrides: ${F.message}`),{decision:F,input:I};
  if(F?.behavior==="ask"){ ... `but ask rule/safety check requires full permission pipeline` ... }
  if(C==="allow"){ if(v) ... `but canUseTool is required` ... else `Hook approved tool use for ${n.name}, bypassing permission prompt` }
```

A hook deny returns immediately and the rule engine is never consulted. A hook allow is checked
against the rules anyway, a deny rule overrides it, and an ask rule pushes the call back through the
full pipeline. The only thing a hook allow buys is the last branch, skipping an approval prompt for
a call no rule refuses. Add the confined-session filter quoted in Q2 and the picture is complete:
**a `claude` hook can subtract, and the most it can add is one prompt's worth of convenience.**

### `cursor-agent`, inert by construction

The merge function computes `"allow"` when every hook allows, and nothing reads it.
`grep -o '"allow"===[^,;)]\{0,60\}' 190.index.js` returns no matches at all. Every consumer checks
one or two values and ignores the rest, for example the shell path:

```js
if("deny"===(null==i?void 0:i.permission)){const e=H("Command execution",i.user_message);throw new S(e)}
if("ask"===(null==i?void 0:i.permission))return(0,n.$bB)(i.user_message)
```

An allow is the absence of a veto. It grants nothing. There is also nothing local to grant past,
since print mode already has every tool.

### `agy`, measured

Ticket 26 found that a file write is auto-denied in headless mode because nobody can be prompted.
Run A4 puts a hook allow in front of exactly that refusal, with `matcher: "*"` so it covers whatever
the write tool is called:

```json
{"t30-gate": {"PreToolUse":[{"matcher":"*","hooks":[
   {"type":"command","command":"<rig> AGY-ALLOWALL agy allow","timeout":10}]}]}}
```

```
cd t30/aws1 && agy --add-dir t30/aws1 \
  -p "Create a file named t30-agy-write.txt in the workspace containing the single line HELLO. Use the file writing tool, not the shell. If it is blocked, stop immediately and do not investigate." \
  --output-format stream-json
```

```
STEP 2 write_to_file state=ERROR
  err={"type":"TOOL_ERROR","message":"permission check failed for write_file \".../t30/aws1/t30-agy-write.txt\": user denied permission for write_file(...)"}
stderr: jetski: no output produced — a tool required the "write_file" permission that headless mode
        cannot prompt for, so it was auto-denied. Add an allow-rule under permissions.allow in
        settings.json (e.g. write_file(<target>)). Alternatively, re-run with
        --dangerously-skip-permissions to auto-approve all tools.
19:22:48 AGY-ALLOWALL allow
```

`ls t30/aws1/t30-agy-write.txt` afterwards: no such file. The hook allowed and the permission layer
refused anyway.

One detail for ticket 27's capability table. The step is named `write_to_file` and the permission
rule for the same action is `write_file`. A matcher written against one name will not match the
other, so the mapping table needs both.

## Q4: what Cursor's deduplication does to this

The mechanism, from `190.index.js`:

```js
getHookKey(e){return(0,_.uu)(e)?`prompt:${e.prompt}`:`command:${e.command}`}
dedupeClaudeHooksAgainstCursorHooks(e){
  const t=[e.enterpriseHooks,e.teamHooks,e.userHooks,e.projectHooks],o=new Map;
  ... // collect every key, grouped by event name
  n(e.claudeUserHooks,"claude-user"),n(e.claudeProjectHooks,"claude-project"),n(e.claudeProjectLocalHooks,"claude-project-local")
}
```

with the dropped entries logged at info level as `Removed duplicate ${t} hook for ${n}: ${o}`.

Three properties fall out. The key is the command string alone, with no source, no cwd and no
matcher in it. Only the four Cursor-format sources contribute keys, and only the three Claude-format
sources get filtered, so a Cursor-format hook can never be dropped. And the grouping is per event
name, so a Claude `PreToolUse` translated to `preToolUse` only collides with a Cursor-format hook
declared on `preToolUse`, never with one on `beforeShellExecution`.

Run U5 confirms it. The identical command string in both the project Cursor config and the project
Claude config, on the same event:

```json
// t30/cws/.cursor/hooks.json
{"version":1,"hooks":{"preToolUse":[{"command":"<rig> DUP cursor allow"}]}}
// t30/cws/.claude/settings.json
{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"<rig> DUP cursor allow"}]}]}}
```

```
{"success": {"command": "echo T30MARKER", "exitCode": 0, "stdout": "T30MARKER\n", ...}}
19:18:59 DUP allow
```

One fire, not two. The Claude-format copy was dropped.

**This cannot decide the question by accident.** Two hooks only collapse when their command strings
are byte-identical, and byte-identical commands return the same decision, so the surviving one says
whatever the dropped one would have said. The one place that reasoning could break is
`getCwdForSource`, which gives each source its own working directory. A command written as a
relative path could resolve to a different file depending on which source survived. Hydra writes
absolute paths, which ticket 26 already forced for a different reason, so this stays theoretical.

`claude` deduplicates too, which ticket 26 did not record. From the binary:

```js
function MZ(e,n){return `${e.pluginRoot??e.skillRoot??""}\x00${n}`}
...
j=Array.from(new Map(B.filter(je=>je.hook.type==="command").map(je=>[MZ(je,SW(je.hook)??""),je])).values())
```

A hook declared in settings has no plugin or skill root, so its key is the command string alone and
identical commands collapse across user, project, local and `--settings` alike. The verbose log line
is `Matched N unique hooks for query "..." (M before deduplication)`. Hydra's command string carries
a per-run path, so a collision needs the user to have written the same per-run path.

Two more `claude` facts worth carrying, from the same reading. `Policy disableAllHooks: skipping
configured hooks` shows a managed policy file can switch every configured hook off, and
`permissionDecision=defer` exists as a fifth decision that only works in print mode and only for a
solo tool call.

## Q5: `agy`'s `hook_deny_unless_prior_grant` and what counts as the grant

Ticket 26 concluded that `ask` and `force_ask` collapse to deny for a detached child and that
`permissionOverrides` did not reopen them, so a hook's usable decisions are `allow` and `deny`.
**That is true under the default permission mode and not true in general.** The collapse is the
permission layer's answer, not the hook's, and a grant in the permission layer changes it.

Run A5a, a hook returning `{"decision":"ask"}` with `--dangerously-skip-permissions`:

```
cd t30/aws1 && agy --add-dir t30/aws1 --dangerously-skip-permissions \
  -p "Run the shell command: echo T30MARKER. If it is blocked, stop immediately and do not investigate." \
  --output-format stream-json
```

```
INIT permission_mode=always-proceed tools=57
STEP 2 run_command state=ACTIVE params={"CommandLine": "echo T30MARKER"}
STEP 2 run_command state=DONE params={"CommandLine": "echo T30MARKER"}
19:23:08 AGY-ASK ask
```

The command ran. Under `request-review`, which is this machine's default and what ticket 26 measured,
the same `ask` is denied with `permission check failed ... user denied permission`. So the symbol
name is accurate: the hook defers, and the answer depends on whether a prior grant exists.

The grant that reopened it came from the command line. **A grant from the user's own config could
not be probed**, because `agy`'s three permission scopes all live in the user's home and
`/permissions` reports all three empty on this machine:

```
cd t30/aws1 && agy --add-dir t30/aws1 -p "/permissions" --output-format json
usage total: 0
{"permissions":[{"scope":"project"},{"scope":"shared"},{"scope":"global"}]}
```

Creating one means writing `~/.gemini/antigravity-cli/settings.json` or a file under
`~/.gemini/config/projects/`, which this ticket does not do. There is no workspace-local settings
file to use instead. `agy --help` has no `--settings` equivalent, and the binary's own changelog
places permission merging in `~/.gemini/config/projects/` and `~/.gemini/antigravity-cli/settings.json`
only. Recorded as a gap rather than guessed.

The gap does not reach the ticket's question, because run A6 closes it from the other side. A hook
returning `ask` and a hook returning `deny` on the same call, with `--dangerously-skip-permissions`
so the permission layer is as open as it goes:

```json
{"t30-gate": {"PreToolUse":[{"matcher":"run_command","hooks":[
   {"type":"command","command":"<rig> AGY-ASK agy ask","timeout":10},
   {"type":"command","command":"<rig> AGY-B agy deny","timeout":10}]}]}}
```

```json
{"event":"step_update","step_update":{"conversation_id":"0030b806-89c9-470a-9248-1fea6348a07d",
 "step_index":2,"state":"ERROR","step_type":"tool","tool_name":"run_command","duration_seconds":0.079984,
 "tool_info":{"name":"run_command","parameters":{"CommandLine":"echo T30MARKER"},
  "error":{"type":"TOOL_ERROR","message":"tool call denied by pre-tool hook: t30 AGY-B says deny"}}}}
```

```
INIT permission_mode=always-proceed tools=57
19:23:36 AGY-ASK ask
19:23:36 AGY-B deny
```

**A hook deny beats an `ask` that a prior grant would otherwise have satisfied, and beats
`always-proceed` at the same time.** Whatever a prior grant can reopen, it cannot reopen past a
deny. That re-verifies ticket 26's `bypassPermissions` result on 1.1.25 as a side effect.

## One more thing the probes settled: Cursor reads Claude's response dialect

Ticket 27 decided the hook script is told which dialect to answer in, because a wrong dialect is an
open door on `claude`. That reasoning still holds, and one of its assumptions has moved.

The CLI builds its hook executor with the compatibility flag on. From `227.index.js`, `3385.index.js`
and `5421.index.js`:

```js
new S.WL(...,{enableClaudeNestedHookSpecificOutputCompatibility:!0,commandHookPayloadTransport:o})
```

and the translation, in the bundled hooks module, applies to `preToolUse` only:

```js
i = e===r.preToolUse ? function(e,t){
      ...
      if(!W(r,"PreToolUse"))return n;
      const i=Object.assign({},n);
      void 0!==r.permissionDecision&&(i.permission=r.permissionDecision,s=!0),
      "string"==typeof r.permissionDecisionReason&&(i.user_message=r.permissionDecisionReason,s=!0),
      ...
```

Run U6 confirms it. A Cursor-format `preToolUse` hook answering in Claude's nested shape:

```json
{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"t30 CCDIALECT says deny"}}
```

```json
{"rejected": {"command": "echo T30MARKER",
  "reason": "t30 CCDIALECT says deny\n\nAgent note: Do not suggest workarounds to the blocked tool.",
  "isReadonly": false}}
```

So one response dialect covers `claude` and `cursor-agent`, as long as the Cursor hook is wired to
`preToolUse` rather than `beforeShellExecution`, which gets no translation. That does not change
ticket 27's decision to pass the dialect explicitly, since `agy` still needs its own shape and
guessing is still an open door. It does mean the script has two answers to produce rather than three.

## The exit code is still a lie

Every blocked run in this ticket exited 0 and reported success. All seven `agy` runs came back
`status: SUCCESS`, including the six where the gate refused everything, and Cursor's result frames
carry `"subtype": "success", "is_error": false` over a `rejected` tool call. Ticket 14's hazard and
ticket 26's confirmation of it hold on today's builds without exception.

## What I could not measure, and why

Seven gaps, all of them the same shape: the config lives in the user's home and this ticket does not
write there.

1. **A `claude` user-scope hook returning allow.** Would need `~/.claude/settings.json` edited. The
   deny direction was measured live in C9. The aggregation code carries no source information, so
   the allow direction is settled by construction rather than by measurement.
2. **A `cursor-agent` user-scope hook returning allow**, at `~/.cursor/hooks.json`, which does not
   exist and would have to be created in the user's home. The deny direction was measured live in
   U7 through the claude-user scope. Cursor's merge is symmetric and source-blind, same argument.
3. **Cursor's enterprise scope**, `/Library/Application Support/Cursor/hooks.json`. Needs root and
   is a machine-wide change. Not attempted.
4. **Cursor's team scope**, which turns out to be `~/.cursor/managed/active-team-hooks/hooks.json`.
   In the home directory, so not attempted. It joins the same merge as every other source.
5. **`agy`'s shared and global hook files**, `~/.gemini/config/hooks.json` and the settings under
   `~/.gemini/antigravity-cli/`. The two-root probe in A7 exercises the same cross-config merge
   path from directories Hydra owns.
6. **An `agy` prior grant from the user's own `permissions.allow`.** All three scopes are in the
   home directory and all three are empty today. The command-line grant was probed instead, in A5a.
7. **`agy`'s execution order across separately named hooks.** It is not the file order, and pinning
   it down would cost paid runs to answer a question whose outcome does not change the decision. It
   does mean the identity of the hook that answers first is not stable across runs.

None of these can flip the answer without also contradicting a merge rule read out of the shipped
binaries on two providers and measured five ways on the third.

## The answer to the ticket's framing

**A deny beats an allow, on all three providers. The answer holds across declaration order and
across every scope I could set, and it holds when the two hooks answer in different dialects.**

The evidence for each half:

| | two hooks, both orders | cross-scope | vs the user's live config | can a hook widen |
|---|---|---|---|---|
| `claude` 2.1.257 | C1, C2: deny | C3, C4: deny | C9: user deny beats CLI allow | no, C6 and C8 |
| `cursor-agent` 2026.09.02 | U1, U2: deny | U4a, U4b: deny | U7: user deny beats project allow | no, nothing reads allow |
| `agy` 1.1.25 | A2, A3: deny | A7: deny | not probeable, see gaps | no, A4 |

**Ticket 27's composition decision stands as written.** The user's hooks compose with Hydra's, they
can only tighten Hydra's floor, and the sentence that ticket flagged as its one load-bearing
unmeasured fact is now measured. Nothing about ticket 27 inverts. `--restricted` does not become
mandatory on `claude`. Ticket 22's rule is not triggered, the routable set stays at three, and
nobody has to design a config-root mechanism.

The result is stronger than composition needed, and the stronger form is what should go in the
harness spec: **on all three providers the hook layer is a veto and cannot grant.** A user hook
cannot widen Hydra's policy because no hook can widen anything. That removes a class of failure
rather than bounding it.

Three things the spec should carry alongside the answer.

**The decider recorded is not always Hydra's.** On `agy` the first deny short-circuits, so when the
user's hook refuses first, Hydra's hook never runs and the reason in the tool error belongs to the
user. On Cursor every hook's message is concatenated into one reason string and the model may quote
the wrong half, as it did in U7. `permission_decision` with decider `hook` is derivable, but the
hook it names is whichever one the provider chose to report.

**`--setting-sources` deserves a line in the spec next to `--restricted`.** It dropped the user's
hooks while keeping 25 tools, `bypassPermissions` and Hydra's own `--settings` hooks. Composition is
safe, so isolation is a choice rather than a requirement, and this is the cheap way to take it.

**Ticket 26's Cursor path table needs one row corrected.** The team hook file is in the user's home,
not the workspace.

## Costs

Free: every `agy` `/hooks` and `/permissions` probe, all bundle and `strings` reads, all version and
help output. The `usage` block returned zeros on each `agy` slash-command run.

Paid:

```
claude          13 runs, sonnet. Captured usage from the 4 runs that were teed to disk:
                10,167 cache-creation, 133,466 cache-read, 739 output.
cursor-agent     8 runs. 51,248 input, 175,364 cache-read, 1,220 output.
agy              7 runs. 124,883 input (65,049 of them cache reads), 2,816 output,
                 1,861 thinking. 127,699 total.
```

Ticket 26's lesson paid off. Every `agy` prompt ended with "If it is blocked, stop immediately and do
not investigate", and no run repeated its 86,944-token investigation of its own failure.

## What the CLIs wrote, and what was cleaned up

No user-scope config was modified or created. Verified after the last probe:

```
~/.claude/settings.json                  unchanged, mtime Aug 31 22:57, 1510 bytes,
                                         sha1 7fd1e3d3637f3f62dc77d77268afebba015ae4a5
~/.gemini/antigravity-cli/settings.json  unchanged, mtime Sep 1 05:28, 82 bytes
~/.gemini/config/config.json             unchanged, mtime Sep 1 05:28, 279 bytes
~/.cursor/hooks.json                     does not exist, was never created
~/.cursor/managed/                       does not exist, was never created
~/.gemini/config/hooks.json              does not exist, was never created
```

`find ~/.claude ~/.cursor ~/.gemini -name "*t30*"` returns only the providers' own session stores,
which are normal run residue and were left in place rather than deleted:

```
~/.claude/projects/-private-tmp-...-scratchpad-t30-ws/
~/.cursor/projects/private-tmp-...-scratchpad-t30-cws/
~/.gemini/antigravity-cli/brain/<one uuid per agy run>/
```

Nothing was written under `~/hydra` outside `.scratch/hydra/research/`.

## Recommendation

No grilling ticket. This one had a single yes-or-no question holding up ticket 27, the answer is yes
in every configuration reachable without touching the user's files, and two of the three providers
ship the rule as code that says the same thing the measurements do.

One thing worth a line in someone's ticket rather than a ticket of its own: the recorded decider.
Ticket 09's `permission_decision` assumes the denial names who denied. On `agy` and on Cursor it
names whoever the provider felt like naming, and on Cursor the model may narrate the hook that
allowed. If the record is going to carry a policy identifier, the identifier has to be read from a
field Hydra controls rather than parsed out of the reason string.
