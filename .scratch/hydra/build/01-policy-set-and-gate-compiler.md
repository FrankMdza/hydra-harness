# 01 - The policy set, the capability table, and the gate compiler

Type: build
Status: resolved
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

## Outcome

Built, 2026-09-05. `policies/` holds the five plus `floor.json` and a README,
`src/policy/mod.rs` loads and hashes them and compiles the gate,
`src/policy/table.rs` is the capability table for `claude` 2.1.261's twenty-five
tools, and `src/policy/gate.py` is the script template. `hydra::policy::write_gate`
writes it executable. No provider was launched.

67 tests pass, clippy and `cargo fmt --check` clean. 23 of them are `tests/gate.rs`,
which compiles a real gate per policy and feeds it synthetic `PreToolUse` payloads
in the shape ticket 26 measured, covering every row of the ticket's "Done when":
the seven path denials under `permissive` and `curate`, `read-only` refusing a
write, `standard` allowing one inside the worktree and refusing one outside,
`judge` refusing everything, every policy but `permissive` refusing `Task`, and the
floor's git, AWS and Slack patterns including `-C` and `--git-dir` consumption.

Provider versions re-read at the top of the session, all three unchanged from what
`IMPLEMENTATION.md` records: `claude` 2.1.261, `cursor-agent` 2026.09.02-c22c1a3,
`agy` 1.1.26.

### The two decisions the ticket asked for

**The policy directory is `policies/` in this repo**, as the ticket expected.
`policy_sha` is a sha256 over every `.json` in it, in name order, each file's name
and body framed by its length so two directories cannot collide by concatenation.
`README.md` is excluded: a wording fix should not move a value every record carries.

**The script is Python**, for the ticket's own reasons. What the ticket did not
anticipate is the shebang: `/usr/bin/python3` and not `/usr/bin/env python3`.
`env` resolves through the child's inherited PATH, and on `claude` a hook that
fails to start fails open. That is the one dependency this script cannot afford.

### What the ticket assumed that turned out incomplete

**Six capabilities do not cover twenty-five tools.** The ticket inherited the
coverage matrix from research 26, six capabilities across three providers, and
`claude` offers nineteen tools that answer to none of them: `CronCreate`,
`SendMessage`, `ToolSearch`, `Skill` and the rest. 7.1 denies a name the table does
not recognise, so a six-capability table would have denied nineteen tools under
`permissive`, which is supposed to allow everything. They share one row,
`provider_native`, granted by `permissive` alone. **This means `standard` denies
`ToolSearch` and `Skill`**, a real loss on coding runs, in the direction 7.1 chose
on purpose. Ticket 02's first gated run is where it shows up, and the fix, if the
records ask for one, is a policy file edit rather than code.

**Four of the five protected configs have no file.** 7.5's seven denied paths
assume five config files exist. Two of the seven are real today, `policies/` and
`~/hydra/runs/`, and the gate script is written per spawn. The environment
allowlist is still Rust code in `src/env.rs`, and the export allowlist, the
preamble and the canonical MCP document belong to tickets this build has not
reached. All four are named at `config/*` and denied by path anyway, because
denying a write to a path holding nothing is the same denial it will be the day the
file lands.

### What the tests caught, and it was not a style point

**The gate script was writable under `permissive`.** `std::fs::canonicalize` fails
on a path whose file does not exist, and the gate script is exactly that at compile
time. The first version fell back to the unresolved path, leaving `/var/folders/...`
compiled in while the script resolved `/private/var/folders/...` at fire time, so
the two never matched. Every protected path under a symlinked directory had the same
hole. Fixed by resolving the longest existing ancestor and re-appending the tail,
which is what Python's `realpath` does on the other side.
`the_path_denials_hold_under_permissive_and_curate` is the regression test; it fails
without the fix.

**The suite was testing the wrong interpreter.** `python3` on this shell's PATH is
Homebrew's 3.14.7 and the gate runs on `/usr/bin/python3`, 3.9.6. The harness now
names the absolute path, so the suite measures the interpreter that will actually
run. Same class as the build plan's provider-drift note, one layer down. Nothing
broke on 3.9.6, which is worth knowing rather than assuming.

### One thing that is a decision rather than an implementation

`curate`'s `skill_store` scope is `~/hydra/skills/`, which is what harness spec 7.2
says in as many words. `docs/ouroboros-spec.md` 3.6 has the curator working in its
own git worktree of the store instead, and **9.1 already flags that as a refinement
to confirm rather than assume**. Beside it sits a question 9.1 does not quite name:
3.7 has the curator commit to `ouroboros/curate/<run-id>` in the store's repo, and
`curate` grants no `shell`, so either the supervisor commits or the policy needs
something it does not have.

This ticket did not decide it. The literal reading of 7.2 is implemented, the
question belongs to ticket 12 or to a reopened map, and nothing in the build is
blocked by it today.
