# What is built

`docs/` is the design, finished and checked. This file is the gap between it and the
code, kept current by whoever narrows the gap.

Read it before trusting a run record written by this binary. **A record whose `policy`
is null was written before the gate reached a provider.** Ticket 02 wired it, and every
record written after that carries the policy that was in force and the sha of the
directory it came from.

**`hydra_version` does not separate the two.** It used to be the field this file pointed
at, and three gated records were written under `0.1.0` during ticket 02's own
measurement, before the version moved to `0.2.0`. A null `policy` is the test that holds.

## What the gate does about `shell`, and what it still does not

Harness spec 7.5 states seven path denials as delivered outright: "a hook sees the path
and refuses the write before it reaches disk". The gate delivers that for tools that
carry a path in their arguments. `Bash` carries none, `shell` is unscoped in every
policy that grants it, and no argv match can decide what a shell command will write.

**Ticket 02's code review found all seven reachable by redirect**, the run's own gate
script included, which is 7.5's induction base case. That one composed into a full
bypass: an emptied script exits 0 printing nothing, and 6.6 measured `claude` failing
open on a response in a shape it does not recognise, so two `Bash` calls under
`standard` ended the gate for the rest of the run. `--permission-mode
bypassPermissions` had removed the layer previously refusing those calls, so the flag
and the hole arrived in the same change.

Two mechanisms now stand where none did. They are worth telling apart, because one is a
floor and one is not.

**A shell command that names one of the seven is refused whole.** Every token of every
segment is expanded and resolved before the comparison: `~` and the variables the
child's own environment carries, the tail of a `--flag=/path` option, and the tail of a
`D=/path` assignment, which is where a variable the command sets itself is written down.
Measured on run `01a07340-67a3-7c41-8c69-8d1e28bb58a7`, `standard`, on a real `claude`:
a `Bash` append to `~/hydra/runs/` was refused with `hydra/protected-path`, the reason
reached the model, no file was created, and a second `Bash` call writing inside the
worktree in the same run was allowed and landed.

**This one is a floor and it has 7.4's failure mode.** A path the command never writes
down still walks past. `P=<parent>; echo x > $P/runs/r.jsonl` is the shape: the variable
holds the parent, no token is a protected path, and no expansion the gate can do
produces one. Catching it means evaluating the command, which 7.4 rules out.
`tests/gate.rs` pins it so that closing it breaks a test on purpose.

**The base case does not rest on that floor.** The hook config carries a sha256 of the
gate script, computed from the bytes that reached the disk, and the command checks it
before running the script. All three providers snapshot the hook config at process
start (6.6) while 7.5 has the script read fresh on every invocation, so the sha is fixed
before the child can run anything and a rewritten or deleted script fails **closed**,
answering `hydra/gate-tampered` in the provider's own dialect. That holds whatever
spelling reached the script, which is what makes it a mechanism rather than a floor.
The script still lives where 7.5 puts it; only the pointer got stricter.

Measured on run `01a07357-58a5-79e2-a25a-3df578a750b4`: `claude` runs the 717 character
hook command and the gate answers through it. `tests/spawn.rs` empties the script and
then deletes it, and asserts the refusal both times.

**The refusal is coarse and reads are refused too.** Which token is a redirect target is
not decidable without running the command, and neither is which is an argument to a
writing binary, so `cat ~/hydra/runs/r.jsonl` is refused under `curate` as well. That is
a false positive in the direction 7.4's floor already takes. The precise access is the
file tools, which carry a path: `curate` scopes `file_read` to the corpus and reads it
that way.

**What is left for the map.** Closing the residual needs a mechanism that does not read
argv, and each option trades something the design has argued about: denying `shell` its
unscoped grant rewrites 7.2's `standard`, and running the child under a second uid or a
sandbox is nowhere in the spec. The base case no longer waits on that answer, so what
remains at risk is a write to the policy directory or the corpus through a path the
command assembles.

Provider versions this slice was built and measured against, read 2026-09-05:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.261 | 2026-09-05. The specs were written against 2.1.257; every flag in harness spec 6.7 survives the move |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-05, unchanged since 2026-09-02 |
| `agy` | 1.1.26 | 2026-09-05, unchanged since 2026-09-04 |

## The commands

```
hydra run --provider claude --task "..." [--model <id>] [--policy <name>]
          [--timeout <dur>] [--allow-floor-escape]
hydra status [<run-id>]
hydra logs <run-id> [--follow]
hydra reap [<run-id>] [--delete-branch]
hydra prune [<run-id>...] [--delete] [--dry-run]
hydra curate [--model <id>] [--timeout <dur>] [--dry-run]
```

A launch builds the child's environment from the allowlist, reads the provider's
version, loads the policy set and the preamble, refuses if the concurrency cap is
already met, mints a UUIDv7 run id, creates a git worktree and seeds it with the
parent's uncommitted work, compiles the run's gate into `~/hydra/spawn/<run-id>/`,
composes `claude`'s argv and the whole of line 0, and hands all of it to a detached
half that spawns the provider on a pty, writes `run_start`, tees every frame verbatim
into `~/hydra/runs/<run-id>.jsonl` and writes `run_end` when the child exits. The
foreground tails that file, and on a run that ends inside its budget it reads the file
back through the normalizer and prints contract 17's JSON on stdout.

Section 9 gives `hydra run` eight optional flags and four are here: `--model`,
`--policy`, `--timeout` and `--allow-floor-escape`. The four that are not each belong
to a ticket that is not built: `--parent` and `--files` to delegation, `--shared-tree`
to a worktree question nothing has asked yet, `--allow-billed` to the raw-model plane.

Harness spec 17 lists eleven commands. Six are built. `hydra watch` is a twelfth and
is not one of them: it is the detached half of a launch, `hydra run` is the only thing
that types it, and `clap` hides it from the help for that reason.

`hydra reap` commits a checkout to the child's branch, then removes the checkout and
the run's spawn directory. `hydra prune` moves run files to `~/hydra/pruned/`. Neither
appends to or removes a run record the other owns, and the section below says what each
guarantees and how.

`hydra curate` is Ouroboros rung one: it reads the corpus, launches one recorded run
under the `curate` policy with what it found, and commits what that run proposed onto
a branch in the skill store that a human merges or does not. `--dry-run` reads the
corpus and launches nothing, which is the only half that works before ticket 07
creates the store.

`cursor-agent` and `agy` are routable in the design and refused here by name, with an
error that says the slice routes one provider rather than that the provider is
unknown.

## Cleanup: `hydra reap` and `hydra prune`

Ticket 06. Harness spec 10 gives both to a human to type and Ouroboros to neither, and
harness spec 7.5 already ruled out enforcing that with the gate: the binary is reached
through `shell`, so any rule against it is an argv match with the pattern floor's failure
mode. What holds it here is that nothing in `src/` calls `reap::reap` or `prune::prune`
except `main`.

**Reap's commit-first rule is enforced by git rather than by `src/reap.rs`.** That is the
whole point of it. `git commit` leaves the checkout clean, and only a clean checkout is
one `git worktree remove` will take without `--force`, which reap never passes. So a
commit that did not happen stops the removal instead of preceding it, and the guarantee
is not a sentence in a doc comment. Harness spec 10 calls this the Hermes lesson turned
on our own tooling.

**The branch survives whatever it carries, decided by the same mechanism.**
`git branch -d` refuses a branch that is not fully merged, so the commit reap just made
is exactly what keeps the branch alive. A branch with nothing on it is litter and goes.
`--delete-branch` is `git branch -D`, and it is the same shape prune gives deletion:
recoverable by default, destructive only when a human typed the flag.
`docs/ouroboros-spec.md` section 4.11 already expects a leftover branch to be removed
with `git branch -D`; the flag is that sentence typed through the command.

**Only the branch Hydra named for the run is ever deleted.** A child that checked out one
of the human's own branches gets its work committed there and the branch left alone.

**A branch that survives is reported in git's own words.** `git branch -d` refuses for two
causes that want different answers: the branch holds commits nothing else has, or it is
checked out in another worktree. The second is refused by `-D` as well, so a report that
said "it holds commits" both times told the human the wrong reason and handed them a flag
that cannot help.

**A detached HEAD is refused rather than reaped.** A commit on one is reachable from
nothing once the checkout is gone, which is the single shape where committing first saves
nothing, so reap leaves the pair and says why.

**Both commands take the launch grace period as an argument, the way
`status::abandoned_worktrees` does, and `main` passes `run::LAUNCH_BUDGET`.** A checkout
is created and seeded before its run file exists, and the file itself exists before line 0
does, so for ten seconds a launch in progress has a checkout and an empty record and looks
exactly like a run whose record was pruned. `RunView::launched` plus the age is what tells
them apart; the section on `hydra status` below has the detail. Reaping there removes the
checkout out from under a child that is starting, and pruning there moves the record out
from under the writer that has it open, so the launcher waits ten seconds for a line 0
being written into `pruned/` and then reports a running run as one that wrote nothing.
`--delete` destroys it outright. Naming a run id skips the grace for reap, because the
grace is a launch window and a human who typed the id has answered that question; prune
keeps it even on a named run, because there the file is being written rather than merely
sitting there.

**Prune works on the path the scan found, not on a path rebuilt from a run id.**
`read_view` takes the run id off line 0 and only falls back to the filename, so a record
whose envelope disagrees with its name would otherwise have the sweep move a file that is
not there, and the named form find no view and skip the liveness check entirely.

**A move that cannot finish takes its copy back out.** The rename is what happens on one
filesystem; the copy-and-remove fallback is for a home directory spanning two mounts. A
copy that lands followed by a remove that fails would leave the record in both places, and
the destination check would then refuse every retry as an overwrite, turning one bad
minute into a permanent refusal.

**A sweep walks `spawn/` as well as `worktrees/`.** Harness spec 5 reaps the pair, and
the pair is missing on both sides in practice: the three oldest runs on this machine
predate the spawn directory, and a run outside a repo gets a spawn directory and no
checkout at all.

**Prune moves to `~/hydra/pruned/<run-id>.jsonl`, and the destination is this build's
choice.** Harness spec 5's table says prune "moves rather than deletes" and names no
destination, the same gap 19.1 leaves for the filenames inside the spawn directory. A
sibling of `runs/` keeps the move a rename on one filesystem, and the run id stays on the
file, so putting a record back is a `mv`. A destination that already holds that run id is
a refusal rather than an overwrite. `pruned/` is in `.gitignore` beside `runs/`.

**Prune with no run ids takes every run that is not live, and that is safe because the
default is a move.** The command that destroys something needs a flag; the command that
tidies does not need a list. A live run is never pruned under either form: contract 6
gives a run file one writer, and a rename does not take the file away from an open
descriptor, so the writer would keep appending into a path nothing looks in.

**Typing `hydra prune` on this machine would empty the corpus
`tests/record_contract.rs` walks.** That test reads every `.jsonl` under `~/hydra/runs/`
and returns quietly when there are none, so a prune sweep costs the check silently. The
records are the evidence and they are cheap; prune the ones you have finished with by
name.

**Reap, then prune.** Reap derives liveness from the pid in each run record, so pruning
first leaves it reading records that are no longer there.

## The policy set and the gate, on a run

Tickets 01 and 02. `policies/` holds the five policies of harness spec 7.2 and the
pattern floor of 7.4, `src/policy/` loads and hashes them, `hydra::policy::write_gate`
compiles a policy name into the script of 7.5, and every `hydra run` writes one into
`~/hydra/spawn/<run-id>/` and passes it to `claude` on `--settings`. `tests/gate.rs`
feeds the compiled script synthetic hook payloads and asserts its answers, 23 of them;
`tests/spawn.rs` checks that a run writes the script where the run will find it.

**The `PreToolUse` matcher is `*`, and one entry covers every tool.** `claude` routes
reads through `PreToolUse` like everything else, so one event is enough on this
provider. Cursor is the one that needs `beforeReadFile` as well, and that is ticket 09.
The wildcard is documented rather than measured in the research, so ticket 02 measured
it: the gate fired on `PreToolUse:Write` on all three probes below.

**The hook command carries the dialect and the script path is shell-quoted.** The
config's `command` is a string a shell runs, and the spawn directory sits under a home
directory whose name Hydra does not choose. A space in it would split the command and
run the gate with no `argv[1]`, which denies in two shapes and exits 2. That fails
closed, but it fails closed on every call of every run, so the quoting is what keeps it
from being a silent outage.

**No hook `timeout` is set**, so `claude`'s default applies. A hook that fails to answer
fails open on this provider, and inventing a shorter budget than the default only moves
the same failure closer.

**The script is Python, and its shebang is `/usr/bin/python3` rather than
`/usr/bin/env python3`.** The language is what 7.5's ticket expected: the gate needs
JSON and path matching, macOS ships an interpreter, and the user's own 418 lines of
`PreToolUse` guards are already Python. The absolute path is the part worth stating.
`env` resolves through the child's PATH, the child's PATH is inherited, and on `claude`
a hook that fails to start fails **open** (6.6). A PATH lookup is the one dependency
this script cannot afford. `/usr/bin/python3` is 3.9.6 on this machine, read 2026-09-05.

**The suite runs the gate on that interpreter and not on the shell's.** `python3` on
this shell's PATH is Homebrew's 3.14.7. Testing it would leave the interpreter that
actually runs the gate unmeasured, which is the drift the build plan's second note
warns about, one layer down from the providers.

**`policy_sha` is a sha256 over every `.json` in `policies/`, in name order, each file's
name and body framed by its length.** `README.md` is left out: it is prose about the
policies rather than one of them, and a wording fix should not move a value every run
record carries. The framing is what stops two directories colliding by concatenation.

**Nineteen of `claude`'s twenty-five tools answer to no capability the five policies
speak in.** 7.1 denies a tool name the table does not recognise, so a table covering
only the six capabilities of the coverage matrix would deny nineteen tools under
`permissive`, which is supposed to allow everything. They map to one capability,
`provider_native`, granted by `permissive` alone. One row rather than nineteen invented
categories, and 7.1's drift check still fires on a twenty-sixth name. Splitting it is a
design decision and this build did not make it.

**`standard` therefore denies `ToolSearch` and `Skill`.** That follows from the
paragraph above and it is a real capability loss on ordinary coding runs, in the
direction 7.1 chose on purpose: an allowlist fails toward a run that cannot do its job,
which someone notices. Ticket 02's first gated run is where it gets noticed, and the
fix, if the records ask for one, is a policy file edit rather than code.

**Three protected configs are still paths rather than files.** 7.5 denies writes to
seven paths under every policy. Four exist now: `policies/`, `~/hydra/runs/`, the
per-spawn gate script, and `config/preamble.md`, which ticket 02 wrote. The other three
are named at `config/env-allowlist.json`, `config/export-allowlist.json` and
`config/mcp.md`, and **none of those files exists**. The environment allowlist is still
Rust code in `src/env.rs`, which 15.2 protects at admission rather than at run time, so
the config the table means has no file. Denying a write to a path that holds nothing is
the same denial it will be the day the file lands, which is why the names went in early
rather than being added later and forgotten.

**Hydra finds its own policy directory through `CARGO_MANIFEST_DIR`, so the binary
only works from its source tree.** `PolicySet::discover()`, `preamble::path()` and the
four `config/` paths are compiled against the manifest directory, which is right for a
binary run out of
`target/debug` and wrong for an installed one. A copied `hydra` would compile gates
against a `policies/` that is not where it thinks. Nothing has needed an installed
binary yet, and the answer is not obvious, since the policy directory is protected
config #1 and a run must not be able to point Hydra at a different one. Naming it here
rather than leaving it as a doc comment in `src/policy/mod.rs`.

**A path is compared after resolving the longest ancestor that exists.** Plain
`canonicalize` fails outright on a path whose file has not been created, and the gate
script is exactly that at compile time. The first version fell back to the unresolved
path, which left `/var/folders/...` on one side and `/private/var/folders/...` on the
other, so **the gate script was writable under `permissive`** until `tests/gate.rs`
caught it. Any protected path sitting under a symlinked directory had the same hole.
The gate script resolves the same way at fire time, with `realpath`, and both sides
lowercase because 15.2 matches case-insensitively.

**The floor refuses rather than evaluates, and the refusal is coarse.** 7.4 says a
pattern must refuse a subcommand arriving from a shell substitution. Which segment a
substitution feeds is not decidable without running it, so a command carrying `$(`,
`${` or a backtick **and** naming git, aws or Slack anywhere in it is refused whole.
`echo $(date) && git status` is refused. That is a false positive on purpose, and
`--allow-floor-escape` is the human's way past it.

**`git push` with no arguments is not checked against the protected branches.** The
floor cannot know the current branch from argv. It is the failure mode 7.4 names out
loud, written down here so nobody reads the passing tests as coverage.

## The one departure from the record contract

**`skills` and `mcp` are absent, not null.** Nothing is injected, so which sha the run
pinned is not a question with a missing answer. Contract 5.5 set the rule and 5.6
applied it; this is the third place it applies.

Ticket 02 closed the other two. `policy` and `policy_sha` carry the policy that was in
force and the sha of the directory it was compiled from, and the preamble rides the task
string, so `run_start.task` is the preamble followed by a blank line and the caller's
task verbatim.

**`--allow-floor-escape` is not on the run record.** Harness spec 16 says a per-run
human flag "lands in the run record as an argument, so the record shows why a run was
allowed to do the thing", and `--allow-billed` has `auth.allow_billed` for exactly that.
The floor flag has no field, and contract 5.1 names none, so adding one is a contract
change rather than a line of code. Until it lands, a floor-lifted run is
indistinguishable in the record from one that was never offered the lift, and the
normalizer has to pass `floor_lifted: false` because there is no field to read. So a
shell call that a human lifted the floor for re-derives to a `hydra/floor` deny it never
got, and the same call then reports a gate leak, because nothing refused it. Ticket 03
left the gap where it found it: inventing a field is a contract change.

Everything else in contract sections 2 to 6 is met, and `tests/record_contract.rs`
checks it rather than asserting it. That test also walks every record under
`~/hydra/runs/` on this machine and fails if any of them drifted.

## Reading a record back

**One provider.** `src/normalize/` turns a stored `claude` run into contract 7's events.
`read_file` takes a path and the policy set; `read_with` takes text and a layout, which
is how the tests ask a real question about a synthetic run without touching `~/hydra`.
Nothing is written, which contract 11 requires and `tests/normalize.rs` checks by
comparing every byte under `runs/` before and after a walk.

**`usage` says `unavailable` for an object it cannot read, not zero.** A `result` frame
with no `usage`, and one whose `usage` holds none of section 9's four count fields, both
produce `source: unavailable` with no counts. Zeroes there would be 7.8's
null-against-zero confusion arriving through the other door.

**Eight of the ten types have a producer.** `message`, `thinking`, `tool_call`,
`tool_result`, `permission_decision` and `usage`, plus the two stored Hydra lines.
`message_delta` needs `--include-partial-messages` and 7.4 keeps it off in v1.
`correction` comes from an interactive harvest and this build launches no interactive
runs. Both types are in the enum, because a type the reader cannot name is a type nobody
knows is missing.

**`permission_decision` runs the run's own policy again**, compiled into the same gate
script 7.5 writes and asked about each recorded call. Two costs, both accepted: a
`/usr/bin/python3` process per tool call, and that interpreter is now a read-time
dependency as well as a run-time one. A second implementation of the rules in Rust was
the alternative and it is the wrong trade. The gate decides with the seven path denials,
the scope resolution, the `mcp__` prefix rule and a floor that tokenizes shell with
`shlex`, and a Rust copy that answered `allow` where the gate answered `deny` would
report a gate leak that never happened.

**`decider` reads the `hydra/` prefix and nothing else.** Every reason the gate writes
starts with the stable code ticket 01 added, so a `PreToolUse` deny whose reason carries
no prefix belongs to somebody else, which on this machine means the user's own guards.
7.7 forbids parsing reason *text* for attribution and this is not that: the prefix is a
code Hydra writes, the prose after it is what 7.7 is about.

**A hook frame carries no `tool_use_id`, so the corroboration is joined by tool name and
order, inside a window.** `hook_name` reads `PreToolUse:<Tool>` and the frame lands
between the `tool_use` block and its result. Ticket 02's run fired four `PreToolUse:Bash`
responses for one call, one Hydra's and three with an empty `output`, so filtering on a
parseable `permissionDecision` picks Hydra's out. **The window closes at the next call
that came later**, and that bound is load-bearing rather than tidy: without it the first
of two `Bash` calls takes the refusal a guard wrote about the second, and both decisions
come out wrong, one denied for somebody else's reason and one allowed. Two calls inside a
single assistant frame share a `seq` and so share a window, and they are handed answers
in hook order, which is an assumption and not a measurement. That last part reaches only
the disagreement check, never the decision.

**Two of the three scopes are rebuilt from the layout and the third from the
record.** `worktree` comes off `run_start`, and `corpus` and `skill_store` are
Hydra's own directories, which is what makes them recomputable at all. The store
is the one with a per-run answer: a curate run's writes are scoped to its own
checkout, and `Layout::skill_store_scope` is the rule both the launch and this
re-evaluation read, so a curator's record does not come back as a gate that leaked.

**`policy_sha` does not cover the gate script.** It hashes `policies/`, and the rules
that read a policy live in `src/policy/gate.py`. A record can therefore re-evaluate
against gate code that changed while its sha matched, and what reports that is a
disagreement rather than a stale policy. All nine records on this machine re-evaluate to
the verdict their gate gave, including the two shell calls ticket 06's measurement left,
and `tests/normalize.rs` asserts it per call rather than in aggregate.

**`parent_run_id` is lifted off the envelope onto the normalized run.** Contract 4
puts it there rather than in the body, because the parent writes it at spawn and
nothing reconciles it against the child. Rung one reads it twice, for the retry
class and to leave out the runs a curator pass spawned, so a reader that had only
the body would have neither. It is null on every record this build writes.

**Findings are not events.** An event says what happened; a finding says two sources
disagree or that the derivation could not do its job. Seven of them: a gate leak,
subagents spawned under a policy that denies them, a disagreement between the derivation
and the gate's recorded answer, a gated call the gate never answered, a tool name the
capability table does not know, a `policy_sha` that is not today's, and a bare limit for
anything else. The fourth is the fail-open case 6.6 measured: a hook that will not start
lets the call through on this provider, `--include-hook-events` is on for every recorded
run, so a call with no gate answer and no refusal in Hydra's own voice is one that reached
the model with nothing in front of it. Four things produce a `Limit` and two of them matter most. A file with
no `run_start` reads exactly like a pre-gate run, since both carry a null policy and no
decisions, so the derivation says which one it is looking at. And a record that stops
between a `tool_use` block and its result leaves a call whose fate nothing recorded, so a
policy that denies it reports a limit rather than a leak: "the file ends here" is not
evidence that the call went through. Ticket 14's abandoned-run class is exactly that
shape, and so is the tail of a killed run: the writer syncs per line, so a torn last line
costs its own frames and a limit rather than the whole read. The other two are a record
that names a policy and carries no `policy_sha`, whose decisions then answer for today's
`policies/` rather than the run's, and a `tool_use` block with no id, which nothing can
join a result or a hook answer to. The decision still follows the policy in both. 7.1's table check reads the
`system`/`init` frame, which lists every tool at no cost; `claude` 2.1.261 offered
exactly the twenty-five names `table.rs` holds, so the check is quiet today and a rename
breaks the first record written after it.

## Deriving an outcome, and what `hydra run` returns

Ticket 04. `src/outcome.rs` turns a normalized run into contract 8's nine outcomes and
8.2's `outcome_source`, and `hydra::run::result` projects that plus the diff stat into
contract 17's JSON. Nothing else in the crate derives `Serialize`, so the module boundary
is where contract 11's never-store rule stops being a comment.

**Nothing reads the exit status to pick an outcome**, and the rate-limited run below is
why. It exited 1. The one thing the derivation reads off `exit_code` is whether it exists
at all, which separates a child that died from a spawn that never happened.

**`gated` is "the gate refused everything", as a predicate.** That is contract 8's own
phrase, and harness spec 12 and tickets 27 and 14 all use the same words. It comes out
`gated` when the run made at least one tool call, no call got through, and at least one
refusal was Hydra's own gate. Three things it deliberately is not:

- **Not one refused call.** Contract 8.1 dropped `denied` partly because it "read as a
  per-tool-call fact", and a run that made fifty calls, had one denied and finished the
  task is a trajectory rung one should learn from. The denial stays in the record per
  call, where 7.7 puts it.
- **Not a gate leak.** A `Finding::GateLeak` is the policy denying a call nothing
  actually refused, so the call went through. An outcome says what happened, and filing a
  leak as `gated` would report the run as blocked by the hole that let it through.
- **Not somebody else's hooks.** A `decider` of `external` is the user's own `PreToolUse`
  guards, which 7.7 spends a section separating from Hydra's gate. A run blocked entirely
  by those comes out `completed`.

**The rule has one weak spot beyond that.** A run with one allowed `Read` and four
denied `Write`s comes out `completed`, because something got through. That run genuinely
failed on too narrow a policy, which ticket 27 calls a policy bug rather than a skill bug,
and nothing in the record says what it was trying to do. Fixing it means a threshold like
"mostly refused", which is a number no ticket settled. So the predicate stays literal, and
`tests/outcome.rs` pins both directions so a change to it breaks a test on purpose.

**A pre-gate record with a real denial in it comes out `completed`.** Contract 7.7 gives
a run with no policy no `permission_decision` at all, and `external` "presumes a policy
exists to have allowed", so there is nothing for the `gated` rule to read.
`01a07218-8817-7581-9ddb-47565e5d5ba7` is that record: one `Write`, refused, `policy:
null`, derived `completed`. It is the design's own consequence rather than a hole, and it
applies to the three pre-gate records on this machine and to nothing written since.

**`unknown` has one producer and it is the only soft outcome.** A `completed` reached by
nothing saying otherwise, on a run holding a tool result the adapter returned
`unclassified` for, is `outcome_source: unknown`. Every other outcome rests on something
a source said out loud, and an unreadable tool result does not weaken those: a 429 beside
an unclassified result is still `rate_limited`, `derived`.

**`auth_failed` from the stream is a hypothesis and `rate_limited` is not.** 429 is
measured, on the run below. 401 and 403 are what an expired token should produce and
neither has been seen on a record here, so they are in the match with that said out loud
rather than presented as a reading. The direction of the guess is deliberate: an
authorization refusal filed as `failed` teaches rung one a skill lesson from a lapsed
login. Anything else the provider reports falls through to `is_error`, so a 500 is a real
failure of the run.

**`crashed` reads the terminator's absence, not a signal.** `claude` writes exactly one
`result` frame on every path, so a `run_end` with no terminator behind it is a child that
stopped mid-run. With a null `exit_code` it is `failed` instead, because no process ran
and nothing died.

**`run_end.error` is read on one side of that split and not the other.** With a
terminator present the run reached its own end, so the field is a fidelity note: a lossy
line means the record has every line and one is degraded, and filing that run as `failed`
would teach rung one a skill lesson from a UTF-8 byte. With no terminator it is the only
thing in the file that says which side stopped. So a `run_end` with no terminator, a real
exit code and an `error` is Hydra's tee having broken on a child that may have gone on to
finish, and it comes out `failed` rather than `crashed`, because contract 8 defines
`crashed` as "the process died" and nothing witnessed one. `crashed` is left for the
shape where the child ended without its terminator and Hydra has nothing to say about
why.

**The final assistant text is the final turn, not the final block.** Contract 7.3 gives
one `message` event per content block, which is what `Message::block` numbers, so a frame
that interleaves thinking with prose arrives as `[text, thinking, text]`. Taking the last
event returned the second half of what the model said and dropped the first. The blocks of
the last frame are joined in block order, separated by a blank line.

**A run with no `run_end` gets no outcome, and `derive` returns `Option` to say so.**
Either the child is still going or the run is ticket 14's abandoned class, and telling
those apart needs 5.2's pid liveness test, which contract 8.6 gives to `hydra status`.
Flattening it to `crashed` would file every live run in the corpus as a dead one. That
reader is now built: `hydra status` answers `live` or `abandoned` where the derivation
answers nothing.

**A record Hydra cannot read back does not fail the command.** The file is whole and
synced before the read starts, so propagating the error would report a run that worked as
one that did not and would take the run id and the path to the answer with it. The
fallback returns contract 17's field list with the derivations null and `error` naming the
read failure, which is what stops `outcome: null` reading as a measurement. That null now
has two producers, a run with no `run_end` and a record that would not read, and `error`
says which. What triggers it is not tested and cannot cheaply be: it needs `read_file` to
fail on a file `hydra run` just wrote, and `hydra run` launches a provider. The fallback's
field shape is tested.

**A spawn that never started still writes a record and returns no JSON.** The detached
half writes `run_start` and `run_end` with the reason in `error` and exits; the foreground
reads that record back like any other, sees a null `exit_code` with no `refusal` beside
it, and returns the error rather than the projection, so `hydra run` exits non-zero with
nothing on stdout. A launch that fails before the fork, so that there is no detached half
to write anything, writes the same record from the foreground instead: `run::never_started`
is that path, and by then a worktree, a branch and a compiled gate are on disk with a run
id on them, so under D10 a file has to exist saying so. The record derives `failed` and `tests/outcome.rs` pins it, so nothing
is lost for the corpus; what is missing is the JSON for a caller that wanted the
projection rather than an exit code. Making it print instead would mean exiting 0 on a run
that never launched, which breaks any script reading the status, so the fix belongs with
the MCP delegate face in ticket 11, where a caller has no exit code to read.

**That test is "no exit code and no refusal", and it is a reading of the record rather
than a fact the launcher kept.** The launcher no longer runs the spawn, so it has nothing
in memory to check; contract 8.3's refused spawns carry a `refusal` code and a null exit
code, which is why the second half of the test is there. `tests/detached.rs` drives a
binary that is not on disk and asserts the record, the null pid and the closing line.

**The result is nine fields and `provider` is not among them.** Contract 17 gives the
list and then says "Nothing else", so `provider` and `provider_session_id`, which the
first slice returned, are gone. Both are in the record and `run_file` is on the list.
`branch` and `diff_stat` are absent rather than null on a run with no checkout, following
5.5's rule; the rest are always present.

**Reading the record back costs a python process per recorded tool call.** Measured on
this machine: a record with no calls reads back in 0.6ms, one with two calls in 94ms, so
the gate is about 47ms a call and it is all of the cost. A 200-call run therefore spends
about nine seconds after the child exits before a blocking `hydra run` prints anything.
That is the price of `permission_decision` being a re-evaluation rather than a copy of the
gate's answer, which contract 7.7 requires and the "Reading a record back" section above
argues for.

**Detaching moved the child off the caller's critical path and not that read.** A blocking
call still pays it, because a projection of the record is what it asked for. What changed
is that the cost is now optional: a call that gives up pays nothing, since a handle is
built from line 0 and derives nothing, and `hydra status <run-id>` pays it later on a run
it did not launch. `hydra status` with no argument pays none of it at all: it reads line 0
and the tail of each file and never normalizes. Measured on this machine, twelve records:
under 10ms for the listing, 20 to 30ms for one ended run with no tool calls in it.

## The detached spawn, the timeout, and what `hydra status` reconciles

Ticket 05. Harness spec 9 makes every child detached and blocking a view over it, and
10 makes process exit the authoritative end of a run with liveness derived and never
announced. Both are built. `src/watch.rs` is the half that owns the pty and the record,
`src/run.rs` is the half a human talks to, `src/live.rs` is contract 5.2's test, and
`src/status.rs` and `src/logs.rs` are the two commands that read what the test says.

**`hydra run` re-execs itself as `hydra watch <spec>` in a session of its own.** The
launcher composes everything provider-specific first, writes it to
`~/hydra/spawn/<run-id>/launch.json`, and forks a copy of itself with `setsid` and its
three standard descriptors on `~/hydra/spawn/<run-id>/watcher.log`. Closing a Ghostty
window sends `SIGHUP` to the foreground process group of its tty, and a process in its
own session has no controlling terminal to be hung up on. `tests/detached.rs` asserts
the session id rather than a symptom of it.

**An in-process `fork` was the alternative and it is the wrong trade.** It would have
put an allocator, a pty and an open record file on the child side of a call with no
`exec` after it, and nothing about it would be reachable from a test. Re-exec costs one
process start per run, which is noise beside a provider launch.

**The launch spec is not a test hook and it is not a second source of truth.** It holds
the binary, the argv, the cwd, the run file and the whole of line 0 with `pid` and
`process_start_time` null, which are the only two fields the detached half fills in and
the only two that cannot exist before a process does. It is read once, at launch, before
the record exists, so harness spec 5's rule holds: nothing needs a spawn artifact to
interpret a run. Taking a binary and an argv rather than a provider is what lets
`tests/detached.rs` drive the whole path with `/bin/sh` and spend no subscription.

**No environment values go in it.** `HTTPS_PROXY` can carry a password, and contract 5.3
keeps values out of the record for the same reason, so the detached half rebuilds the
child's environment from the allowlist out of what it inherited. Names reach the record;
values never touch the disk.

**The timeout returns a handle and the handle is not contract 17's result.** Harness spec
9 degrades a blocking call rather than thinning it, so the two shapes are different on
purpose: a run that has not ended has no derived fields, and printing contract 17's list
with nulls in it would offer a page of measurements nobody took. The handle is `run_id`,
`outcome`, `alive`, `pid`, `timeout`, `branch`, `run_file` and the `hydra logs --follow`
line that answers "what happened next".

**`timed_out` is the call's outcome and never the record's, and this is a reading the
ticket left open.** Contract 8's row is "the watcher gave up; the child may still be
running", which is a sentence no `run_end` can carry: by the time a record has a closing
line the child has stopped. Ticket 05 says both "the timeout's handle carries no outcome"
and "`timed_out` has no producer until you write one", and those point opposite ways. What
is built takes the second: `Outcome::TimedOut` is produced where the giving up happens,
on the handle, and the record keeps its own answer for whoever reads it later. Measured on
run `01a07471-0a3e-74d2-b576-153ac0e7427b`, `read-only`, `--timeout 4s`: the call returned
the handle with `outcome: timed_out` and `alive: true`, the child carried on with nobody
watching, and the record it closed by itself derives `completed`. The alternative reading
would have filed that run in the corpus as a timeout, which is a lesson about the caller's
patience dressed up as one about the task.

**There is no `outcome_source` on the handle.** Contract 8.2's three values all describe
how a record was read and this outcome was not read from one. A fourth value is a decision
for the map rather than a field for this build to invent.

**Nothing kills a child, ever.** Not the timeout, not the cap, not `hydra status`. The
only thing that removes anything is `hydra reap`, which a human types, and that is ticket
06. So a wedged run stays wedged until someone looks: harness spec 19.5 rules out a stall
detector and contract 18.4 says why, and a timeout with a decimal point on it would be
exactly that detector under another name.

**Liveness is one function and nothing else asks the kernel.** `procinfo::is_alive` was
already built and tested; `live::read_view` is what feeds it a record's pid and start
time. Three states come out: a file with a closing line has ended, one without whose pid
still matches is live, one without whose pid is gone is contract 6's abandoned class.
`tests/status.rs` builds all three by hand, because the reconciliation is what is under
test rather than the launch that produced the files.

**A status pass reads line 0 and the last 64 KiB and nothing between.** `run_end` is
always the last line and is always small, so a fixed tail either holds it whole or the
file has not ended, whatever the frames above it look like. A whole-file read would make
every status pass cost the size of the corpus.

**A run file the scan cannot read is skipped with a note.** The scan runs before every
launch, so propagating that error would mean one unreadable entry under `~/hydra/runs/`
stops `hydra run` and `hydra status` alike until a human finds it by hand. It is the same
stance the normalizer takes on a torn record, one level up.

**A dead pid is not by itself an ending, and two places had to learn that.** The detached
half reaps its child and then writes the closing line, so every ordinary run passes
through a window where the liveness test says gone and the record has not ended. It is an
`fsync` wide against a 100ms poll: small, and hit often enough to matter. `wait_for_end`
and `hydra logs --follow` both re-check the file before they conclude anything, and
`tests/status.rs` holds that window open with a pid killed before the record was written.

**A checkout younger than the launch budget is a launch in progress, not a reap, and the
record being there does not change that.** `hydra run` creates the worktree and seeds it
before the detached half writes line 0, and on a large repo that is seconds. Age is what
tells that apart from a run whose record was pruned, and `abandoned_worktrees` takes the
grace period as an argument so ticket 06's reaper decides it at the call site rather than
inheriting it.

The first version applied the grace only when there was no record at all, and ticket 06's
code review found the hole: `watch` creates the run file before it spawns the child, so
during a launch there is an empty record that reads as abandoned with a null pid, and a
view existed, so age was skipped. `RunView::launched` is the test now, meaning the record
carries a pid or has ended. Everything else is judged on age. A refused spawn passes
because it has a closing line, which is what makes contract 5.2's null pid a claim rather
than a silence.

A filesystem with no creation time keeps the directory on the list, which is the safe end
for a command that only reports.

**A record `hydra status <run-id>` cannot normalize costs the projection and not the
command.** `hydra run` already degrades that way. `state`, `pid` and `run_file` come off
line 0 and are what lead a human to the file, and a status command that refuses to report
a damaged record hides the run most worth looking at.

**A `--timeout` bigger than the clock can reach means "do not give up".** `Instant +
Duration` panics on overflow and the flag takes any `u64` of seconds, so the deadline is
computed with `checked_add` and `None` is a budget that never expires. Panicking there
would have aborted the launcher after the worktree, the gate and the child already
existed.

**A torn line 0 still reports a run.** The run id falls back to the filename, which is
where it came from, and the state falls back to abandoned. A status command that refuses
to report a damaged record is one that hides the run most worth looking at.

**`hydra status <run-id>` returns the same projection `hydra run` does, off the same
function, on a run it did not launch.** That is contract 17 being a projection of the
record rather than a summary of a launch, made checkable. It appears only on an ended run:
a live one has no ending to derive and gets `state` instead.

**The diff stat's base is recovered from the seed index rather than persisted.** The sha
`worktree::create` returned lives in one process's memory and a detached run returns to a
caller that never had it. `~/hydra/spawn/<run-id>/seed-index` is the index the seed
snapshot staged into and nothing writes to it again, so `git write-tree` against it
answers with the same sha it answered with at launch. A file holding the sha beside it
would be a second source of truth for a fact the index already carries. Measured on
`01a073ef-701c-7522-b5bb-56d5ecba0f57`, launched by ticket 04 and read back by a later
binary: base tree `7c222fb9d996872a39801ad14ef7db1d835280fb`, the same one that run
reported.

**Abandoned worktrees are every checkout no live run is using**, which after any finished
run is most of them. Nothing is cleaned up automatically (harness spec 10), so that list
is work for `hydra reap` rather than a list of faults. A directory whose run file has been
pruned is on it too: a checkout nobody can explain is exactly what a human wants told.

**`hydra logs` prints the run file verbatim and does nothing else.** Observation is
read-only: no attach, no input, no answering a permission prompt from outside. `--follow`
stops at the closing line, and also when the liveness test says the writer is gone, so
following an abandoned run ends with a line saying so rather than waiting forever.

**Harvest is the half of `hydra status` that is not here.** Section 10 also has it
complete a dead interactive run from the vendor's transcript. No command in this build
launches an interactive run, so there is no unharvested body on this machine, and a
harvester with no producer would be untested code guarding a case that cannot arise.

## The diff stat, and the seed it has to subtract

Contract 17's one returned field with no record equivalent, and the trap in it is real
rather than theoretical. Harness spec 6.3 seeds a run's worktree with the parent's
uncommitted work, so `git diff HEAD` inside the checkout counts the human's work in
progress as the child's.

**So the base is a tree, snapshotted after seeding.** `worktree::seed_tree` stages the
freshly seeded checkout into a scratch index and calls `git write-tree`; the sha rides on
the `Worktree` and the closing `git diff-tree -r --numstat` is against it. The returned
`diff_stat` carries `base_tree` and a `covers` sentence, because the number is useless to
anyone who does not know what it is measured from.

**The scratch index is why the snapshot is invisible.** `git add -A` against the
checkout's own `.git/index` would leave the child looking at a fully staged tree, and a
child asked to finish work in progress would see something the parent does not have.
`GIT_INDEX_FILE` points at `~/hydra/spawn/<run-id>/seed-index` instead, with a second
file for the closing pass so staging cannot destroy the base. `tests/worktree.rs` asserts
the child's own `git status` still shows the seed as uncommitted work.

Measured on this repo, 2026-09-05: a worktree seeded from nine dirty files, one file
written into it afterwards. `diff_stat` reported one file and one insertion; `git diff
HEAD` in the same checkout reported nine files.

**Committing the seed was the alternative and it is the wrong trade.** One commit on the
run's branch would make `git diff HEAD` correct for free, including for the child itself.
It also hides the work in progress from a child asked to continue it, which is what 6.3
seeds the worktree for, and it changes what the branch means. The snapshot changes
nothing the run can see.

**A stat that cannot be computed says so on stderr.** Neither failure refuses a run: by
the time the closing diff runs the record is already whole, and a seed snapshot that will
not write costs the field and nothing else. Both print a line, because an absent
`diff_stat` otherwise reads as "this run had no checkout".

**A record with no `run_id` gets no diff stat.** A torn line 0 normalizes to an empty
`run_id` with a `Finding::Limit` rather than failing the read, and `Layout::spawn("")` is
`~/hydra/spawn/` itself, so the scratch index would be one file shared by every such
record and two at once would stage into each other. The guard skips the stat, the same
way a missing seed tree does.

**A `permissive` run can make the stat wrong.** Writes are scoped to the worktree under
four of the five policies, so nothing can reach the index files; `permissive` grants
unscoped writes and the gate protects only the gate script inside the spawn directory.
Such a run can rewrite its own `seed-index`, which costs the diff stat and nothing else.
The record is what carries the truth about what a run wrote.

## What a rate-limited run really looks like

The first `hydra run` after ticket 04 landed hit the session limit, which measured
contract 8's nastiest shape on a live run rather than on a synthetic frame. Run
`01a073ef-701c-7522-b5bb-56d5ecba0f57`, `claude` 2.1.261, `read-only`, 2026-09-05:

```
subtype: "success"      is_error: true       api_error_status: 429
terminal_reason: "api_error"                 stop_reason: "stop_sequence"
exit code 1             usage: every count present and 0
```

Derived `rate_limited` with `outcome_source: derived`, and `result_text` carried the
provider's own words: "You've hit your session limit · resets 7:50pm (America/Bogota)".

**Two things the contract did not already say.** `terminal_reason` reads `api_error`,
which is a second honest signal beside `api_error_status`; nothing keys on it, and it is
written down because a provider that drops the status field would leave it as the only
one. And the exit code was 1, so a reader keying off the exit status would have filed
this as an ordinary failure and rung one would have learned a skill lesson from an
exhausted quota.

**`usage` is `provider` with zeroes here, and that is correct.** The frame reported every
count and every count was zero, because the request was rejected before a token moved.
Contract 7.8's rule is about a provider that reported *nothing*, which is the
`unavailable` case; a provider reporting zero is a measurement.

**The `rate_limit_event` frame does not have the shape contract 8.5 describes.** 8.5 puts
`rateLimitType`, a `status`, a `resetsAt` epoch and a per-window `utilization` on the
frame. Measured, they are nested one level down under `rate_limit_info`, `utilization`
lives under `unifiedWindows.<window>`, and three fields 8.5 does not name are beside
them: `overageStatus`, `overageDisabledReason` and `isUsingOverage`. This frame has no
reader until rung two's early abort, so nothing is broken today. Whoever builds that
reads the shape here rather than 8.5's sentence, and the `resetsAt` staleness test 8.5
asks for still works.

## What the command line does not carry

**`--mcp-config` and `--strict-mcp-config` are not passed.** 6.7 puts both on a
non-`judge` recorded run. MCP injection is ticket 08 and writes the file they point at,
so passing them now would name a file that does not exist. The record says the same
thing by leaving `mcp` absent.

**`< /dev/null` is dropped**, and harness spec 6.7 now says why.

`--permission-mode bypassPermissions` is passed, and the section below says what was
measured before it went on.

## What ticket 02 measured: the gate on a real run

Three probes on `claude` 2.1.261, 2026-09-05, each a run asked to write `probe.txt`
inside its own worktree with the file tool and to stop if blocked. The records are on
this machine and the run ids are below.

**A hook deny beats `bypassPermissions`, on this wiring and not only in ticket 26's
rig.** Run `01a07308-39e3-79c2-8901-ac052723b79a`, policy `read-only`, `permissionMode:
bypassPermissions` on its own init frame. The gate answered `deny` on
`PreToolUse:Write`, the reason `hydra/capability: policy read-only does not allow
file_write (Write).` reached the model verbatim as the tool result with `is_error:
true`, no file was written, and `result` closed `subtype: success`, `is_error: false`,
exit 0. **The refusal holds and the terminator lies about it**, which is what contract
section 8 says to expect of all three providers. `result.permission_denials[]` carries
the denial with its `tool_use_id`, contract 8.5's corroboration.

**The flag went on after that, not before.** Run
`01a07307-2693-7ef0-b572-a565ffcc3190` is the same probe with `bypassPermissions` held
back, and it is what proved the hook was mounted and the `*` matcher matched before the
provider's own permission layer was taken away. Same denial, same reason, no file. The
sequencing is the point: a gate that had failed to mount would have been invisible under
the provider's default mode and wide open the moment the flag went on.

**The gate discriminates rather than blanket-denying.** Run
`01a07308-a250-7792-9e8b-0d99697c99d9`, policy `standard`, same probe, same flag: the
gate answered `allow` with `hydra/allow: policy standard allows file_write.` and
`probe.txt` is on disk in that worktree. Without this run the other two prove only that
something says no.

**The user's own hooks reach the child, and the record shows it.** Every one of the
three carries a `UserPromptSubmit` `hook_response` from the user's `~/.claude`
guards, alongside Hydra's `PreToolUse`. That is 6.6's composition working rather than
isolation, and it is also 6.6's warning arriving: the record now carries the stdout of
a script the user edits.

**`standard` did not hit the `ToolSearch` and `Skill` denial.** Ticket 02 expected the
first gated `standard` run to notice the capability loss below. This probe was one tool
call long and never reached for either. The loss is still there and still real; it
wants a run with ordinary coding shape to surface it.

## What the first slice measured

Three findings, all on `claude` 2.1.261 over a pty Hydra owned, 2026-09-05. **All
three are in the documents now**, so what follows is the short version and the
document is the source: harness spec 6.7 for the first two, contract 3 for where a
non-JSON line goes, contract 7.1 and 7.7 for the third.

**`< /dev/null` was a finding about a pipe.** Harness spec 6.7 adds the redirect so
`claude` does not wait three seconds for stdin and write `no stdin data received in
3s, proceeding without it` into the record. On a pty there is no wait and no warning.
Measured off the record's own timestamps: 0.83 s from spawn to the first frame, which
includes the provider's startup and the user's own `UserPromptSubmit` hook, and 2.37 s
for the whole run. There is no three-second stall anywhere in it. Hydra
owns a pty because it has to be the parent to tee anyway, so the redirect is not
reachable from here and is not needed. The spec sentence is true for the shape it was
measured on and does not carry to the shape Hydra launches.

**`claude` writes a bare `ESC[?25h` after its last frame, with no trailing newline.**
Show-cursor, on the way out, because stdout is a tty. Any tee that assumes every line
is a frame will either drop it or fail to parse it. This slice stores it as a JSON
string in `raw`, because never-discard has no exception for noise, and a
string-valued `raw` is how a reader tells a line that was not JSON from a frame that
was.

**`system/permission_denied` is a frame the contract's producer table does not
name.** Contract 7.2 lists no `claude` frame for a provider-side refusal, because
every recorded run was specified to launch with the permissive flag plus the gate, so
the decision is made before the process starts. Take the permissive flag off and the
provider makes decisions again, and it announces them: one `system/permission_denied`
carrying `tool_name`, `tool_use_id` and a message, plus an entry in
`result.permission_denials[]`, which contract 8.5 already names as corroboration. On
these records nothing derives from either, because 7.7 needs a policy. It gets no row
in the producer table for that reason and a paragraph in 7.7 instead, so the next
reader meets it in the document rather than in a record.

## Ouroboros rung one: `hydra curate`

Ticket 12. `src/ouroboros/` is the whole of it. `corpus.rs` decides which runs the
curator may learn from, `curate.rs` composes what it is told, launches it as an
ordinary recorded run under `curate`, and commits whatever that run wrote into its
checkout of the skill store onto `ouroboros/curate/<run-id>`. `tests/curate.rs`
drives both halves and launches no provider.

**The corpus is read through the normalizer and never off `raw`.** Contract 11 is
the reason: an adapter fixed next month reads last month's records correctly
because nothing was migrated, and a curator that greps the stored frame gets none
of that. `tests/curate.rs` also checks that a walk changes no byte under `runs/`.

**Three filters, each a field rather than a guess.** A harness run goes on its
kind, `provider: null`, which is contract 8.6 and is what keeps an interrupted
`hydra evolve` from reading as an abandoned run: it is a dead pid with no
`run_end`, which is exactly the shape rung one's first class looks for. Contract
8's right-hand column goes on `Outcome::rung_one_learns`. A run that is still
writing is skipped before its file is opened, because the liveness test of 5.2 is
cheaper than a normalize and the record would look different in a minute.

**A fourth filter is not in the design, and the corpus is what made it
necessary.** A recorded run with a null policy predates the gate, so contract 7.7
derives no `permission_decision` for it and a refusal inside it is invisible:
`01a07218-8817-7581-9ddb-47565e5d5ba7` is one `Write`, refused, derived
`completed`. **The test is a null policy beside `launch_mode: recorded`, and
dropping the second half would take the interactive population with it**, since an
interactive run carries a null policy legitimately (5.6) and is the only source
rung one's human-rejection class has.

**Skipped is not thrown away and the brief says so.** It carries a count per
reason, because a curator told only what it may learn from cannot tell an absent
signal from an absent producer. Two of the five classes have no producer in this
build at all, human rejection and the retry loop, and the brief names them as
that rather than printing a zero somebody could read as a measurement.

**The curator writes into a git worktree of the store and never into
`~/hydra/skills/`** (ouroboros spec 3.6), so the run's `skill_store` scope points
at the checkout rather than at the store. `hydra::run::Request` gained one field
for it, `run_id`: the checkout is named after the run, so the id has to exist
before the gate is compiled, which is the one thing `hydra run` mints for itself
and this caller cannot.

**Where that scope points is a rule on the layout rather than an argument, and a
real record is why.** `Layout::skill_store_scope` takes a run id and a policy name
and answers with the checkout for `curate` and the store for everything else. Both
sides read it: the launch compiles the gate against it, and contract 7.7's
read-time re-evaluation rebuilds it. The first version passed the path in at launch
and left the reader to guess, and the pass measured below came back with ten
`Finding::GateLeak`, one per file the curator wrote or probed inside its own
checkout, because the reader was still resolving `skill_store` to `~/hydra/skills/`.
Nothing on the record carries a scope and inventing a field is a contract change,
so the fix is a rule both sides derive from the run id. The corpus-wide gate-leak
check in `tests/normalize.rs` is what caught it, and `tests/curate.rs` now pins the
round trip.

**The checkout lives at `~/hydra/skill-worktrees/<run-id>/`, which is a sixth
place and a choice.** Section 9.1 leaves the location open and says the spawn
directory would do. It will not: `hydra reap` removes that directory whole, and a
pass whose run outlived its call leaves an uncommitted proposal in the checkout.
Work is not something a cleanup command should be able to take. Nothing but
`hydra curate` creates or removes one.

**The commit is the pass's only post-run action, which is why the pass gets no run
record of its own** (contract 5.5). Its order is git's guarantee rather than a
sentence here: `git worktree remove` without `--force` refuses a checkout with
changes in it, so a commit that did not happen stops the removal instead of
preceding it, exactly as in `hydra reap`. A pass that proposed nothing has its
branch deleted by `git branch -d`, which refuses any branch holding a commit, so
the mechanism that tidies an empty pass is the one that cannot touch a real
proposal.

**The commit message keeps Hydra's facts and the model's words apart.** The
curator's rule against taking an intent declaration from a summary afterwards
exists because a model's free text stopped matching what it did. A pass commits
after the run, so a summary afterwards is all there is; what it can do is label
the halves, and it does.

**`hydra curate` prints text rather than contract 17's JSON.** That projection
belongs to `hydra run` and the delegate face, where a caller switches on it. This
command is typed by a human like reap and prune are, and its answer is a branch.

**A curate run still gets a checkout of whatever repo you typed the command in**,
because harness spec 6.3 gives every recorded run one and this is an ordinary
recorded run. Under `curate` nothing may write to it, so it is a checkout the run
cannot use, and `hydra reap` is what removes it.

**Measured on this machine, 2026-09-06.** Thirteen records: 6 learnable, 7 skipped
as 3 pre-gate, 3 `gated` and 1 `rate_limited`, and the walk costs 0.55 s end to
end including process start. One full pass ran against a real `claude`, run
`01a07513-baa3-7cf0-b20a-ded11ede6a38`: `completed`/`unknown`, 489,440 tokens, and
it proposed nothing. It read the six records, declined to propose a skill edit for
the one denied `Bash` call because the brief says a denial is a policy bug, ranked
the one `unclassified` result without diagnosing from it, and said the store held
no skill to edit. The empty branch was deleted, which is the shape above working
rather than a pass that failed.

**One ordering the skill store is about to break.** The checkout is of the store's
`HEAD`, and 3.6 has a launch commit a dirty store before pinning it. `hydra curate`
checks out before it launches, so once ticket 07 wires that commit the curator will
be reading the store as it was before its own launch tidied it. Whoever wires the
pinning owns the fix, and it is an ordering decision rather than a line of code:
either the store is committed before the curate checkout, or the curate checkout
moves after the launch and the gate is compiled against a directory that does not
exist yet. The pass measured above hit the harmless half of this, an empty `HEAD`
under a store whose skills were not committed.

## Choices this build made that no ticket settled

**`--policy` defaults to `standard`.** Section 9 makes the flag optional and names no
default. `standard` is the ordinary coding shape and the only one of the five that fits
a run a human typed without thinking about policy. The alternative worth naming is
having no default and refusing without the flag, which is safer on paper and would mean
every `hydra run` carries a flag whose value is `standard` in almost every case.

**A run with no repo scopes `worktree` at a path nothing creates.** `hydra run` outside
a git repo has no checkout, and the child runs where Hydra was invoked. Pointing the
`worktree` scope at that directory would hand the run write access to whatever the human
happened to be standing in, so it points at `~/hydra/spawn/<run-id>/no-worktree`, which
nothing creates. Under `standard` that scopes reads to the same missing directory, so
such a run reads nothing through the file tools either, and shell is what it has left.
That is the honest reading of D9: Hydra writes only under a path it created, and a run
with no checkout has none.

**Reap deletes a branch only when git says nothing would be lost, and a flag goes
further.** Harness spec 10 and 17 both stop at the worktree and the spawn directory, and
neither says what becomes of the branch. Committing to it and then deleting it would make
the commit theatre, which is the failure the section is about, so the default is
`git branch -d` and `--delete-branch` is the human's way past it. Ticket 06's "done when"
asked for the thirteen branches on this machine to be gone by the command; they were kept
instead, and the ticket's outcome says why.

**The spawn directory's filenames are `gate` and `settings.json`.** Harness spec 19.1
leaves the names inside the spawn directory open. `settings.json` comes from 6.6's own
table; `gate` comes from 7.5's prose. Neither is a ticket's decision.

**The preamble and the caller's task are joined by a blank line and nothing else.**
Section 8 allows the preamble three assertions and no more, and a marker line saying the
task follows is a fourth sentence in Hydra's voice. The cost is that a reader of
`run_start.task` has to know where the preamble ends, and the preamble is a fixed string
in `config/preamble.md`, so they can.

**The concurrency cap is eight.** Harness spec 9 asks for "a cap on live children" and
names no number. Eight is more than the parallel tool calls an orchestrator issues in one
turn and few enough that eight `claude` processes on one subscription is a bill you notice
before the machine does. It refuses rather than queues, which is what section 9 asks for:
a queue holds a slot the caller can neither see nor cancel. The refusal is a type,
`live::TooManyLive`, carrying the run ids, because a caller that hits it has to decide
between waiting on one and reaping it and neither is decidable from a sentence.

**A run refused by the cap writes no record.** Contract 8.3 makes a refused spawn a record
when the refusal is one of section 6's four codes, and the cap is not one of them. Minting
a fifth is a contract change rather than an implementation, so the refusal goes to stderr
and no run id is minted.

**The cap is checked and not held, so two launches racing can both pass it.** The count
comes from a directory scan, and nothing locks between that scan and the fork. Two
`hydra run` calls started in the same instant with eight already live would leave ten. A
lock would fix it and would be a second source of truth about how many runs exist, which
is what D10 spends the run file avoiding; the cap is a brake on an orchestrator issuing
parallel tool calls, not a quota anyone is defending.

**`--timeout` takes `600`, `30s`, `10m` or `2h`, and defaults to `10m`.** Section 9 says
"roughly ten minutes" and names no spelling. Bare digits are seconds, which is what every
other tool on this machine means by a naked timeout. Days are not a unit: a call nobody is
waiting on for a day should have returned a handle and been polled. `0` is legal and means
launch and hand me the handle, which follows from nothing here ever killing a child.

**The view polls the run file every 100ms.** So a blocking call can add up to that after
the child exits, which is noise beside the record read that follows it. Watching the file
with a kernel notification would remove it and would put a platform dependency on the one
path a test can already drive by hand.

**The cap's refusal names `hydra logs --follow` and `hydra status`, not `hydra reap`.**
The count is of live runs, so a run that has ended has already given its slot back and
reaping it frees nothing, and `reap` is not a command this binary answers to yet either.
A refusal that tells you to run something that fails twice over is worse than one that
just says no.

**The spawn directory holds two more files: `launch.json` and `watcher.log`.** Harness
spec 19.1 leaves the names inside it open. Both die with the worktree under `hydra reap`,
and neither is read to interpret a run.

**Worktrees live at `~/hydra/worktrees/<run-id>/`, on branch `hydra/<run-id>`.** No
closed ticket names the location, which is the same gap harness spec 19.1 flags for
the filenames inside the spawn directory. A Hydra-owned path keeps D9 true when the
repo being worked on lives somewhere else. Confirm it rather than inherit it.

**`run_start` is written after the spawn, not before it, and by the detached half.**
Contract section 5 says the line is written before the child process starts and 5.2 puts
the child's pid on it. Both cannot hold literally. What run_end's derivation leans on is
that the pid is in the file, and a pty buffers everything the child writes until the tee
reads it, so spawning first loses nothing and `run_start` is still line 0. The launcher
composes every other field on it; `pid` and `process_start_time` are the two the half
that owns the process fills in.

**`process_start_time` is RFC 3339 with microseconds**, read from `kinfo_proc` at
offset 0 through `sysctl`, because `libc` carries no `kinfo_proc` for Apple targets.
A test compares it against `ps -o lstart=` on every run of the suite, so the day that
offset stops being `p_starttime` the suite says so instead of the liveness check
quietly matching everything.

**A `HOME` that disagrees with the passwd entry is dropped rather than forwarded.**
Harness spec 6.2 says an absent `HOME` is safer than a wrong one, because the keychain
search list falls back to `getpwuid`. This follows that reasoning to its conclusion,
and the drop is visible after the fact because `env_allowlist` carries the names
actually passed.

**A `PATH` without `/usr/bin` refuses the launch** before a run id exists, with an
error naming the silent logout it would otherwise cause. It writes no run record: the
contract's four `refusal` values are all Hydra-side policy checks this slice does not
run, and inventing a fifth is a contract change.

**The result is read back off the file rather than assembled in memory.** Contract 17
makes it a projection of the record, and `hydra run` still has every value in scope when
the child exits, so building it from those would be faster and would also let the JSON
carry a fact the record does not have. Section 6 says that is the mistake ticket 11 made.
Reading the file is what keeps the projection true rather than intended, and it costs
what the section below measures.

**Two refusal codes are minted here, not in a ticket.** Contract 6 names four Hydra-side
checks in prose and no closed ticket gives them strings. `record::Refusal` is the
vocabulary, one code each, in the shape ticket 01 gave the gate's `hydra/` reason prefix
and for the same reason: the derivation matches on it, and a sentence is not something to
match on. Nothing writes one yet, so the four tickets that will are free to disagree
with the spelling before any record carries it.

## Not built

Out of scope for this slice, and each one has a document that already says what it is:

- Skill injection and the store. 6.4. Nothing creates `~/hydra/skills/`, so a run
  under any policy but `curate` is scoped to a store that is not there. The gate
  denies rather than resolving elsewhere, which is the safe end of that, and ticket
  07 is what makes the scope mean something. A curate pass is the exception and
  points the scope at its own checkout of the store, which is why it refuses before
  it launches when there is no store to check out.
- MCP injection and the Cursor pre-flight. 6.5. The spawn directory exists now and holds
  the gate and the hook config; the `mcp.json` beside them and Cursor's `cursor-data/`
  are ticket 08 and ticket 09.
- Five of harness spec 6.1's seven pre-flight checks. Auth, depth, policy containment,
  skill collision and both MCP checks belong to tickets that are not built. What runs
  before a run id exists today is the environment, the provider version, the policy name,
  the preamble file and the concurrency cap.
- Delegation: `--parent`, depth, monotonic narrowing, the MCP delegate face. 7.3, 9.
- Two of contract 8's nine outcomes have no producer in a record. `skill_collision` and
  `mcp_unavailable` are launch checks belonging to tickets 07 and 08, and both arrive
  through `run_end.refusal`, which nothing writes. Both are in the enum, because a value
  the reader cannot name is one nobody knows is missing. `timed_out` has a producer now
  and it is the handle rather than the record; the section above argues that reading.
- The `cursor-agent` and `agy` adapters, tickets 09 and 10, and harvest, 12. Contract
  7.2's other two columns and 7.2.1's transcript vocabulary have no reader.
- Interactive runs and harvest. 5.6, 12.
- Five of harness spec 17's eleven commands: `show`, `resume`, `evolve`, `export`
  and `usage recount`.
- Ouroboros rung two. `hydra evolve`, the protected-path refusal, the admission
  gate, the panel of three and the `ApprovedPatch` type are all absent, and so is
  the harness run a pass would write (contract 5.5). Rung one is built and the
  section above says what it does. The review checklist of ouroboros spec 5 is a
  document rather than code: nothing reads it, because nothing reviews yet.

## Rough edges worth knowing

- **A run with no `run_end` is not corrupt.** Contract section 6 says so. Killing the
  detached half mid-run produces one, and nothing completes it: `hydra status` reports it
  as abandoned and stops there, because completing it means harvest and no run on this
  machine has a vendor transcript to harvest from.
- **Killing `hydra run` no longer kills the run.** Ctrl-C on the foreground stops the
  view and leaves the child, its writer and its record alone, which is what harness spec
  9 asks for and is also a way to leak a run you meant to stop. `hydra status` finds it;
  nothing in this build can end it. That is deliberate rather than missing: nothing here
  signals a child, and the design gives removal to `hydra reap`, which a human types.
- **A frame that is not valid UTF-8 is stored lossily** and the count lands in
  `run_end.error`. It has not happened; `claude` writes JSON through Node.
- **The tee has no line-length cap.** A provider that streams one enormous frame
  would grow the buffer without limit. Frames of 200 KB are tested and fine.
- **`~/hydra/spawn/` is not denied as a whole, only this run's own script.** 7.5 names
  "the generated gate script", singular, and `denied_write_paths` follows it literally.
  A `permissive` run could therefore rewrite a concurrent run's live script, which 7.5
  says is read fresh on every invocation. This is live now rather than latent: ticket 05
  detached the spawn, so eight runs can be in flight at once and a `permissive` one among
  them can reach the other seven's scripts. It is smaller than the shell hole above only
  because shell already reaches
  every one of these paths. Ticket 04 put two more files in there, the diff stat's
  scratch indexes, and the section above says what a `permissive` run rewriting one
  costs: the stat, and nothing else.
- **A gate that fails to start still fails open on `claude`.** 6.6 measured it. The
  shebang is absolute and the script imports nothing outside the standard library,
  which is what that risk is spent on, and the sha check now catches a script that
  changed rather than one that will not run. An interpreter that is missing or broken
  is the case still open, and nothing checks at launch that the script runs. Adding
  such a check would be an eighth entry in 6.1's pre-flight list, which is a design
  decision rather than an implementation.
