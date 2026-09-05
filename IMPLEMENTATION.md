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

**Ticket 02's code review found all seven reachable by redirect**, including the run's
own gate script, which is 7.5's induction base case. That one composed into a full
bypass: the script is `0o700` and owned by the uid the child runs as, an emptied script
exits 0 and prints nothing, and 6.6 measured `claude` failing open on a response in a
shape it does not recognise. Two `Bash` calls under `standard` ended the gate for the
rest of the run. `--permission-mode bypassPermissions` is what had removed the layer
previously refusing those calls, so the flag and the hole arrived in the same change.

**A shell command that names one of the seven is now refused whole.** Every token of
every segment is expanded, resolved and compared against the deny list, the tail of a
`--flag=/path` option included. Measured on run
`01a07340-67a3-7c41-8c69-8d1e28bb58a7`, `standard`, on a real `claude`: a `Bash` append
to `~/hydra/runs/` was refused with `hydra/protected-path: the command names
/users/user/hydra/runs`, the reason reached the model, no file was created, and a
second `Bash` call writing inside the worktree in the same run was allowed and landed.

**The refusal is coarse and reads are refused too.** Which token is a redirect target is
not decidable without running the command, and neither is which is an argument to a
writing binary, so `cat ~/hydra/runs/r.jsonl` is refused under `curate` as well. That is
a false positive in the same direction 7.4's floor already takes. The precise access is
the file tools, which carry a path: `curate` scopes `file_read` to the corpus and reads
it that way.

**What is still open: a path the command never spells.** `echo x > $HOME/hydra/runs/r`
walks past, and so does one assembled from a variable. Spelling it inside a substitution
does not help an attacker, because the token is still there and still caught, but a
path the shell builds is not, and catching it means evaluating the command, which 7.4
rules out. This is a floor with 7.4's own failure mode, silent permission on the
spelling nobody wrote down, and it is not a gate.

**Closing the residual needs a mechanism that does not read argv.** The options all
trade something the design has argued about, so it is a decision for the map rather than
a patch: denying `shell` its unscoped grant rewrites 7.2's `standard`; running the child
under a second uid or a sandbox is nowhere in the spec; and moving the gate's logic into
the hook config, which all three providers snapshot at process start, would close the
base case by construction but departs from 7.5's named artifact.

`tests/gate.rs` carries four tests over this: the denial, its coarseness, the residual
pinned so that closing it breaks a test on purpose, and ordinary shell left alone.

Provider versions this slice was built and measured against, read 2026-09-05:

| Provider | Version | Read |
|---|---|---|
| `claude` | 2.1.261 | 2026-09-05. The specs were written against 2.1.257; every flag in harness spec 6.7 survives the move |
| `cursor-agent` | 2026.09.02-c22c1a3 | 2026-09-05, unchanged since 2026-09-02 |
| `agy` | 1.1.26 | 2026-09-05, unchanged since 2026-09-04 |

## The one command

```
hydra run --provider claude --task "..." [--model <id>] [--policy <name>]
          [--allow-floor-escape]
```

It builds the child's environment from the allowlist, reads the provider's version,
loads the policy set and the preamble, mints a UUIDv7 run id, creates a git worktree and
seeds it with the parent's uncommitted work, compiles the run's gate into
`~/hydra/spawn/<run-id>/`, spawns `claude` on a pty it owns with `--settings` pointing at
the hook config, writes `run_start`, tees every frame verbatim into
`~/hydra/runs/<run-id>.jsonl`, and writes `run_end` when the child exits. It prints JSON
on stdout.

Section 9 gives `hydra run` eight optional flags and three are here: `--model`,
`--policy` and `--allow-floor-escape`. The five that are not each belong to a ticket
that is not built: `--parent` and `--files` to delegation, `--timeout` and
`--shared-tree` to the detached spawn, `--allow-billed` to the raw-model plane.

`cursor-agent` and `agy` are routable in the design and refused here by name, with an
error that says the slice routes one provider rather than that the provider is
unknown.

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
indistinguishable in the record from one that was never offered the lift, and 7.7's
re-derivation of a permission decision from `policy` and `policy_sha` gets the wrong
answer on any run where a human typed it.

**Nothing derives `permission_decision` yet.** The gate's answers are in the record and
no reader turns them into the contract's event type. Contract 7.7 derives it by
re-evaluating the run's own policy, and re-evaluation is the normalizer's job, which is
ticket 03. What a record carries today is the raw evidence: `hook_started` and
`hook_response` frames naming `PreToolUse:<tool>` and carrying the gate's verdict
verbatim, the reason string on the tool result with `is_error: true`, and an entry in
`result.permission_denials[]` with its `tool_use_id`. The gate-leak check waits on the
same ticket.

Everything else in contract sections 2 to 6 is met, and `tests/record_contract.rs`
checks it rather than asserting it. That test also walks every record under
`~/hydra/runs/` on this machine and fails if any of them drifted.

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

**The spawn directory's filenames are `gate` and `settings.json`.** Harness spec 19.1
leaves the names inside the spawn directory open. `settings.json` comes from 6.6's own
table; `gate` comes from 7.5's prose. Neither is a ticket's decision.

**The preamble and the caller's task are joined by a blank line and nothing else.**
Section 8 allows the preamble three assertions and no more, and a marker line saying the
task follows is a fourth sentence in Hydra's voice. The cost is that a reader of
`run_start.task` has to know where the preamble ends, and the preamble is a fixed string
in `config/preamble.md`, so they can.

**Worktrees live at `~/hydra/worktrees/<run-id>/`, on branch `hydra/<run-id>`.** No
closed ticket names the location, which is the same gap harness spec 19.1 flags for
the filenames inside the spawn directory. A Hydra-owned path keeps D9 true when the
repo being worked on lives somewhere else. Confirm it rather than inherit it.

**`run_start` is written after the spawn, not before it.** Contract section 5 says the
line is written before the child process starts and 5.2 puts the child's pid on it.
Both cannot hold literally. What run_end's derivation leans on is that the pid is in
the file, and a pty buffers everything the child writes until the tee reads it, so
spawning first loses nothing and `run_start` is still line 0.

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

**`hydra run` returns fewer fields than contract 17 specifies.** `outcome`,
`outcome_source`, `usage` and the final assistant text are all normalizer output, and
the diff stat would report the seed plus the child's work rather than the child's. A
null `usage` reads as a measurement of zero, so the JSON carries a
`not_yet_derived` list naming the five instead. That field goes away when they land.

## Not built

Out of scope for this slice, and each one has a document that already says what it is:

- Skill injection and the store. 6.4. `~/hydra/skills/` is the `skill_store` scope and
  nothing creates it, so a `curate` run today is scoped to a directory that is not
  there. The gate denies rather than resolving elsewhere, which is the safe end of that,
  and ticket 07 is what makes the scope mean something.
- MCP injection and the Cursor pre-flight. 6.5. The spawn directory exists now and holds
  the gate and the hook config; the `mcp.json` beside them and Cursor's `cursor-data/`
  are ticket 08 and ticket 09.
- Six of harness spec 6.1's seven pre-flight checks. Auth, depth, policy containment,
  concurrency, skill collision and both MCP checks all belong to tickets that are not
  built. What runs before a run id exists today is the environment, the provider version,
  the policy name and the preamble file.
- Delegation: `--parent`, depth, monotonic narrowing, the MCP delegate face. 7.3, 9.
- Detached spawning and the timeout. Section 9 spawns every child detached and makes
  blocking a view over it. Here the run is foreground and `hydra run` dies with it.
  Ctrl-C leaves an orphan child, a record with no `run_end`, an abandoned worktree and
  now an abandoned spawn directory, which is exactly ticket 14's abandoned-run class
  arriving early.
- The normalizer, the ten event types, and outcome derivation. Contract 7 and 8.
- Interactive runs and harvest. 5.6, 12.
- Every command but `run`. Notably `hydra reap`, so worktrees and spawn directories
  accumulate. Section 5 says the two are reaped together. Removing one by hand takes
  `git worktree remove ~/hydra/worktrees/<run-id>`, `git branch -D hydra/<run-id>` and
  `rm -rf ~/hydra/spawn/<run-id>`, and read harness spec 10 first, because the specced
  reap commits uncommitted work to the child's branch before removing anything. Ticket
  06 builds it.
- Ouroboros, both rungs.

## Rough edges worth knowing

- **A run with no `run_end` is not corrupt.** Contract section 6 says so. This slice
  produces one every time Hydra is killed mid-run, and nothing yet completes it.
- **A frame that is not valid UTF-8 is stored lossily** and the count lands in
  `run_end.error`. It has not happened; `claude` writes JSON through Node.
- **The tee has no line-length cap.** A provider that streams one enormous frame
  would grow the buffer without limit. Frames of 200 KB are tested and fine.
- **`~/hydra/spawn/` is not denied as a whole, only this run's own script.** 7.5 names
  "the generated gate script", singular, and `denied_write_paths` follows it literally.
  A `permissive` run could therefore rewrite a concurrent run's live script, which 7.5
  says is read fresh on every invocation. Nothing exercises this today, because runs are
  foreground and one at a time until ticket 05 detaches them. It is latent rather than
  live, and it is smaller than the shell hole above only because shell already reaches
  every one of these paths.
- **A gate that fails to start fails open on `claude`.** 6.6 measured it and nothing
  here changes it. The shebang is absolute and the script imports nothing outside the
  standard library, which is what that risk is spent on. Nothing checks at launch that
  the script runs, and adding such a check would be an eighth entry in 6.1's pre-flight
  list, which is a design decision rather than an implementation.
