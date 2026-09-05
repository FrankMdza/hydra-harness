# 01 - The policy set, the capability table, and the gate compiler

Type: build
Status: open
Blocked by:

## Goal

The five policies as files, the pattern floor beside them, a capability table for
`claude`'s tools, and a compiler that turns a policy name into a gate script. No
provider is launched in this ticket. The script is fed synthetic hook payloads on
stdin and its answers are asserted.

## Where the design says it

- Harness spec 7.1, capabilities rather than tool names, and why there is no deny list.
- 7.2, the five policies. 7.3, monotonic narrowing, which is a pre-flight check rather
  than part of the script. 7.4, the pattern floor and its initial patterns.
- 7.5, the script itself: generated per spawn with the policy compiled in, never
  reading a policy file at fire time, and the seven paths denied under every policy.
- 7.6, in-process subagents, denied under every policy but `permissive`.
- 15.1 and 15.2, the five protected configs and the twelve protected paths, with the
  two-column split between admission refusal and run-time denial.

## Decisions this ticket makes, and they are implementation rather than design

**Where the policy directory lives.** It is protected config #1 and `policy_sha` is its
sha, so it is a directory in this repo rather than under `~/hydra`. `policies/` holding
five files plus the floor is the obvious shape.

**What language the script is.** One script, policy compiled in, dialect passed as an
argument, no callback into `hydra`, which 7.5 rules out for two reasons. Python is the
likely answer: it needs JSON and path matching, macOS ships it, and the user's own 418
lines of guards are already Python. Say which and why in `IMPLEMENTATION.md`.

## Done when

- `policies/` holds the five policies and the floor, and `policy_sha` is computable.
- A compiler turns a policy name into a script with no file reads at fire time.
- The script answers in `claude`'s dialect and denies when the dialect argument is
  absent, because `claude` allows a response in a shape it does not recognise.
- Tests feed it payloads and assert both directions per policy: the seven universal
  path denials hold under `permissive` and `curate`, `read-only` refuses a write,
  `standard` allows one inside the worktree and refuses one outside, `judge` refuses
  everything, and every policy but `permissive` refuses `Task`.
- The floor's git, AWS and Slack patterns are tested, including that each consumes `-C`
  and `--git-dir` before reading the subcommand.

## Watch for

**The floor is a deny list and the spec says so out loud.** It has that shape's failure
mode, silent permission on the spelling nobody wrote down. Do not test it as if it were
coverage, and do not let a passing test become a claim it does not make.

**`claude` fails open on a response dialect it does not recognise.** That is the one
provider where a script that guesses wrong is an open door, which is why the dialect is
an argument rather than something inferred.

**`claude`'s init frame lists all 25 tool names for free**, measured 2026-09-05 on
2.1.261. Section 7.1 claims that for `agy` and it is true here too, so the run-time
check that a tool name the table does not recognise is denied and recorded costs
nothing on this provider either.
