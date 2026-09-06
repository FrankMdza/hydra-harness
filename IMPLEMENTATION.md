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

Provider versions this build has been measured against:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.263 | 2026-09-06. The specs were written against 2.1.257. Every flag in 6.7 still runs, and one claim about a flag has expired: `--restricted` no longer drops MCP, which the section on `judge` below has |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-06, unchanged since 2026-09-02. Routed by ticket 09 on this build; the section on it below says what was measured and what was read off the bundle |
| `agy` | 1.1.26 | 2026-09-06, unchanged since 2026-09-04 |

## The commands

```
hydra run --provider <claude|cursor-agent> --task "..." [--model <id>] [--policy <name>]
          [--timeout <dur>] [--allow-floor-escape]
hydra status [<run-id>]
hydra logs <run-id> [--follow]
hydra reap [<run-id>] [--delete-branch]
hydra prune [<run-id>...] [--delete] [--dry-run]
hydra curate [--model <id>] [--timeout <dur>] [--dry-run]
```

A launch builds the child's environment from the allowlist, reads the provider's
version, loads the policy set, the preamble and the canonical MCP document, refuses if
the concurrency cap is already met, mints a UUIDv7 run id, creates a git worktree and
seeds it with the parent's uncommitted work, writes the servers the run's policy admits
into `~/hydra/spawn/<run-id>/mcp.json`, links a snapshot of the skill store into the
checkout, asks the provider in one free probe what it resolved of both, compiles the
run's gate into the same spawn directory, composes `claude`'s argv and the whole of line
0, and hands all of it to a detached half that spawns the provider on a pty, writes
`run_start`, tees every frame verbatim into `~/hydra/runs/<run-id>.jsonl` and writes
`run_end` when the child exits. The foreground tails that file, and on a run that ends
inside its budget it reads the file back through the normalizer and prints contract 17's
JSON on stdout.

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

`cursor-agent` is routed as of ticket 09, and the section on it below is the one to read
before trusting a record it wrote. `agy` is routable in the design and refused here by
name, with an error that says which ticket it is waiting on rather than that the
provider is unknown.

## Cleanup: `hydra reap` and `hydra prune`

Ticket 06. Harness spec 10 gives both to a human to type and Ouroboros to neither, and
harness spec 7.5 already ruled out enforcing that with the gate: the binary is reached
through `shell`, so any rule against it is an argv match with the pattern floor's failure
mode. What holds it here is that nothing in `src/` calls `reap::reap` or `prune::prune`
except `main`.

**Reap also sweeps `~/hydra/skills-snapshots/`, added by ticket 07.** It is harness spec
5's second row and it has a lifetime of its own: keyed by sha, shared between the runs
that pinned it, and reaped when no live run references it. The sweep runs whether or not
a run id was named, because reaping one run does not release a snapshot another still
holds and reaping none can release several. The section on the skill store below has the
refcount and the two guards on it.

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

**Twenty-one of `claude`'s twenty-nine tools answer to no capability the five policies
speak in.** 7.1 denies a tool name the table does not recognise, so a table holding only
the eight names that answer to the six capabilities of the coverage matrix would deny the
other twenty-one under `permissive`, which is supposed to allow everything. They map to
one capability, `provider_native`, granted by `permissive` alone. One row rather than
twenty-one invented categories, and 7.1's drift check still fires on a thirtieth name.
Splitting it is a design decision and this build did not make it.

**`standard` therefore denies `ToolSearch` and `Skill`.** That follows from the
paragraph above and it is a real capability loss on ordinary coding runs, in the
direction 7.1 chose on purpose: an allowlist fails toward a run that cannot do its job,
which someone notices. Ticket 02's first gated run is where it gets noticed, and the
fix, if the records ask for one, is a policy file edit rather than code.

**Two protected configs are still paths rather than files.** 7.5 denies writes to seven
paths under every policy. Five exist now: `policies/`, `~/hydra/runs/`, the per-spawn
gate script, `config/preamble.md` from ticket 02 and `config/mcp.md` from ticket 08. The
other two are named at `config/env-allowlist.json` and `config/export-allowlist.json`,
and **neither of those files exists**. The environment allowlist is still Rust code in
`src/env.rs`, which 15.2 protects at admission rather than at run time, so the config
the table means has no file. Denying a write to a path that holds nothing is the same
denial it will be the day the file lands, which is why the names went in early rather
than being added later and forgotten. `tests/gate.rs` asks the compiled gate about all
six paths under `permissive` and `curate`, so the two that are still empty are covered
by the same test as the four that are not.

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

## The skill store, the snapshot and what a run is linked

Harness spec 6.4, built by ticket 07 for `claude`. `~/hydra/skills/` is a git repo of
its own inside the `~/hydra` working tree, ignored by it, created on the first launch
with an empty root commit so there is always a sha to pin. A dirty store is committed
before pinning, because launching from `HEAD` and ignoring the working tree would mean
editing a skill, running, and watching nothing happen.

**The commit trailer names the run that pinned the store, not the one that dirtied it.**
6.4 asks for authorship in the trailer and Hydra cannot supply the author: a `curate`
child writes into the store with its own tool calls and is long gone by the time the
next launch finds the change. `Hydra-Pinned-By: <run-id>` is what Hydra actually knows.
The committer is `hydra <hydra@localhost>` rather than whatever git identity the machine
carries, so the commit is attributable to the harness and a machine with no global
identity can still launch.

**The links point into `~/hydra/skills-snapshots/<sha>/` and never into the store.** The
snapshot is a detached git worktree of the store at the pinned sha, shared between every
run on that sha. `tests/skills.rs` proves the thing this is for: a skill edited after the
pin does not reach the run that pinned it.

**`hydra reap` sweeps snapshots, and which command does that is a choice.** Harness spec
5 says a snapshot is reaped "when no live run references it" and names no command. Reap
already removes a run's scratch and is already typed by a human, so it got this too. The
refcount is the corpus rather than a counter file: `skills::referenced_shas` reads the
`skills.store_sha` off every live run's line 0, which is the same scan `hydra status`
uses. Two guards sit on top, and the second is the interesting one. A snapshot younger
than the launch grace is left alone, the same rule spawn directories get. And a checkout
younger than the grace whose record has not spoken stands the whole sweep down, because a
launch reuses an existing snapshot without touching it, so age cannot see that one and
removing it would leave a running child with dangling links.

**The injected root ignores itself and the test says so against real git.** A
`.gitignore` holding `*` goes into `<worktree>/.claude/skills/` before the links, and
`tests/skills.rs` asserts that `git status` reports nothing and `git add -A` stages
nothing. An existing `.gitignore` is left alone: a repo that tracks its own would see the
overwrite as a modification to the human's checkout.

**A name the repo already carries is not linked over.** The store's copy is held back
rather than overwriting a project skill the human's repo tracks, and the collision check
reports it. `hydra.toml`'s `providers` is how a skill opts out of a provider; an absent
sidecar admits all of them.

### Asking `claude` what it resolved

`skills.resolved` comes from a free probe: `claude -p x --model
hydra-skill-preflight-not-a-model --session-id <uuid>` in the checkout. Claude Code
rejects an unknown model before it calls the API and still writes the session transcript,
which carries one `attachment` line whose `attachment.type` is `skill_listing`, holding
`skillCount` and a `names` array. Measured 2026-09-05 on 2.1.263: 1.8s, no API call, no
tokens.

Three details are load-bearing and each was measured. `--session-id` names the transcript
file, so the read is a path Hydra can predict rather than the newest file in a directory
another session may be writing to. Stdin is `/dev/null`, or the binary waits three
seconds for input that is not coming. And the transcript directory is
`~/.claude/projects/<slug>/`, where the slug is the checkout's path with every character
that is not a letter, a digit or a hyphen replaced by a hyphen; that rule was measured by
running the probe in a directory called `slug.probe_1`.

**Four things about the probe are worth knowing before trusting it.**

Its freeness is a property of 2.1.263 rather than a promise. A build that started
resolving unknown model names would turn this into a paid call, which is why the model
name says what it is.

It answers names and not descriptions. On the probe's path every filesystem-discovered
skill renders as a bare name, so the listing settles identity and precedence, which is
what the collision check asks, and cannot say whether a description survived.

It runs the human's own `UserPromptSubmit` hooks, because it is an ordinary `claude`
invocation with the user's settings loaded.

It leaves a transcript in `~/.claude/projects/` that nothing reaps, which is worth
holding against D9 and then letting go. Hydra never writes there; `claude` does, as it
already does for every recorded run in the same directory. The probe adds a second file
per run to a directory ticket 02's runs were already filling.

A probe that cannot answer does not refuse the run. `skills.resolved` goes in as null,
which contract 5.3 already requires a reader to treat as unanswered rather than as an
empty listing, and the third of the three collision tests does not run. The other two do,
and the launcher says so on stderr.

### The write check exists and has no caller

6.4 checks collisions twice: "at write, the curator refuses to create or rename a store
skill to a name the index already holds, so no collision Hydra causes reaches a run".
`skills::admit` is that check, and nothing calls it. No code path in this build writes to
the store: the curator writes through its own gated tool calls, and the step that turns
those writes into a store commit belongs to rung one. It is written here, with a test,
for the reason `record::Refusal` was written before any of its four checks existed: one
home for the rule, so the ticket that acquires the caller does not invent a second one.

The launch check is not a substitute. It fails the whole run, which 6.4 calls harsh and
justifies by what the write check does to the odds. Until something calls `admit`, every
badly chosen name costs a refused launch instead of a refused write.

### The three collision tests, and why the cheap two are not redundant

A store skill whose name the checkout already holds is a collision, decided before anyone
is asked. A linked name the user-scope index holds is a collision. A linked name the
provider did not list is a collision.

The first test reads both keys, the directory name and the frontmatter name, on both
scopes. A store skill Hydra could not link at all is a hit whatever holds the name, even
a directory that is not a skill, because a store skill silently absent from a run is what
this check exists to prevent. A skill that did link can still clash on a project skill's
frontmatter name, which the filesystem let through because `claude` keys on the directory
and `agy` does not.

The middle one looks redundant against the third and is not. Ticket 24 put one name in
`~/.claude/skills` and in the injected root: the provider listed it once, carrying the
user-scope copy, and the store's entry was silently absent. The name is in the listing,
so the diff sees nothing. Only the index comparison catches that one.

The index scans all seven user-scope roots of 6.4, including `~/.codex/skills` and
`~/.grok/skills`, which do not exist on this machine and appear the day the matching CLI
is installed. `Provider::user_skill_roots` is the per-provider filter over it, and today
`claude` reads one of the seven.

**Measured end to end on 2026-09-06.** A store holding `unslop`, a name
`~/.claude/skills` also holds, refused the launch in 2.3s with `outcome:
skill_collision`, `outcome_source: refusal`, a null exit code and no child. Contract 8's
`skill_collision` has a producer now.

### The write-time validator, and a ticket claim that expired

The store's validator parses the frontmatter with a real YAML parser rather than scanning
for `: `, because a scan is wrong in both directions. Measured on this machine 2026-09-06
with the validator itself: `writing-fragments` and `writing-shape` both carry `: ` inside
a **quoted** description and are valid YAML, and `3:1` with no space is valid too. What
breaks is a bare colon in an unquoted scalar, which is what `agy` drops and Cursor empties.

**Ticket 07 says `~/.claude/skills/handoff-frank` fails this today, and it no longer
does.** Ticket 24 measured that on 31 August and the file has been edited since; its
description is 656 characters and parses cleanly. Nothing in `~/.claude/skills` fails the
YAML rule as of 2026-09-06, so the validator's proof is a fixture rather than a file on
this machine.

Two rules beyond the file format, both of which the store needs and 6.4 implies. The
frontmatter `name` must equal the directory name, because `claude` keys on the directory
and `agy` on the frontmatter and a store entry where they disagree is a different skill to
each of them. And `disable-model-invocation: true` is refused, because it hides the skill
from the provider's own listing, which is the answer the launch check reads. Eight of the
eleven skills in `~/.claude/skills` set it, and none of the eight appears in `claude`'s
listing.

A store skill the validator refuses is skipped with a line on stderr rather than failing
the launch. The store is a directory a human and a curator both write to, and one bad
file in it must not stop every launch on the machine.

### What injection did to the listing, measured

The side effect 6.4 and contract 5.3 both warn about reproduced on this build. One real
`claude --model haiku` run with two injected links: the listing was 7,950 characters over
28 entries, and 5 of the 28 rendered as a bare name with no description. **Both injected
store skills were among the five.** So the model was shown two names it had no reason to
reach for, and `store_sha` plus `linked` would have claimed otherwise. That is the whole
argument for `skills.resolved` existing, arriving on the first run that used it.

`resolved` records identity and not description survival, for the reason above: the free
probe strips every filesystem description, so the field cannot carry what the paid run's
transcript showed. That is a smaller field than contract 5.3's paragraph implies and it
is the largest one that costs nothing. The run's own transcript holds the rest and
nothing reads it.

### A linked skill is unreachable under four of the five policies

This is the edge of the design rather than a gap in the code, and it needs a decision
rather than an implementation. It is map ticket 39.

Measured on 2026-09-06 with a real run and with that run's own compiled gate. Under
`standard`, a linked skill is listed to the model, and then:

- `Skill` is a `provider_native` tool, which only `permissive` grants, so the run cannot
  invoke it. The gate answered `hydra/out-of-scope` and the run derived `gated`. That
  half was already written down above and now has a second sighting.
- The link resolves into `~/hydra/skills-snapshots/<sha>/`, outside the checkout, so
  `file_read` scoped to `worktree` denies reading the `SKILL.md` too. The gate resolves
  symlinks before comparing, which is deliberate and is what `tests/gate.rs` calls
  `a_symlink_out_of_the_worktree_does_not_widen_the_scope`.

Both denials are correct against the documents as written. Together they mean injection
delivers a name and a description into the system prompt and nothing else under `judge`,
`read-only`, `standard` and `curate`. Under `permissive` it works, because that policy
grants `provider_native` and `file_read` anywhere.

**Re-measured on 2026-09-06 while preparing map ticket 39, and three of the sentences
above are too strong.** A gate compiled per policy and driven with synthetic payloads, no
provider launched, says: the `Skill` refusal's code is `hydra/capability`, not
`hydra/out-of-scope`, which only fires for a scoped capability whose path fell outside.
The body is out of reach under three policies rather than four, because `standard` grants
`shell` unscoped and `cat <worktree>/.claude/skills/<name>/SKILL.md` is allowed: the
snapshot is not one of 7.5's seven denied paths, so the capability boundary and the
practical one already disagree. And "under `permissive` it works" is read off the gate
rather than observed, since no run on this machine has ever had a `Skill` call allowed.
The ticket's prepared section has the per-policy table and the reason strings.

Nothing here was changed to fix it. 6.4 says the links point into a snapshot and 7.2 says
the `worktree` scope is the checkout, and no closed ticket connects the two. The store's
own scope is a second half of the same question, and ticket 12 answered it one way while
harness spec 7.2 and `docs/ouroboros-spec.md` 3.6 still answer it two ways. That half is
map ticket 40.

**Decided by map ticket 39 on 2026-09-06, and not yet built.** A linked skill is meant to be
invocable and readable. `Skill` becomes its own capability, `skill_invoke`, and the run's snapshot
becomes a read scope of its own, `skill_snapshot`, resolved from `hydra.skills.store_sha`. One
`permissive` run, `01a07886-a450-7a21-92cf-799b26358a05`, showed the body arriving on the `Skill`
call with no read the gate sees and the sibling arriving through `Read` on the link path, which is
what settled both halves. Build ticket 13 carries the code; until it lands, everything above this
paragraph still describes the binary.

## The canonical MCP document, and what a run is handed

Harness spec 6.5, built by ticket 08 for `claude`. `config/mcp.md` is protected config
#5 and the only place a server is declared; `src/mcp.rs` reads it and decides what one
launch gets; `src/provider/claude.rs` turns that into the file `--mcp-config` names.
`tests/mcp.rs` drives the whole path and launches nothing.

**The document is prose with one fenced `toml` block in it, and the block is the data.**
Everything else in the file is for whoever edits it. Exactly one block is allowed, and a
document with none is an error rather than a document with no servers, because a human
who breaks the fence would otherwise get a launch that silently admits nothing.

**A server is refused at parse time for four things**, each of which would otherwise
become a silent shortfall or an ambiguous record: an unknown field, so `admits` for
`admit` fails instead of admitting the server to nothing; an `admit` entry that is not
one of the five policy names; a name carrying `__`, because `claude` joins a server and
a tool with it and the record's `resolved` is read back off that join; and a server that
names both a command and a url, which `agy` refuses and which no one document can
generate for all three providers.

**Admission is not permission, and a real record now says so.** `mcp_call` is granted by
`permissive` alone (7.2), so a server admitted to `standard` connects, its tools reach
the model, and the gate refuses every call to it. Measured on run
`01a077c4-68e9-7203-9cc9-030044661116`: the model called `mcp__hydra-probe__ping` and
got `hydra/capability: policy standard does not allow mcp_call
(mcp__hydra-probe__ping).` back as the tool result. That is ticket 27's decision working
as written, and it is the same shape as the linked-skill finding above, which is map
ticket 39. Nothing here was changed to fix it and the document says it out loud instead.

**`judge` is the one launch shape that can be handed no server**, so 6.1 item 6's check
has exactly one instance today. 6.7's `judge` argv carries no `--mcp-config`, so a
document admitting a server to `judge` is a launch that cannot deliver, and the run is
refused with `mcp_unavailable` before anything spawns. Everything else delivers what it
admits, and a policy that admits nothing proceeds untouched, which is what keeps `judge`
runs launching on a machine whose document names servers for other policies.

**A run that admits nothing still gets a file.** `{"mcpServers":{}}` plus
`--strict-mcp-config` is the difference between a run with no servers and a run with
whatever the human has configured, and it is what lets the record claim the set.

**`mcp.doc_sha` is a sha256 of the whole file, prose included.** So `shasum -a 256
config/mcp.md` reproduces what a record carries, and a reader holding a record can check
the document in front of them with one command. It is the opposite call from
`policy_sha`, which leaves `policies/README.md` out of its hash so a wording fix does
not move a value every record carries. The two differ because there the prose is a
separate file, and a sha over part of a file is one nobody can recompute by hand.

**The document ships with no server**, because nobody has configured an MCP server on
this machine. What is in it is the format and one commented example per transport. Both
the unit tests and `tests/mcp.rs` assert that emptiness, so the day a human adds a real
server the suite says which tests now describe something else.

**Measured on two real runs, 2026-09-06, `claude` 2.1.263, `--model haiku`.** Run
`01a077c3-e32f-7612-bb19-89227bf6f531`, `standard`, on the shipped document: both flags
were accepted, `mcp.json` was `{"mcpServers":{}}`, the init frame reported
`mcp_servers: []`, the record carried `doc_sha` plus two empty lists, and the whole call
took 6.8s. The `doc_sha` is what `shasum -a 256 config/mcp.md` prints, checked by hand.
Run `01a077c4-68e9-7203-9cc9-030044661116`, `standard`, with one stdio server admitted:
`injected` and `resolved` both read `["hydra-probe"]`, the init frame independently said
`[{"name":"hydra-probe","status":"connected"}]` and listed `mcp__hydra-probe__ping` among
its tools, and the call took 12.6s. **The second run's document is not the one that
ships.** The server was added by hand for the measurement and taken out again, so that
record's `doc_sha` names a document no longer on disk, which is the honest thing for a
record to do and worth knowing before anyone tries to reproduce it.

### `--restricted` no longer drops MCP, so a `judge` run inherits the human's servers

This is the edge of the design rather than a gap in the code, and it needs a decision
rather than an implementation. Nothing was changed for it.

Ticket 23 measured `claude` 2.1.257 dropping all MCP under `--restricted` "despite the
help text saying otherwise", and 6.5 and 6.7 were both written from that: 6.7 leaves
`--mcp-config` off the `judge` argv because "`--restricted` drops it anyway", and 6.5
says `claude` closes ticket 27's inherited-server residue.

Measured 2026-09-06 on 2.1.263 with the free probe, no API call: `claude --restricted
--mcp-config <file>` resolved the server in that file. 2.1.263's own `--help` now says
`--restricted` ignores user, project and local settings files and "add
--strict-mcp-config to skip MCP servers too". Both point the same way, so the claim has
expired rather than being wrong when it was made.

What follows for this build. A `judge` run carries neither MCP flag, so on 2.1.263 it
keeps whatever the human has configured in `~/.claude.json`. The gate refuses every
`mcp__` call under `judge`, which allows nothing at all, so what is left is server
processes starting and their tool descriptions reaching the model's context. That is
ticket 27's residue, arriving on the one provider 6.5 says is free of it.

**Unmeasurable on this machine, and said out loud rather than assumed.** Nobody has a
user-scope MCP server configured here, so the inheritance itself was not observed. What
was observed is that `--restricted` keeps servers passed on `--mcp-config`, which is the
same code path in the binary. Measuring the other half means writing a server into the
human's own config, which is theirs to do.

The fix is one flag: `--strict-mcp-config` on the `judge` argv, with no `--mcp-config`
beside it, which would give that policy an empty server set the way the other four get
theirs. It is not taken here because 6.7 writes the `judge` command line and changing it
is a design decision. Whoever takes it should also decide what a `judge` run's
`mcp.resolved` becomes, since with the flag the probe could answer for it and today it
is null.

### Asking `claude` which servers it got

**One probe answers both `resolved` fields.** Ticket 07's free probe already ran per
launch; it now carries the run's own `--mcp-config` and `--strict-mcp-config`, and its
transcript's `deferred_tools_delta` attachment is the answer. Measured 2026-09-06 on
2.1.263 against `tests/fixtures/mcp-probe-server.py`, a stdio server written for this:
`addedNames` gained `mcp__hydra-probe__ping`, and the same attachment carries
`pendingMcpServers`, `needsAuthMcpServers` and `failedMcpServers`, the last as objects
with a `name` and an error string. The probe costs 1.8s with an empty config and 1.9s
with one server in it, against the 1.8s ticket 07 measured with no MCP flags at all.

**The probe carries the run's flags so that its answer is the run's.** Without them it
would resolve the human's servers and the record would claim them for a run that will
not have them. `--strict-mcp-config` on the probe is chosen for that reason rather than
reached for as a speed lever, which is the decision ticket 07 left open. The cost is
that every admitted server starts twice per launch, once for the probe and once for the
run.

**A `judge` run's `resolved` is null rather than empty**, and it is one of three producers
of that null. The probe is not `--restricted`, so it cannot reproduce what a `judge` run
resolves, and an empty array there would be a claim rather than a measurement. The other
two are a probe that failed, and a run with no checkout, where the probe never runs because
it is the same process that answers `skills.resolved` and 6.4 gives that run no worktree to
ask in. Contract 5.3 already requires a reader to tell all three apart from an empty
listing.

**A server that connects and offers no tool is invisible to this reading**, since the
answer is derived from tool names. Nothing on this machine has one, and the run's own
init frame is where a later reader would go: it carries `mcp_servers` with a `status`
per server, measured as `[{"name":"hydra-probe","status":"connected"}]` on the run
above. That frame arrives after line 0 is written, so it corroborates the probe rather
than replacing it, and a reader wanting the provider's own words takes it from `raw`.

**A server that will not start does not refuse the launch.** 6.5 gives the fatal
pre-flight to Cursor alone and says why: on `claude` the strict flag already guarantees
the set. So a failure is a line on stderr and a difference between `injected` and
`resolved` in the record, which is exactly what that pair of fields is for.

## `cursor-agent`, the second provider

Ticket 09. `src/provider/cursor.rs` holds every fact that is true of that binary,
`src/normalize/cursor.rs` reads its records back, and `src/policy/table.rs` carries its
capability table beside `claude`'s. Every claim below was measured on 2026.09.02-c22c1a3
on 2026-09-06 unless it says it was read off the bundle at
`~/.local/share/cursor-agent/versions/2026.09.02-c22c1a3/`, and contract 9.3's warning
applies to all of it: the display name in `model` tracks the account's selection, not
the binary, so a claim about it can expire without a version moving.

**Two real runs, both under `--model composer-2.5`, and the corpus walks pass with
both in it.** Run `01a07891-a339-7173-96d9-70beb88e5efb`, `standard`, was asked to read
`Cargo.toml`, run `echo T09HYDRA` and write `t09-probe.txt`: all three went through,
the record is 33 lines, `completed` with `usage_source: provider` (12,856 input, 320
output, 73,256 cache read, 0 cache write), `diff_stat` one file and one insertion, and
11.9 seconds from line 0 to `run_end`. Run `01a07892-459d-7420-904e-91d6a6986159`,
`read-only`, was asked to read `/etc/hosts`, read `Cargo.toml` and write the same file:
the first was refused with `hydra/out-of-scope`, the second went through, the third was
refused with `hydra/capability`, the model answered `BLOCKED\nOK\nBLOCKED`, and the
record derives two `permission_decision` denies by `hydra_policy` beside one allow, no
finding, and `completed`, because one call got through and that is the predicate the
outcome section above defends. The first run was reaped afterwards to check that a
spawn directory holding Cursor's socket file goes cleanly; it did, and the record stays.
Two probes outside Hydra preceded them, one exercising six tools under a logging hook
to learn the hook's spellings and one denying a read at `beforeReadFile`, and both are
what the frame shapes in `tests/common/mod.rs` are copied from.

**The gate answers in Cursor's own shape, on both events, and 6.6's "two shapes rather
than three" sentence is one shape short.** 6.6 says the script answers in two shapes
because Cursor accepts Claude's response dialect on `preToolUse`. It does, through a
translation the bundle applies to that one event and to no other, and `beforeReadFile`
gets none: research 26 read it accepting `allow` or `deny` in Cursor's own `permission`
field, and the second probe measured that shape denying a read. So the `cursor` dialect
answers `permission` plus `user_message` plus `agent_message` on both events, which
research 30's rig had already used on `preToolUse` and had obeyed. The tamper wrapper
speaks the same shape, because a refusal Cursor cannot parse is an invalid response that
`failClosed` blocks with the reason lost. Three shapes, then, one per provider, and the
gate still decides once and only speaks at the end.

**The gate is compiled for one provider and refuses to be asked in another's dialect.**
The compiled config carries `dialect`, and `hydra/wrong-dialect` is the answer when
argv names a different one. A gate compiled for Cursor that was asked in `claude`'s
shape would otherwise answer for a tool table the provider never calls.

**`preToolUse` fires for reads on this build, so `beforeReadFile` is the second line
rather than the first.** 6.6 registers `beforeReadFile` because "`preToolUse` alone
leaves the file-read path ungated on that provider", from ticket 26. Measured on
2026-09-06: a `Read` fires `preToolUse` with `tool_name: Read` and `tool_input:
{file_path}` before the read executes, and `beforeReadFile` fires after the read
succeeds inside the provider and before the content reaches the model, carrying the
path, the content and no tool name. On the shipped gate the first hook refuses an
out-of-scope read and the second never sees it; the second probe, with a hook that
allowed at `preToolUse`, measured the second event refusing on its own. Both are
registered, with `failClosed: true` on each, and the gate maps a `beforeReadFile`
payload onto `Read` with its measured path field. Whether the next build still fires
the first for reads is exactly the claim 6.6 declined to rest on.

**The hook's spelling of a tool is not the stream's, and the table holds the hook's.**
Five names arrived on the hook across the two probes: `Read`, `Shell`, `Write`, `Grep`
and `Delete`. Thirteen more are read off the bundle's hook descriptors, each of which
builds the `tool_input` the hook sees: `List`, `ReadLints`, `Fetch`, `WriteShellStdin`,
`task`, `FetchMcpResource`, `ListMcpResources`, `ComputerUse`, `RecordScreen`,
`askQuestion`, `createPlan`, `generateImage` and `updateTodos`. `WriteShellStdin` is
`shell` with no command in its input, so the pattern floor cannot see what it feeds a
background shell, which is 7.4's failure mode on a second spelling and is the reason
the row says so. An MCP tool arrives as
`MCP:<tool>`, read off the same bundle and unmeasured because no server is configured
here. Three spellings are Cursor's own and would otherwise surprise somebody. The
listing tool arrives as `Grep` carrying `output_mode: files_with_matches`, not as a tool
of its own, though the stream calls it `globToolCall`. A `Write` fires a `Read` hook for
the same `tool_use_id` first, so a write under `read-only` is refused on the second
hook. And a `Delete` is `file_write` on the path it names. There is no init frame to
check the table against, unlike `claude`, so 7.1's run-time table check has nothing to
read on this provider and a name it does not know is caught by the denial that reveals
it.

**A `Skill` split would land on `Read` here.** Cursor has no `Skill` tool. A linked
skill is rendered into the prompt as an `<agent_skill fullPath=...>` element and the
model reads its `SKILL.md` through the ordinary read tool. So if map ticket 39 splits
`Skill` out of `provider_native` into a capability of its own, this table has no row to
move; what moves is which scope a `Read` of a path under `~/hydra/skills-snapshots/`
resolves to, which is the same question ticket 39 already asks about the link
resolving outside the `worktree` scope. The table is built as the design stands today.

**The normalizer rebuilds the hook's spelling from the stream's, and that is what
`tool_call.arguments` carries.** The stream says `editToolCall` with `path` and
`streamContent`; the hook that gated it saw `Write` with `file_path` and `content`.
Contract 7.7 re-evaluates the policy against what the gate saw, so the adapter maps
each measured oneof key back to the hook's name and input the way the bundle's own
`createToolInput` does, and the stream's object survives in `raw`. A key it does not
know keeps its own spelling with no capability, which is an `UnknownTool` finding.
Both events for a call come from frames sharing one id, the frame's `call_id` and the
object's `toolCallId` agreed on every frame measured, and a `completed` frame for a
refused call carries `args: null`, so the call is taken from `started` and the result
alone from `completed`.

**Hydra's voice is read out of the refusal text, and nothing else is.** There are no
hook frames on this provider, so the tool result is the only second source on a
refusal, and 7.7 says its reason text cannot carry attribution. It cannot, and the
decider is derived from the policy as on `claude`. What the text can carry is whether
Hydra's gate spoke and which code it spoke, because every gate answer starts with a
`hydra/<code>:` prefix and no other hook's does, and the code is checked against the
gate's own list so a guard quoting `~/hydra/runs` is not mistaken for it. Cursor
concatenates every hook's message into one reason, allowing hooks included, so
`hydra/allow` inside a refusal means Hydra allowed and something else denied, which is
`external`. A refusal with no code at all beside Cursor's own `Agent note: Do not
suggest workarounds to the blocked tool.` line, or a `rejected` case on a shell command,
is somebody else's. Where a denied read lands on the stream: the message is
`readToolCall.result.error.errorMessage`, with `File read was blocked by a hook: ` in
front of it when `beforeReadFile` refused and nothing in front when `preToolUse` did; a
denied write is `editToolCall.result.error.error`; a denied shell command is
`shellToolCall.result.rejected.reason`.

**An allowed call raises no `Ungated` finding on this provider, and the reason is the
config.** On `claude` a hook that fails to start fails open, so a call with no gate
answer behind it is worth a finding. Here the config says `failClosed: true`, a hook
that failed to run would have blocked the call, and the stream carries no gate answer
for an allowed call anyway, so its absence says nothing. The finding survives for the
one shape where it means something: the policy denies, something refused, and Hydra's
voice is not in the text, which is a config the loader did not run, and the loader
refuses a symlinked one silently.

**No terminator means `failed`, not `crashed`, on this provider.** Harness spec 10:
`cursor-agent` writes its `result` frame only on success, and its every hard failure
is the `catch` path, one sentence on stderr, no frame, exit 1. Over the pty Hydra owns
that sentence is a line of the record, kept as a string per contract 3, and the
normalizer keeps those lines on `Run::noise`. Two of them are refusals rung one must
not learn from, and the derivation reads them: `ActionRequiredError` beside `usage
limit` is `rate_limited`, which research 33 measured twice on 2026-09-04 by pinning a
premium model, and `Authentication required` is `auth_failed`, which research 21 read
out of the bundle's logged-out messages. That is a text match and it is written down as
one; a reworded sentence sends the run back to `failed`. Neither was reproduced on a
Hydra run, because reproducing the first means burning a premium model's refusal and
the second means logging out. A signal death reports the same exit code 1 through the
pty, which is why the detached half now writes the signal into `run_end.error` and the
derivation reads that note first.

**Two noise lines per run, and one of them is new.** Cursor writes the same bare
show-cursor escape `claude` does after its last frame, and before its first frame it
writes a coloured tip, `Tip: You can start the Cursor CLI with 'agent' (same as
'cursor-agent').`, on stderr, which the pty merges into the record. Both are kept as
strings and normalize to nothing.

**The fatal pre-flight is three free commands, and its listing names the servers the run
will start.**
6.1 item 7, built as 6.5 writes it: `cursor-agent mcp enable <name>` for each injected
server, run with cwd inside the worktree because `enable` resolves the project from cwd
and ignores `--workspace`; `mcp list`; `mcp disable <name>` for every server the listing
names that Hydra did not inject, which is 6.5's per-run kill switch for the human's
user-scope servers under the redirect; then `mcp list` again. Any injected server the
final listing does not report `ready` refuses the run with `mcp_unavailable` and a
remediation naming the pre-flight and the status each server reported. A disable that
fails is a line on stderr rather than a refusal, because the gate refuses every call
to a server Hydra did not inject whatever the policy, so the cost is exposure and not
execution, and the final listing reports the server `ready` for the record to keep.
The sequence is pinned by a unit test that drives it without the binary. Measured by hand
on 2026-09-06 against a worktree naming `tests/fixtures/mcp-probe-server.py` and
`/bin/echo` under a scratch `CURSOR_DATA_DIR`: both `not loaded (needs approval)` before,
`hydra-probe: ready` and `broken: Error: Connection failed` after the enables, the
approval written under the scratch directory, and `~/.cursor/projects` untouched.
`tests/mcp.rs` drives the verdict and the refusal record from that listing shape.
**Not measured through `hydra run`**, because `config/mcp.md` admits no server and is
protected config #5; the two real runs each listed nothing, wrote `{"mcpServers":{}}`,
and recorded `injected: []` beside `resolved: []`.

**`mcp.resolved` is the pre-flight's listing, free, and `skills.resolved` is null.**
Ticket 08's note to this ticket said both answers were paid. The MCP half is not: the
listing after the approvals is the provider reporting what the run will start, so the
`ready` names are `resolved`. The skill half is paid, because ticket 24 put it in
`~/.cursor/chats/<hash>/<session>/store.db` after a real run, so the launcher says so on
stderr and writes null, which contract 5.3 makes optional on this provider, and the
third collision test does not run. The other two do, against six user-scope roots
rather than `claude`'s one, and `~/.cursor/skills-cursor` holds twenty-odd of Cursor's
own skills on this machine, so a store skill named `review` or `loop` would refuse a
Cursor launch today.

**`CURSOR_DATA_DIR` moves more than the approval state, and all of it is reaped.** 6.5
says the redirect moves `projects/<slug>/` with `mcp-approvals.json` and
`mcp-disabled.json`. Measured after the first run, the run's `cursor-data/` also held
`mcp-auth.json`, `repo.json`, `worker.log`, a `worker.sock` under a second project
directory, and `agent-transcripts/<session>/<session>.jsonl`, the transcript the hook
payload's `transcript_path` points at. `~/.cursor/projects` gained nothing across both
runs. What did not move is what 6.5 said would not: `~/.cursor/chats/<hash>/<session>/`
gained a `store.db` for each run, so the harvest source of section 10 is where the
design expects it and the per-project transcript is a second copy that dies with the
spawn directory under `hydra reap`. The variable's name lands in `env_allowlist`,
because contract 5.3 records the names passed and this one was, and its value travels
to the detached half in the launch spec as a path Hydra computed rather than a value
the launcher inherited.

**A `cursor-agent` run needs a checkout of Hydra's own, and two things about the
parent repo refuse it before a run id exists.** The hook config, the MCP config and the
skill links are all read from cwd, and cwd is the worktree, so a run outside a git repo
would have to write `.cursor/` into a directory Hydra does not own. Harness spec 2 says
a provider Hydra cannot gate is not launched, so the launch stops there with an error
and no record, the same way a wrong PATH does. And a repo that already carries
`.cursor/hooks.json` or `.cursor/mcp.json` is refused the same way, because the worktree
is seeded from it and Hydra has to write both files: writing over the human's copies
edits their work and `hydra reap` would commit the edit, and loading theirs launches an
ungated run. `write_spawn_config` and `mcp::write_config` refuse a checkout that holds
either file as the second line. 6.5's `--shared-tree` is the same shape one flag over,
and `mcp::plan` takes whose tree the run stands in so that a Cursor run with no tree of
Hydra's admits no server, which is what `Checkout::None` compiles to today.

**The two files Hydra writes into `.cursor/` are ignored by name.** The skill root
ignores itself with `*` (6.4); `.cursor/` cannot, because the human's repo may track its
own `.cursor/rules/` and a `*` there would hide a file the child creates from the commit
`hydra reap` makes. So `.cursor/.gitignore` lists `/hooks.json`, `/mcp.json` and itself,
an existing one is left alone, and the first run's reap committed exactly the probe
file and nothing of Hydra's.

**The argv is 6.7's line with two spellings settled.** `-p` is `--print` on this binary
and the prompt is a positional, so 6.7's `-p <task>` is one boolean flag and one
argument, task last. No `--workspace`, because cwd is the worktree and `mcp enable`
ignores the flag anyway. `--force` covers the workspace trust gate research 14 found
as well as the permission layer.

**`permissionMode: "default"` on every init frame is the hardcoded literal research
01 found**, and it still lies under `--force` on this build. Nothing reads it.

## What the record does not carry

**Both injection groups are written now.** Ticket 08 closed the last one: every recorded
`claude` run carries `mcp` with `doc_sha`, `injected` and `resolved`, so the section
this one replaces, which said `mcp` was absent because nothing MCP was injected, no
longer describes anything. On `cursor-agent` both groups are present too, and one half
of one is null on purpose: `skills.resolved` is the paid listing, so the record says
nobody asked rather than claiming an empty one, and `mcp.resolved` is the pre-flight's
own listing, which is free. The section on that provider has the reason for each.

**`skills` is present on every run with a checkout and absent on the rest.** Ticket 07
closed that half: a run in a repo carries `store_sha`, `linked` and `resolved`. A run
outside a repo gets no worktree, and 6.4 puts injection in the worktree and nowhere
else, so there is no location to link into and the question does not apply.

**`mcp` is present whether or not there is a checkout**, and the asymmetry is 6.5's
rather than an oversight here. On `claude` the servers ride the command line, so a run
outside a repo still gets exactly the set Hydra wrote. A record with `mcp` and no
`skills` is that run.

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

**Two providers.** `src/normalize/` turns a stored `claude` or `cursor-agent` run into
contract 7's events, one adapter per column of 7.2, picked by the envelope's `provider`.
`read_file` takes a path and the policy set; `read_with` takes text and a layout, which
is how the tests ask a real question about a synthetic run without touching `~/hydra`.
Nothing is written, which contract 11 requires and `tests/normalize.rs` checks by
comparing every byte under `runs/` before and after a walk. What follows in this
section is the `claude` adapter; the `cursor-agent` section below has the other.

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
`system`/`init` frame, which lists every tool at no cost. On 2026-09-06 `claude` 2.1.263
offered twenty-nine names, four more than the twenty-five 2.1.261 offered the day before:
`TaskCreate`, `TaskGet`, `TaskList` and `TaskUpdate`. Until this change gave them rows
they denied under every policy, `permissive` included, which is what the check is for.
They are `provider_native` now, so it is quiet again and the next rename breaks the first
record written after it.

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

**`crashed` reads the terminator's absence, and now one signal note.** `claude` writes
exactly one `result` frame on every path, so a `run_end` with no terminator behind it is
a child that stopped mid-run. With a null `exit_code` it is `failed` instead, because no
process ran and nothing died. Ticket 09 added the one witness the record can have: the
pty reports a signal death as exit code 1, so the detached half writes `the child died
on signal <name>` into `run_end.error` when it saw one, and the derivation reads that
note ahead of the tee-error branch. It matters on `cursor-agent`, where exit 1 with no
terminator is also the provider's orderly failure path; the section on that provider
has the split.

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

**`--mcp-config` and `--strict-mcp-config` are passed together on every non-`judge`
recorded run**, which is how 6.7 pairs them, and the section on MCP below says what the
file they point at holds. A `judge` run carries neither, which is 6.7's own argv and is
now the reason its policy can be handed no server.

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
call long and never reached for either.

**Ticket 07's first injected run hit it, on 2026-09-06.** Run
`01a07512-d36e-7651-b8d9-e9e279e5730b`, `standard`, `--model haiku`, two skills linked
and a task naming one of them. The gate refused `Skill` with `hydra/out-of-scope`, the
model replied "The policy compiled for this run does not allow the Skill tool", and the
run derived `gated`. The loss is real and it is now on the record twice. The section on
the skill store above has the other half, which is that the link resolves outside the
`worktree` scope so the `SKILL.md` cannot be read either.

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

**One ordering the skill store broke, and it is live now.** The checkout is of the
store's `HEAD`, and 6.4 has a launch commit a dirty store before pinning it. `hydra
curate` checks out before it launches, so a pass started with an uncommitted skill
in the store reads the store as it was before its own launch tidied it. Ticket 07
wired the pinning and did not touch this, because it is an ordering decision rather
than a line of code: either the store is committed before the curate checkout, or
the checkout moves after the launch and the gate is compiled against a directory
that does not exist yet. Neither ticket decided it, so it is open. It is map ticket 41,
blocked by 40 because the order depends on where the checkout lives. The pass measured above hit the harmless half, an empty `HEAD` under a
store whose skills were not committed.

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
match on. One of the four is written now, `skill_collision`, and a record carries it, so
that spelling is settled and the other three are still free to move.

**`hydra reap` is what removes a skills snapshot.** Harness spec 5 says a snapshot is
reaped when no live run references it and names no command. Reap is the one already typed
by a human that already removes a run's scratch.

**The store's pin commit is attributed to `hydra <hydra@localhost>` with the run id in a
trailer.** 6.4 asks for authorship in the trailer and does not say whose. Hydra cannot
know which run dirtied the store, so the trailer names the run that turned the working
tree into a commit.

**The store's validator holds two rules the file format does not.** The frontmatter
`name` must equal the directory name, and `disable-model-invocation: true` is refused.
The section on the skill store above gives the reason for each. Both are stricter than
6.4 says in words and both follow from what it asks the store to do; confirm them rather
than inherit them.

**The skill probe leaves a transcript under `~/.claude/projects/` per run and nothing
reaps it.** That is the user's own directory and Hydra writes nothing else in it.

**The canonical MCP document is Markdown with one fenced `toml` block in it.** 6.5 fixes
the file's name, says it is none of the three provider files, and requires a per-server
admission field keyed on a policy name. The rest was open. Markdown because the file is
protected config #5 and a human edits it by hand, so the format that carries the rules
beside the data wins; TOML for the block because the skill store's `hydra.toml` sidecars
already use it and a table of servers reads well in it. One block rather than one per
server, so a broken fence is an error rather than a server nobody notices is missing.

**`mcp.doc_sha` is a sha256 of the whole file.** The section above has the argument and
the alternative it beat.

**A server name may not carry `__`, and an unknown field is refused.** Neither rule is in
6.5. The first is what makes `mcp.resolved` exact rather than a guess, since the free
answer is `mcp__<server>__<tool>` split back apart. The second is the same instinct
`deny_unknown_fields` serves everywhere: a typo in an admission field is a server that
reaches no run, which is the failure the launch check exists to make loud.

**The document ships with no server on this machine.** Adding one is a human's decision:
15.1 calls a server definition a command Hydra spawns inside every future run. The one
in `tests/fixtures/mcp-probe-server.py` is a fixture for measuring, not a server, and it
is not admitted to anything.

**The `cursor` dialect is Cursor's own shape, on both events.** 6.6 counts two response
shapes; ticket 09 found `beforeReadFile` takes only Cursor's, and the section on that
provider has the measurement. One shape per provider is the rule that survives.

**A `cursor-agent` run outside a repo, or in a repo carrying `.cursor/hooks.json` or
`.cursor/mcp.json`, is refused before a run id exists.** Harness spec 2 gives the rule
and 6.6 the mechanism; which check runs and when is this build's, and the same section
argues it.

**`.cursor/.gitignore` names the two files rather than everything.** 6.4's `*` is right
for a directory Hydra alone writes into and wrong for one the human's repo may track.

**`CURSOR_DATA_DIR` is on `env_allowlist`.** Contract 5.3 says the names passed, and 6.2
calls it a variable Hydra sets rather than one it passes through; the record follows
5.3, so a reader sees the redirect was in force.

**`mcp.resolved` on `cursor-agent` is the pre-flight listing's `ready` names.** Ticket
08 expected null. The listing is free and is taken after the approvals and the
disables, so it is the provider's own account of what the run will start.

**`tool_call.arguments` on `cursor-agent` is the hook's spelling of the input, rebuilt
from the stream's.** Contract 7.6 names the field and not its shape; 7.7 re-evaluates
against what the gate saw, and the gate saw the hook's spelling. The stream's object is
in `raw`.

**A signal death goes into `run_end.error` as `the child died on signal <name>`.**
Contract 6 defines the field as text Hydra captured that is not on the stream, and this
is that. It exists because the pty reports a signal death as exit code 1, which on
`cursor-agent` is also an orderly failure.

**Two `cursor-agent` outcomes are read off stderr.** `rate_limited` and `auth_failed`
have no frame to derive from on that provider's failure path, so the derivation reads
the noise lines the record keeps, keyed on the provider's own error class and phrase.
The section on that provider says why that is a text match and what happens when the
sentence moves.

## Not built

Out of scope for this slice, and each one has a document that already says what it is:

- Two of harness spec 6.1's seven pre-flight checks. Auth, depth and policy containment
  belong to tickets that are not built. What runs before a spawn today is the
  environment, the provider version, the policy name, the preamble file, the MCP
  document, the concurrency cap, the skill collision check, the MCP launch check and,
  on `cursor-agent`, item 7's fatal pre-flight.
- Delegation: `--parent`, depth, monotonic narrowing, the MCP delegate face. 7.3, 9.
- Every one of contract 8's nine outcomes has a producer now. `mcp_unavailable` was the
  last without one and it arrives through `run_end.refusal`, written by 6.1 item 6 when
  a policy admits a server the launch cannot deliver. `skill_collision` is 6.1's fifth
  check, and `timed_out` is the handle rather than the record, which the section above
  argues. No record on this machine carries `mcp_unavailable`: `tests/mcp.rs` drives the
  refusal against a fixture document, and the shipped document admits nothing to
  anybody, so producing one for real means a human adding a server.
- The `agy` adapter, ticket 10, and harvest, 12. Contract 7.2's third column and 7.2.1's
  transcript vocabulary have no reader.
- Interactive runs and harvest. 5.6, 12.
- Five of harness spec 17's eleven commands: `show`, `resume`, `evolve`, `export`
  and `usage recount`.
- Ouroboros rung two. `hydra evolve`, the protected-path refusal, the admission
  gate, the panel of three and the `ApprovedPatch` type are all absent, and so is
  the harness run a pass would write (contract 5.5). Rung one is built and the
  section above says what it does. The review checklist of ouroboros spec 5 is a
  document rather than code: nothing reads it, because nothing reviews yet.

## Rough edges worth knowing

- **`~/hydra` is this checkout on the machine this is developed on**, so everything
  harness spec 5 writes lands inside the source repo and `.gitignore` is what keeps it
  out of git. Ticket 07 found the trap that comes with it: the patterns were unanchored,
  so `skills/` matched `src/skills/` at any depth and hid a new source directory from
  `git status` with no warning. They are anchored to the repo root now. A new directory
  under `src/` sharing a name with one of Hydra's own is worth a `git status` glance
  either way.
- **A record does not carry the paths its scopes resolved to.** `policy` and `policy_sha`
  pin the rules; `worktree`, `skill_store` and `corpus` are resolved at compile time from
  `Layout` and go nowhere. Contract 11 re-derives `permission_decision` by running the
  policy again, so a build that moves where a scope points makes every earlier record
  re-derive differently, and `policy_sha` does not move because no policy file changed.
  Seen for real on 2026-09-06: a concurrent branch pointed `skill_store` at a per-run
  worktree of the store, and its run's ten in-scope reads re-derive on this branch as ten
  `GateLeak` findings. Nothing is wrong with either build. The record is missing the
  field that would tell them apart. That is map ticket 42, and ticket 40 is the same
  question seen from one scope.
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
